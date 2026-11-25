//! A small Vim-style regular expression engine for `/` search, `:s` and `:g`.
//!
//! It reads Vim's magic syntax: `.` `*` `\+` `\=` `\?` `\{n,m}` `\{-n,m}` `^` `$` `[...]`
//! `\(...\)` `\%(...\)` `\|` `\<` `\>`, and the classes `\s \S \d \D \w \W \a \l \u \x \h`.
//! `\c` makes a pattern ignore case and `\C` makes it match case. Without either, a pattern
//! with an uppercase letter matches case and others ignore it (smartcase). A match never
//! crosses a line end: callers match one line at a time.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cls {
    Space,
    NonSpace,
    Digit,
    NonDigit,
    Word,
    NonWord,
    Alpha,
    Lower,
    Upper,
    Hex,
    Head,
}

#[derive(Clone, Debug)]
enum SetItem {
    Ch(char),
    Range(char, char),
    Cls(Cls),
}

#[derive(Clone, Debug)]
enum Node {
    Char(char),
    Any,
    Set {
        items: Vec<SetItem>,
        neg: bool,
    },
    Class(Cls),
    Bol,
    Eol,
    WordStart,
    WordEnd,
    /// A group: its capture slot (`None` for `\%(`) and its alternatives.
    Group(Option<usize>, Vec<Vec<Node>>),
    Repeat {
        node: Box<Node>,
        min: usize,
        max: usize,
        greedy: bool,
    },
}

/// A compiled pattern.
#[derive(Clone, Debug)]
pub struct Regex {
    alts: Vec<Vec<Node>>,
    icase: bool,
    groups: usize,
    /// The chars a match can start with, when every alternative starts with a plain char.
    /// The search then tries the full pattern only where the line has one of them.
    first: Option<Vec<char>>,
}

/// Where a match is: char offsets in the line, and each group's span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub groups: Vec<Option<(usize, usize)>>,
}

struct Parser {
    s: Vec<char>,
    i: usize,
    groups: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn peek2(&self) -> Option<char> {
        self.s.get(self.i + 1).copied()
    }

    /// Alternatives until the end of the pattern or a `\)`.
    fn alts(&mut self) -> Result<Vec<Vec<Node>>, String> {
        let mut alts = Vec::new();
        let mut seq: Vec<Node> = Vec::new();
        loop {
            match (self.peek(), self.peek2()) {
                (None, _) => break,
                (Some('\\'), Some(')')) => break,
                (Some('\\'), Some('|')) => {
                    self.i += 2;
                    alts.push(std::mem::take(&mut seq));
                }
                _ => {
                    let at_start = seq.is_empty();
                    let atom = self.atom(at_start)?;
                    let atom = self.quant(atom)?;
                    seq.push(atom);
                }
            }
        }
        alts.push(seq);
        Ok(alts)
    }

    /// Whether the parser is at the end of a branch: the end, `\)` or `\|`.
    fn at_branch_end(&self) -> bool {
        matches!((self.peek(), self.peek2()), (None, _) | (Some('\\'), Some(')' | '|')))
    }

    fn atom(&mut self, at_start: bool) -> Result<Node, String> {
        let c = self.peek().ok_or("E486: empty atom")?;
        self.i += 1;
        Ok(match c {
            '.' => Node::Any,
            '^' if at_start => Node::Bol,
            '$' if self.at_branch_end() => Node::Eol,
            '*' if at_start => Node::Char('*'),
            '[' => self.set()?,
            '\\' => {
                let Some(e) = self.peek() else { return Ok(Node::Char('\\')) };
                self.i += 1;
                match e {
                    '(' => {
                        let slot = self.groups;
                        self.groups += 1;
                        let alts = self.alts()?;
                        self.close_group()?;
                        Node::Group(Some(slot), alts)
                    }
                    '%' if self.peek() == Some('(') => {
                        self.i += 1;
                        let alts = self.alts()?;
                        self.close_group()?;
                        Node::Group(None, alts)
                    }
                    '<' => Node::WordStart,
                    '>' => Node::WordEnd,
                    't' => Node::Char('\t'),
                    'e' => Node::Char('\x1b'),
                    'n' => Node::Char('\n'),
                    c => match class_of(c) {
                        Some(cls) => Node::Class(cls),
                        None => Node::Char(c),
                    },
                }
            }
            c => Node::Char(c),
        })
    }

    fn close_group(&mut self) -> Result<(), String> {
        if self.peek() == Some('\\') && self.peek2() == Some(')') {
            self.i += 2;
            Ok(())
        } else {
            Err("E54: Unmatched \\(".into())
        }
    }

    /// A `[...]` set. Without a closing `]` the `[` is a plain char.
    fn set(&mut self) -> Result<Node, String> {
        let start = self.i;
        let mut neg = false;
        if self.peek() == Some('^') {
            neg = true;
            self.i += 1;
        }
        let mut items = Vec::new();
        let mut first = true;
        loop {
            let Some(c) = self.peek() else {
                self.i = start;
                return Ok(Node::Char('['));
            };
            if c == ']' && !first {
                self.i += 1;
                break;
            }
            first = false;
            self.i += 1;
            let c = if c == '\\' {
                let Some(e) = self.peek() else { break };
                self.i += 1;
                match e {
                    't' => '\t',
                    'e' => '\x1b',
                    'n' => '\n',
                    '\\' | ']' | '^' | '-' => e,
                    _ => {
                        if let Some(cls) = class_of(e) {
                            items.push(SetItem::Cls(cls));
                            continue;
                        }
                        e
                    }
                }
            } else {
                c
            };
            if self.peek() == Some('-') && self.peek2().is_some_and(|n| n != ']') {
                self.i += 1;
                let mut hi = self.peek().unwrap_or(c);
                self.i += 1;
                if hi == '\\' {
                    hi = self.peek().unwrap_or('\\');
                    self.i += 1;
                }
                items.push(SetItem::Range(c, hi));
            } else {
                items.push(SetItem::Ch(c));
            }
        }
        Ok(Node::Set { items, neg })
    }

    fn quant(&mut self, mut atom: Node) -> Result<Node, String> {
        loop {
            let (min, max, greedy) = match (self.peek(), self.peek2()) {
                (Some('*'), _) => {
                    self.i += 1;
                    (0, usize::MAX, true)
                }
                (Some('\\'), Some('+')) => {
                    self.i += 2;
                    (1, usize::MAX, true)
                }
                (Some('\\'), Some('=')) | (Some('\\'), Some('?')) => {
                    self.i += 2;
                    (0, 1, true)
                }
                (Some('\\'), Some('{')) => {
                    self.i += 2;
                    self.braces()?
                }
                _ => return Ok(atom),
            };
            atom = Node::Repeat { node: Box::new(atom), min, max, greedy };
        }
    }

    /// The inside of `\{n,m}`, after the `\{`.
    fn braces(&mut self) -> Result<(usize, usize, bool), String> {
        let mut greedy = true;
        if self.peek() == Some('-') {
            greedy = false;
            self.i += 1;
        }
        let num = |p: &mut Parser| -> Option<usize> {
            let start = p.i;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.i += 1;
            }
            (p.i > start).then(|| p.s[start..p.i].iter().collect::<String>().parse().unwrap_or(0))
        };
        let lo = num(self);
        let (min, max) = if self.peek() == Some(',') {
            self.i += 1;
            let hi = num(self);
            (lo.unwrap_or(0), hi.unwrap_or(usize::MAX))
        } else {
            let n = lo.unwrap_or(usize::MAX);
            (if n == usize::MAX { 0 } else { n }, n)
        };
        if self.peek() == Some('\\') {
            self.i += 1;
        }
        if self.peek() != Some('}') {
            return Err("E554: Syntax error in \\{...}".into());
        }
        self.i += 1;
        Ok((min, max, greedy))
    }
}

fn class_of(c: char) -> Option<Cls> {
    Some(match c {
        's' => Cls::Space,
        'S' => Cls::NonSpace,
        'd' => Cls::Digit,
        'D' => Cls::NonDigit,
        'w' => Cls::Word,
        'W' => Cls::NonWord,
        'a' => Cls::Alpha,
        'l' => Cls::Lower,
        'u' => Cls::Upper,
        'x' => Cls::Hex,
        'h' => Cls::Head,
        _ => return None,
    })
}

fn class_has(cls: Cls, c: char) -> bool {
    match cls {
        Cls::Space => c == ' ' || c == '\t',
        Cls::NonSpace => !(c == ' ' || c == '\t'),
        Cls::Digit => c.is_ascii_digit(),
        Cls::NonDigit => !c.is_ascii_digit(),
        Cls::Word => c.is_ascii_alphanumeric() || c == '_',
        Cls::NonWord => !(c.is_ascii_alphanumeric() || c == '_'),
        Cls::Alpha => c.is_ascii_alphabetic(),
        Cls::Lower => c.is_ascii_lowercase(),
        Cls::Upper => c.is_ascii_uppercase(),
        Cls::Hex => c.is_ascii_hexdigit(),
        Cls::Head => c.is_ascii_alphabetic() || c == '_',
    }
}

/// Keyword chars for `\<` and `\>`: letters, digits and `_`, in any script.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a pattern has an uppercase letter outside an escape, for smartcase.
fn has_upper(p: &str) -> bool {
    let mut esc = false;
    for c in p.chars() {
        if esc {
            esc = false;
            continue;
        }
        if c == '\\' {
            esc = true;
        } else if c.is_uppercase() {
            return true;
        }
    }
    false
}

/// The first char of every alternative, when each starts with a plain char.
fn first_chars(alts: &[Vec<Node>]) -> Option<Vec<char>> {
    let mut out: Vec<char> = Vec::new();
    for alt in alts {
        let Some(Node::Char(c)) = alt.first() else { return None };
        if !out.contains(c) {
            out.push(*c);
        }
    }
    Some(out)
}

impl Regex {
    /// Compiles `pattern` with smartcase, `\c` and `\C`. Searches use `with_case` and the
    /// editor's `ignorecase` and `smartcase`.
    #[cfg(test)]
    pub fn new(pattern: &str) -> Result<Regex, String> {
        Regex::with_case(pattern, None)
    }

    /// Compiles `pattern`. `icase` overrides smartcase (`:s` flags `i` and `I`).
    pub fn with_case(pattern: &str, icase: Option<bool>) -> Result<Regex, String> {
        let mut forced = None;
        let mut clean = String::new();
        let mut it = pattern.chars().peekable();
        while let Some(c) = it.next() {
            if c == '\\' {
                match it.peek() {
                    Some('c') => {
                        it.next();
                        forced = Some(true);
                        continue;
                    }
                    Some('C') => {
                        it.next();
                        forced = Some(false);
                        continue;
                    }
                    Some(&n) => {
                        it.next();
                        clean.push('\\');
                        clean.push(n);
                        continue;
                    }
                    None => {}
                }
            }
            clean.push(c);
        }
        let icase = forced.or(icase).unwrap_or_else(|| !has_upper(&clean));
        let mut p = Parser { s: clean.chars().collect(), i: 0, groups: 0 };
        let alts = p.alts()?;
        if p.i < p.s.len() {
            return Err("E16: Unmatched \\)".into());
        }
        let first = first_chars(&alts);
        Ok(Regex { alts, icase, groups: p.groups, first })
    }

    /// The first match in `text` that starts at or after `from`.
    pub fn find_at(&self, text: &[char], from: usize) -> Option<Match> {
        let anchored = self.alts.iter().all(|a| matches!(a.first(), Some(Node::Bol)));
        let mut m = Matcher { text, icase: self.icase, caps: vec![None; self.groups] };
        for start in from..=text.len() {
            if anchored && start > 0 {
                break;
            }
            if let Some(first) = &self.first
                && !text.get(start).is_some_and(|&c| first.iter().any(|&f| m.eq(c, f)))
            {
                continue;
            }
            m.caps.iter_mut().for_each(|c| *c = None);
            let mut end = None;
            if m.alts(&self.alts, start, &mut |_, j| {
                end = Some(j);
                true
            }) {
                return Some(Match { start, end: end.unwrap_or(start), groups: m.caps.clone() });
            }
        }
        None
    }

    /// Every match in `text`, left to right, without overlaps. An empty match moves on by one.
    pub fn find_all(&self, text: &[char]) -> Vec<Match> {
        let mut out = Vec::new();
        let mut from = 0;
        while from <= text.len() {
            let Some(m) = self.find_at(text, from) else { break };
            from = if m.end > m.start { m.end } else { m.start + 1 };
            out.push(m);
        }
        out
    }

    /// Whether `text` holds a match anywhere.
    pub fn is_match(&self, text: &[char]) -> bool {
        self.find_at(text, 0).is_some()
    }
}

type Cont<'k, 'a> = dyn FnMut(&mut Matcher<'a>, usize) -> bool + 'k;

struct Matcher<'a> {
    text: &'a [char],
    icase: bool,
    caps: Vec<Option<(usize, usize)>>,
}

impl<'a> Matcher<'a> {
    fn eq(&self, a: char, b: char) -> bool {
        if a == b {
            return true;
        }
        if !self.icase {
            return false;
        }
        // ASCII folds without the Unicode tables. The Kelvin sign folds to `k`, so a pair with
        // a char outside ASCII takes the full compare.
        if a.is_ascii() && b.is_ascii() { a.eq_ignore_ascii_case(&b) } else { a.to_lowercase().eq(b.to_lowercase()) }
    }

    fn alts(&mut self, alts: &[Vec<Node>], i: usize, k: &mut Cont<'_, 'a>) -> bool {
        for alt in alts {
            if self.seq(alt, i, k) {
                return true;
            }
        }
        false
    }

    fn seq(&mut self, seq: &[Node], i: usize, k: &mut Cont<'_, 'a>) -> bool {
        match seq.split_first() {
            None => k(self, i),
            Some((node, rest)) => self.node(node, i, &mut |m, j| m.seq(rest, j, k)),
        }
    }

    /// Whether the single-char node `node` matches the char at `i`.
    fn one(&self, node: &Node, i: usize) -> Option<bool> {
        let c = self.text.get(i).copied();
        Some(match node {
            Node::Char(x) => c.is_some_and(|c| self.eq(c, *x)),
            Node::Any => c.is_some(),
            Node::Class(cls) => c.is_some_and(|c| class_has(*cls, c)),
            Node::Set { items, neg } => c.is_some_and(|c| self.in_set(items, c) != *neg),
            _ => return None,
        })
    }

    fn in_set(&self, items: &[SetItem], c: char) -> bool {
        let lc = c.to_lowercase().next().unwrap_or(c);
        let uc = c.to_uppercase().next().unwrap_or(c);
        let test = |c: char| {
            items.iter().any(|it| match *it {
                SetItem::Ch(x) => x == c,
                SetItem::Range(a, b) => a <= c && c <= b,
                SetItem::Cls(cls) => class_has(cls, c),
            })
        };
        test(c) || (self.icase && (test(lc) || test(uc)))
    }

    fn node(&mut self, node: &Node, i: usize, k: &mut Cont<'_, 'a>) -> bool {
        if let Some(hit) = self.one(node, i) {
            return hit && k(self, i + 1);
        }
        let t = self.text;
        match node {
            Node::Bol => i == 0 && k(self, i),
            Node::Eol => i == t.len() && k(self, i),
            Node::WordStart => i < t.len() && is_word(t[i]) && (i == 0 || !is_word(t[i - 1])) && k(self, i),
            Node::WordEnd => i > 0 && is_word(t[i - 1]) && (i == t.len() || !is_word(t[i])) && k(self, i),
            Node::Group(slot, alts) => {
                let slot = *slot;
                self.alts(alts, i, &mut |m, j| {
                    let Some(s) = slot else { return k(m, j) };
                    let old = m.caps[s];
                    m.caps[s] = Some((i, j));
                    if k(m, j) {
                        true
                    } else {
                        m.caps[s] = old;
                        false
                    }
                })
            }
            Node::Repeat { node, min, max, greedy } => {
                if self.one(node, i).is_some() {
                    // A single-char node: count the run, then try lengths without recursion.
                    let mut n = 0;
                    while n < *max && self.one(node, i + n) == Some(true) {
                        n += 1;
                    }
                    if n < *min {
                        return false;
                    }
                    if *greedy { (*min..=n).rev().any(|len| k(self, i + len)) } else { (*min..=n).any(|len| k(self, i + len)) }
                } else {
                    self.rep(node, *min, *max, *greedy, 0, i, k)
                }
            }
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rep(&mut self, node: &Node, min: usize, max: usize, greedy: bool, n: usize, i: usize, k: &mut Cont<'_, 'a>) -> bool {
        if greedy {
            if n < max && self.node(node, i, &mut |m, j| j != i && m.rep(node, min, max, greedy, n + 1, j, k)) {
                return true;
            }
            n >= min && k(self, i)
        } else {
            if n >= min && k(self, i) {
                return true;
            }
            n < max && self.node(node, i, &mut |m, j| j != i && m.rep(node, min, max, greedy, n + 1, j, k))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(p: &str, t: &str) -> Option<(usize, usize)> {
        let chars: Vec<char> = t.chars().collect();
        Regex::new(p).unwrap().find_at(&chars, 0).map(|m| (m.start, m.end))
    }

    #[test]
    fn the_first_char_skip_keeps_every_match() {
        // Each alternative starts with a plain char, so only those starts are tried.
        assert_eq!(find("cat\\|dog", "a hot dog"), Some((6, 9)));
        assert_eq!(find("DOG", "a hot dog DOG"), Some((10, 13)));
        // The Kelvin sign folds to `k`, outside the ASCII compare.
        assert_eq!(find("\\ck", "\u{212A}ilo"), Some((0, 1)));
        // A pattern that can start with any char tries every start.
        assert_eq!(find("x*y", "aaay"), Some((3, 4)));
        assert_eq!(Regex::new("a\\|b*").unwrap().first, None);
        assert_eq!(Regex::new("ab\\|ac").unwrap().first, Some(vec!['a']));
    }

    #[test]
    fn plain_text_and_smartcase() {
        assert_eq!(find("bar", "foo bar"), Some((4, 7)));
        assert_eq!(find("bar", "foo BAR"), Some((4, 7)));
        assert_eq!(find("Bar", "foo bar"), None);
        assert_eq!(find("\\cBar", "foo bar"), Some((4, 7)));
    }

    #[test]
    fn quantifiers_classes_and_anchors() {
        assert_eq!(find("a*b", "xaaab"), Some((1, 5)));
        assert_eq!(find("\\d\\+", "ab 123 c"), Some((3, 6)));
        assert_eq!(find("^foo", "foo foo"), Some((0, 3)));
        assert_eq!(find("foo$", "foo foo"), Some((4, 7)));
        assert_eq!(find("[a-c]\\{2}", "xxbcx"), Some((2, 4)));
        assert_eq!(find("\\<is\\>", "this is"), Some((5, 7)));
        assert_eq!(find("a.c", "abc"), Some((0, 3)));
        assert_eq!(find("colou\\=r", "color"), Some((0, 5)));
    }

    #[test]
    fn groups_and_alternation() {
        let chars: Vec<char> = "key = value".chars().collect();
        let m = Regex::new("\\(\\w\\+\\) = \\(\\w\\+\\)").unwrap().find_at(&chars, 0).unwrap();
        assert_eq!(m.groups, vec![Some((0, 3)), Some((6, 11))]);
        assert_eq!(find("cat\\|dog", "hotdog"), Some((3, 6)));
    }

    #[test]
    fn find_all_moves_past_empty_matches() {
        let chars: Vec<char> = "abc".chars().collect();
        assert_eq!(Regex::new("x*").unwrap().find_all(&chars).len(), 4);
    }
}
