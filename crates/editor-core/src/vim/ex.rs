//! Ex commands: what runs after `:`, with ranges such as `%`, `'<,'>` and `.,+3`.

use std::path::PathBuf;

use crate::buffer::Pos;
use crate::editor::{Action, Editor, KeyCode, KeyInput, Mode};

use super::ops::{self, Op, Range};
use super::options::set;
use super::regex::{Match, Regex};
use super::{begin, commit_here, dispatch, get_mark, push_jump, set_cur, store};

type Res = Result<(), String>;

/// Runs an Ex command line (without its `:`) and returns what the app must do.
pub fn run_ex(ed: &mut Editor, line: &str) -> Vec<Action> {
    let mut out = Vec::new();
    let started = ed.has_doc() && !ed.buf().in_txn();
    if started {
        begin(ed);
    }
    let res = run(ed, line, &mut out);
    if started && ed.mode != Mode::Insert {
        commit_here(ed);
    }
    if let Err(e) = res {
        ed.error(e.clone());
        out.push(Action::Error(e));
    }
    super::settle(ed);
    out
}

fn need_doc(ed: &Editor) -> Res {
    if ed.has_doc() { Ok(()) } else { Err("E32: No file name".into()) }
}

/// Splits `name[!] args`. Names are letters, or a run of one of `&<>=!#`.
fn split_cmd(s: &str) -> (&str, bool, &str) {
    let n = s.chars().take_while(|c| c.is_ascii_alphabetic()).count();
    let n = if n > 0 {
        n
    } else {
        match s.chars().next() {
            Some(c @ ('<' | '>')) => s.chars().take_while(|x| *x == c).count(),
            Some('&') => s.chars().take_while(|x| *x == '&').count(),
            Some('=' | '#' | '!') => 1,
            _ => 0,
        }
    };
    let (name, rest) = s.split_at(n);
    let (bang, rest) = match rest.strip_prefix('!') {
        Some(r) if !name.is_empty() => (true, r),
        _ => (false, rest),
    };
    let rest = if matches!(name, "s" | "substitute" | "g" | "global" | "v" | "vglobal") { rest } else { rest.trim_start() };
    (name, bang, rest)
}

fn run(ed: &mut Editor, line: &str, out: &mut Vec<Action>) -> Res {
    let s = line.trim_start_matches([' ', ':', '\t']);
    if s.is_empty() {
        return Ok(());
    }
    let (range, rest) = if ed.has_doc() { parse_range(ed, s)? } else { (None, s) };
    let (name, bang, args) = split_cmd(rest.trim_start());
    let cur_line = if ed.has_doc() { ed.cursor().line } else { 0 };
    let r = range.unwrap_or((cur_line, cur_line));
    let whole = (0, if ed.has_doc() { ed.buf().last_line() } else { 0 });
    match name {
        // `:!cmd` runs a command; `:{range}!cmd` filters the range's lines through it.
        "!" => {
            let cmd = args.trim();
            if cmd.is_empty() {
                return Err("E471: Argument required".into());
            }
            match range {
                Some((a, b)) => {
                    need_doc(ed)?;
                    out.push(Action::Filter { first: a.min(b), last: a.max(b), cmd: cmd.to_string() });
                }
                None => out.push(Action::Shell(cmd.to_string())),
            }
        }
        "" => {
            need_doc(ed)?;
            if let Some((_, b)) = range {
                let cur = ed.cursor();
                push_jump(ed, cur);
                let col = ed.buf().first_non_blank(b);
                set_cur(ed, Pos::new(b, col));
            }
        }
        "w" | "write" | "sav" | "saveas" => out.push(Action::Save(path(ed, args))),
        "wa" | "wall" => out.push(Action::SaveAll),
        "wq" | "x" | "xit" | "exi" | "exit" => {
            if name == "wq" || (ed.has_doc() && ed.buf().is_dirty()) {
                out.push(Action::Save(path(ed, args)));
            }
            out.push(Action::Quit { force: bang });
        }
        "q" | "quit" | "clo" | "close" => out.push(Action::Quit { force: bang }),
        "qa" | "qall" | "quita" | "quitall" => out.push(Action::QuitAll { force: bang }),
        "wqa" | "wqall" | "xa" | "xall" => {
            out.push(Action::SaveAll);
            out.push(Action::QuitAll { force: false });
        }
        "e" | "edit" => match path(ed, args) {
            Some(p) => out.push(Action::Open(p)),
            None if !bang && ed.has_doc() && ed.buf().is_dirty() => {
                return Err("E37: No write since last change (add ! to override)".into());
            }
            None => out.push(Action::Reload),
        },
        "bn" | "bnext" => out.push(Action::NextBuffer),
        "bp" | "bprevious" | "bprev" | "bN" | "bNext" => out.push(Action::PrevBuffer),
        "bd" | "bdelete" | "bw" | "bwipeout" => out.push(Action::CloseBuffer),
        "b" | "buffer" if !args.trim().is_empty() => out.push(Action::Open(buffer_arg(ed, args.trim())?)),
        "ls" | "buffers" | "files" | "b" | "buffer" => out.push(Action::PickBuffer),
        "Settings" | "settings" => out.push(Action::Settings),
        "Keys" | "keys" | "h" | "help" => out.push(Action::KeyList(args.trim().to_string())),
        "Find" | "find" | "Files" => out.push(Action::FindFile),
        "Grep" | "grep" | "rg" => out.push(Action::GrepProject),
        "Explore" | "Ex" | "Tree" => out.push(Action::ToggleTree),
        "Rename" | "rename" => {
            let name = args.trim();
            if name.is_empty() {
                return Err("E471: Argument required".into());
            }
            out.push(Action::Rename(name.to_string()));
        }
        "CodeAction" | "codeaction" | "ca" => out.push(Action::CodeActions),
        "marks" => {
            need_doc(ed)?;
            ed.message(list_marks(ed));
        }
        "delm" | "delmarks" => {
            need_doc(ed)?;
            del_marks(ed, args, bang)?;
        }
        "noh" | "nohlsearch" => ed.vim.hlsearch = false,
        "se" | "set" | "setl" | "setlocal" => set(ed, args, out)?,
        "u" | "undo" | "red" | "redo" => {
            need_doc(ed)?;
            let buf = &mut ed.doc_mut().buf;
            let at = if name.starts_with('u') { buf.undo() } else { buf.redo() };
            match at {
                Some(p) => set_cur(ed, p),
                None => ed.message(if name.starts_with('u') { "Already at oldest change" } else { "Already at newest change" }),
            }
        }
        "s" | "substitute" | "&" | "&&" => {
            need_doc(ed)?;
            let args = match name {
                "&&" => "&",
                "&" => "",
                _ => args,
            };
            substitute(ed, r, args)?;
        }
        "g" | "global" | "v" | "vglobal" => {
            need_doc(ed)?;
            let r = range.unwrap_or(whole);
            global(ed, r, args, bang || name.starts_with('v'), out)?;
        }
        "d" | "de" | "del" | "delete" => {
            need_doc(ed)?;
            let (reg, count) = reg_count(args);
            let (a, b) = with_count(ed, r, count);
            ops::apply(ed, Op::Delete, Range::Lines(a, b), reg, 1, out);
        }
        "y" | "ya" | "yank" => {
            need_doc(ed)?;
            let (reg, count) = reg_count(args);
            let (a, b) = with_count(ed, r, count);
            let text = ops::lines_text(ed.buf(), a, b);
            store(ed, reg, text, true, true, out);
            if b - a + 1 > 2 {
                ed.message(format!("{} lines yanked", b - a + 1));
            }
        }
        "pu" | "put" => {
            need_doc(ed)?;
            let reg = args.chars().next();
            let Some(text) = ed.reg(reg).map(|r| r.text) else { return Err("E353: Nothing in register".into()) };
            let buf = &mut ed.doc_mut().buf;
            let first = ops::put_lines(buf, r.1, &text, bang);
            let col = buf.first_non_blank(first);
            set_cur(ed, Pos::new(first, col));
        }
        "m" | "mo" | "move" => {
            need_doc(ed)?;
            let dest = dest(ed, args)?;
            let (a, b) = r;
            if !ops::move_range(ed, a, b, dest) {
                return Err("E134: Cannot move a range of lines into itself".into());
            }
            let line = if dest > b { dest - 1 } else { dest + (b - a) };
            let col = ed.buf().first_non_blank(line);
            set_cur(ed, Pos::new(line, col));
        }
        "t" | "co" | "copy" => {
            need_doc(ed)?;
            let dest = dest(ed, args)?;
            let (a, b) = r;
            let text = ops::lines_text(ed.buf(), a, b);
            let buf = &mut ed.doc_mut().buf;
            let first = if dest == 0 { ops::put_lines(buf, 0, &text, true) } else { ops::put_lines(buf, dest - 1, &text, false) };
            let line = first + (b - a);
            let col = buf.first_non_blank(line);
            set_cur(ed, Pos::new(line, col));
        }
        "j" | "join" => {
            need_doc(ed)?;
            let (a, b) = if r.0 == r.1 { (r.0, r.0 + 1) } else { r };
            let buf = &mut ed.doc_mut().buf;
            if let Some(p) = ops::join_lines(buf, a, b - a + 1, !bang) {
                set_cur(ed, p);
            }
        }
        n if n.starts_with('>') || n.starts_with('<') => {
            need_doc(ed)?;
            let (a, b) = r;
            ops::shift_lines(ed, a, b, n.starts_with('>'), n.len());
            let col = ed.buf().first_non_blank(b);
            set_cur(ed, Pos::new(b, col));
        }
        "sor" | "sort" => {
            need_doc(ed)?;
            let (a, b) = range.unwrap_or(whole);
            sort(ed, a, b, bang, args);
        }
        "norm" | "normal" => {
            need_doc(ed)?;
            let lines: Vec<usize> = match range {
                Some((a, b)) => (a..=b).collect(),
                None => vec![cur_line],
            };
            for l in lines.into_iter().rev() {
                if l > ed.buf().last_line() {
                    continue;
                }
                set_cur(ed, Pos::new(l, 0));
                normal(ed, args, out);
            }
        }
        _ => return Err(format!("E492: Not an editor command: {}", s.trim())),
    }
    Ok(())
}

/// A file argument made absolute against the project root, with `~` expanded.
fn path(ed: &Editor, args: &str) -> Option<PathBuf> {
    let a = args.trim();
    if a.is_empty() {
        return None;
    }
    let p = match a.strip_prefix("~/") {
        Some(rest) => std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(rest),
        None => PathBuf::from(a),
    };
    Some(if p.is_absolute() { p } else { ed.root.join(p) })
}

/// An optional register name and count after `:d` or `:y`.
fn reg_count(args: &str) -> (Option<char>, Option<usize>) {
    let a = args.trim();
    let mut chars = a.chars();
    let reg = match chars.clone().next() {
        Some(c) if !c.is_ascii_digit() => {
            chars.next();
            Some(c)
        }
        _ => None,
    };
    let count = chars.as_str().trim().parse().ok();
    (reg, count)
}

fn with_count(ed: &Editor, r: (usize, usize), count: Option<usize>) -> (usize, usize) {
    let last = ed.buf().last_line();
    match count {
        Some(n) if n > 0 => (r.1, (r.1 + n - 1).min(last)),
        _ => (r.0, r.1.min(last)),
    }
}

/// The destination of `:m` and `:t`: the line to go below, 1-based, 0 for the top.
fn dest(ed: &mut Editor, args: &str) -> Result<usize, String> {
    let (addr, _) = parse_addr(ed, args.trim())?;
    let a = addr.ok_or("E14: Invalid address")?;
    Ok((a + 1).max(0) as usize)
}

/// A line range at the start of a command, 0-based, and the rest of the text.
type RangeParse<'a> = Result<(Option<(usize, usize)>, &'a str), String>;

fn parse_range<'a>(ed: &mut Editor, s: &'a str) -> RangeParse<'a> {
    let last = ed.buf().last_line();
    if let Some(r) = s.strip_prefix('%') {
        return Ok((Some((0, last)), r));
    }
    let (a, rest) = parse_addr(ed, s)?;
    let Some(a) = a else { return Ok((None, rest)) };
    let clamp = |x: isize| x.clamp(0, last as isize) as usize;
    if let Some(sep) = rest.chars().next().filter(|c| matches!(c, ',' | ';')) {
        // `;` moves the cursor to the first address before the second is read, as in Vim.
        if sep == ';' {
            ed.set_cursor(crate::buffer::Pos::new(clamp(a), 0));
        }
        let (b, r2) = parse_addr(ed, &rest[1..])?;
        let b = b.unwrap_or(ed.cursor().line as isize);
        let (x, y) = (clamp(a.min(b)), clamp(a.max(b)));
        return Ok((Some((x, y)), r2));
    }
    Ok((Some((clamp(a), clamp(a))), rest))
}

/// One address: `.`, `$`, a number, `'x`, `/pat/`, `?pat?`, with `+N` and `-N` offsets.
/// Line `0` is -1.
fn parse_addr<'a>(ed: &mut Editor, s: &'a str) -> Result<(Option<isize>, &'a str), String> {
    let cur = ed.cursor().line as isize;
    let last = ed.buf().last_line() as isize;
    let mut rest = s;
    let mut base: Option<isize> = None;
    let first = rest.chars().next();
    match first {
        Some('.') => {
            base = Some(cur);
            rest = &rest[1..];
        }
        Some('$') => {
            base = Some(last);
            rest = &rest[1..];
        }
        Some(c) if c.is_ascii_digit() => {
            let n = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            let v: isize = rest[..n].parse().map_err(|_| "E16: Invalid range")?;
            base = Some(v - 1);
            rest = &rest[n..];
        }
        Some('\'') => {
            let m = rest[1..].chars().next().ok_or("E20: Mark not set")?;
            let p = get_mark(ed, m).ok_or("E20: Mark not set")?;
            base = Some(p.line as isize);
            rest = &rest[1 + m.len_utf8()..];
        }
        Some(d @ ('/' | '?')) => {
            let body = &rest[1..];
            let end = body.find(d).unwrap_or(body.len());
            let pat = &body[..end];
            rest = if end < body.len() { &body[end + 1..] } else { "" };
            let re = Regex::with_case(pat, ed.case())?;
            let buf = ed.buf();
            let n = last + 1;
            let mut chars = Vec::new();
            let hit = (1..=n).map(|k| if d == '/' { (cur + k).rem_euclid(n) } else { (cur - k).rem_euclid(n) }).find(|&l| {
                buf.line_chars(l as usize, &mut chars);
                re.is_match(&chars)
            });
            base = Some(hit.ok_or_else(|| format!("E486: Pattern not found: {pat}"))?);
        }
        _ => {}
    }
    loop {
        let sign = match rest.chars().next() {
            Some('+') => 1,
            Some('-') => -1,
            _ => break,
        };
        rest = &rest[1..];
        let n = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        let v: isize = if n == 0 { 1 } else { rest[..n].parse().unwrap_or(1) };
        rest = &rest[n..];
        base = Some(base.unwrap_or(cur) + sign * v);
    }
    match base {
        Some(b) if b > last => return Ok((Some(last), rest)),
        Some(b) if b < -1 => return Err("E16: Invalid range".into()),
        _ => {}
    }
    Ok((base, rest))
}

/// Splits at the next `delim` not escaped by `\`. `\delim` becomes `delim`. Other escapes stay.
fn split_at_delim(s: &str, delim: char) -> (String, Option<&str>) {
    let mut out = String::new();
    let mut it = s.char_indices();
    while let Some((i, c)) = it.next() {
        if c == '\\' {
            match it.next() {
                Some((_, n)) if n == delim => out.push(n),
                Some((_, n)) => {
                    out.push('\\');
                    out.push(n);
                }
                None => out.push('\\'),
            }
        } else if c == delim {
            return (out, Some(&s[i + c.len_utf8()..]));
        } else {
            out.push(c);
        }
    }
    (out, None)
}

fn substitute(ed: &mut Editor, r: (usize, usize), args: &str) -> Res {
    let (pat, rep, flags) = if args.trim().is_empty() || args.starts_with([' ', '&']) {
        let (p, rp, f) = ed.vim.last_sub.clone().ok_or("E35: No previous regular expression")?;
        let keep = args.trim().starts_with('&');
        (p, rp, if keep { f } else { args.trim().trim_start_matches('&').to_string() })
    } else {
        let delim = args.chars().next().unwrap_or('/');
        if delim.is_alphanumeric() || delim == '\\' || delim == '"' || delim == '|' {
            return Err("E146: Regular expressions can't be delimited by letters".into());
        }
        let (pat, rest) = split_at_delim(&args[delim.len_utf8()..], delim);
        let (rep, rest) = match rest {
            Some(r) => {
                let (rep, rest) = split_at_delim(r, delim);
                (rep, rest.unwrap_or(""))
            }
            None => (String::new(), ""),
        };
        let pat = if pat.is_empty() { ed.search.pattern.clone() } else { pat };
        (pat, rep, rest.to_string())
    };
    if pat.is_empty() {
        return Err("E35: No previous regular expression".into());
    }
    let fl: String = flags.chars().take_while(|c| !c.is_ascii_digit() && *c != ' ').collect();
    let count: Option<usize> = flags[fl.len()..].trim().parse().ok();
    let global = fl.contains('g');
    let count_only = fl.contains('n');
    let confirm = fl.contains('c') && !count_only;
    let quiet = fl.contains('e');
    let icase = if fl.contains('i') {
        Some(true)
    } else if fl.contains('I') {
        Some(false)
    } else {
        ed.case()
    };
    let re = Regex::with_case(&pat, icase)?;
    let (mut a, mut b) = r;
    if let Some(n) = count {
        a = b;
        b = (b + n.max(1) - 1).min(ed.buf().last_line());
    }
    ed.vim.last_sub = Some((pat.clone(), rep.clone(), fl.clone()));
    ed.search.pattern = pat.clone();
    ed.vim.hlsearch = true;
    if confirm {
        if super::confirm::start(ed, re, rep, global, a, b) {
            return Ok(());
        }
        return if quiet { Ok(()) } else { Err(format!("E486: Pattern not found: {pat}")) };
    }
    let mut subs = 0;
    let mut lines = 0;
    let mut last_line = None;
    let mut shift = 0usize;
    let mut text = Vec::new();
    for l in a..=b {
        let line_no = l + shift;
        ed.buf().line_chars(line_no, &mut text);
        let ms: Vec<Match> = if global { re.find_all(&text) } else { re.find_at(&text, 0).into_iter().collect() };
        if ms.is_empty() {
            continue;
        }
        subs += ms.len();
        lines += 1;
        if count_only {
            continue;
        }
        let mut new = String::new();
        let mut at = 0;
        for m in &ms {
            new.extend(&text[at..m.start]);
            new.push_str(&expand(&rep, &text, m));
            at = m.end;
        }
        new.extend(&text[at..]);
        let added = new.matches('\n').count();
        ops::replace_lines(&mut ed.doc_mut().buf, line_no, line_no, &[new]);
        shift += added;
        last_line = Some(line_no + added);
    }
    if subs == 0 {
        return if quiet { Ok(()) } else { Err(format!("E486: Pattern not found: {pat}")) };
    }
    if count_only {
        ed.message(format!("{subs} match{} on {lines} line{}", if subs == 1 { "" } else { "es" }, if lines == 1 { "" } else { "s" }));
        return Ok(());
    }
    if let Some(l) = last_line {
        let col = ed.buf().first_non_blank(l);
        set_cur(ed, Pos::new(l, col));
    }
    if subs > 1 {
        ed.message(format!("{subs} substitutions on {lines} line{}", if lines == 1 { "" } else { "s" }));
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Case {
    Upper,
    Lower,
}

/// The replacement text for match `m`: `&` and `\0` the whole match, `\1`-`\9` its groups,
/// `\r` and `\n` a line break, `\t` a tab, and `\u` `\l` `\U` `\L` `\e` for case.
pub(crate) fn expand(rep: &str, text: &[char], m: &Match) -> String {
    let mut out = String::new();
    let mut one: Option<Case> = None;
    let mut all: Option<Case> = None;
    let whole: String = text[m.start..m.end].iter().collect();
    let group = |i: usize| -> String {
        match m.groups.get(i).copied().flatten() {
            Some((a, b)) => text[a..b].iter().collect(),
            None => String::new(),
        }
    };
    let mut it = rep.chars();
    while let Some(c) = it.next() {
        match c {
            '&' => push_case(&mut out, &whole, &mut one, all),
            '\\' => match it.next() {
                Some('&') => push_case(&mut out, "&", &mut one, all),
                Some('0') => push_case(&mut out, &whole, &mut one, all),
                Some(d @ '1'..='9') => push_case(&mut out, &group(d as usize - '1' as usize), &mut one, all),
                Some('n' | 'r') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('u') => one = Some(Case::Upper),
                Some('l') => one = Some(Case::Lower),
                Some('U') => all = Some(Case::Upper),
                Some('L') => all = Some(Case::Lower),
                Some('e' | 'E') => all = None,
                Some(o) => push_case(&mut out, &o.to_string(), &mut one, all),
                None => out.push('\\'),
            },
            c => push_case(&mut out, &c.to_string(), &mut one, all),
        }
    }
    out
}

fn push_case(out: &mut String, s: &str, one: &mut Option<Case>, all: Option<Case>) {
    for c in s.chars() {
        match one.take().or(all) {
            Some(Case::Upper) => out.extend(c.to_uppercase()),
            Some(Case::Lower) => out.extend(c.to_lowercase()),
            None => out.push(c),
        }
    }
}

fn global(ed: &mut Editor, r: (usize, usize), args: &str, invert: bool, out: &mut Vec<Action>) -> Res {
    let delim = args.chars().next().ok_or("E476: Invalid command")?;
    let (pat, rest) = split_at_delim(&args[delim.len_utf8()..], delim);
    let cmd = rest.unwrap_or("").trim().to_string();
    let cmd = if cmd.is_empty() { "p".to_string() } else { cmd };
    let pat = if pat.is_empty() { ed.search.pattern.clone() } else { pat };
    if pat.is_empty() {
        return Err("E35: No previous regular expression".into());
    }
    let re = Regex::with_case(&pat, ed.case())?;
    let buf = ed.buf();
    let mut chars = Vec::new();
    let lines: Vec<usize> = (r.0..=r.1.min(buf.last_line()))
        .filter(|&l| {
            buf.line_chars(l, &mut chars);
            re.is_match(&chars) != invert
        })
        .collect();
    ed.search.pattern = pat.clone();
    ed.vim.hlsearch = true;
    if lines.is_empty() {
        return Err(if invert { format!("Pattern found in every line: {pat}") } else { format!("E486: Pattern not found: {pat}") });
    }
    if cmd == "p" || cmd == "print" {
        ed.message(format!("{} matching lines", lines.len()));
        return Ok(());
    }
    let word: String = cmd.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let top_down = matches!(word.as_str(), "m" | "mo" | "move" | "t" | "co" | "copy");
    let before = ed.buf().line_count();
    let order: Vec<usize> = if top_down { lines } else { lines.into_iter().rev().collect() };
    for l in order {
        if l > ed.buf().last_line() {
            continue;
        }
        set_cur(ed, Pos::new(l, 0));
        let _ = run(ed, &cmd, out);
    }
    let after = ed.buf().line_count();
    if before > after + 2 {
        ed.message(format!("{} fewer lines", before - after));
    } else if after > before + 2 {
        ed.message(format!("{} more lines", after - before));
    }
    Ok(())
}

fn sort(ed: &mut Editor, a: usize, b: usize, reverse: bool, args: &str) {
    let buf = &mut ed.doc_mut().buf;
    let mut lines: Vec<String> = (a..=b).map(|l| buf.line(l)).collect();
    let icase = args.contains('i');
    let numeric = args.contains('n');
    let num = |s: &str| -> i64 {
        let d: String =
            s.chars().skip_while(|c| !c.is_ascii_digit() && *c != '-').take_while(|c| c.is_ascii_digit() || *c == '-').collect();
        d.parse().unwrap_or(i64::MIN)
    };
    if numeric {
        lines.sort_by_key(|l| num(l));
    } else if icase {
        lines.sort_by_key(|l| l.to_lowercase());
    } else {
        lines.sort();
    }
    if reverse {
        lines.reverse();
    }
    if args.contains('u') {
        lines.dedup_by(|x, y| if icase { x.to_lowercase() == y.to_lowercase() } else { x == y });
    }
    ops::replace_lines(buf, a, b, &lines);
    set_cur(ed, Pos::new(a, 0));
}

/// `:marks`: every mark set in this buffer and the file marks, on one line.
fn list_marks(ed: &Editor) -> String {
    let mut parts: Vec<String> = ed
        .buf()
        .marks()
        .filter(|(c, _)| c.is_ascii_lowercase() || matches!(c, '\'' | '.' | '^' | '<' | '>' | '[' | ']' | '"'))
        .map(|(c, p)| format!("{c} {}:{}", p.line + 1, p.col))
        .collect();
    let mut files: Vec<(&char, &(Option<PathBuf>, Pos))> = ed.vim.file_marks.iter().collect();
    files.sort_by_key(|(c, _)| **c);
    for (c, (path, p)) in files {
        let name = path.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        parts.push(format!("{c} {}:{} {name}", p.line + 1, p.col));
    }
    if parts.is_empty() { "no marks set".into() } else { format!("mark line:col  {}", parts.join("  ")) }
}

/// `:delmarks a b c-e` and `:delmarks!` (every lowercase mark).
fn del_marks(ed: &mut Editor, args: &str, bang: bool) -> Res {
    let mut names: Vec<char> = Vec::new();
    if bang {
        names.extend('a'..='z');
    }
    let chars: Vec<char> = args.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            names.extend(chars[i]..=chars[i + 2]);
            i += 3;
        } else {
            names.push(chars[i]);
            i += 1;
        }
    }
    if names.is_empty() {
        return Err("E471: Argument required".into());
    }
    for c in names {
        ed.doc_mut().buf.clear_mark(c);
        if c.is_ascii_uppercase() {
            ed.vim.file_marks.remove(&c);
        }
    }
    Ok(())
}

/// `:b 2` and `:b name`: the path of buffer 2, or of the one buffer whose name holds `name`.
fn buffer_arg(ed: &Editor, arg: &str) -> Result<PathBuf, String> {
    let path_of = |i: usize| ed.docs[i].buf.path.clone().ok_or_else(|| format!("E86: Buffer {} has no file name", i + 1));
    if let Ok(n) = arg.parse::<usize>() {
        return match n.checked_sub(1).filter(|&i| i < ed.docs.len()) {
            Some(i) => path_of(i),
            None => Err(format!("E86: Buffer {n} does not exist")),
        };
    }
    let hits: Vec<usize> =
        (0..ed.docs.len()).filter(|&i| ed.docs[i].buf.path.as_deref().is_some_and(|p| p.to_string_lossy().contains(arg))).collect();
    match hits.as_slice() {
        [i] => path_of(*i),
        [] => Err(format!("E94: No matching buffer for {arg}")),
        _ => match hits.iter().find(|&&i| ed.docs[i].buf.name() == arg) {
            Some(&i) => path_of(i),
            None => Err(format!("E93: More than one match for {arg}")),
        },
    }
}

/// `:normal`: types `keys` in Normal mode, then ends any insert or selection they left open.
fn normal(ed: &mut Editor, keys: &str, out: &mut Vec<Action>) {
    ed.mode = ed.rest_mode();
    ed.vim.replaying += 1;
    for c in keys.chars() {
        let k = if c == '\n' { KeyInput::plain(KeyCode::Enter) } else { KeyInput::ch(c) };
        dispatch(ed, k, out);
    }
    if ed.mode != ed.rest_mode() {
        dispatch(ed, KeyInput::plain(KeyCode::Esc), out);
    }
    ed.vim.pending.clear();
    ed.vim.replaying -= 1;
}
