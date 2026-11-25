//! Fuzzy matching for the finder: the query's chars in order, scored so matches in the file
//! name, at word starts and in runs rank first.
//!
//! Ranking runs on every key in the finder, so it stays cheap on large projects. ASCII paths
//! score as bytes with no allocation per path. Only the best `limit` matches are kept and get
//! their hit positions. Long lists split across threads.

use std::cmp::Ordering;

/// A path that matches, its score and the char indices of the matched chars.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub index: usize,
    pub score: i64,
    pub hits: Vec<usize>,
}

/// Lists at least this long rank on several threads.
const PARALLEL_FROM: usize = 16 * 1024;
/// The most threads one ranking uses.
const MAX_THREADS: usize = 8;

/// Scores `text` against `query` (lowercase compare unless the query has an uppercase char).
/// `None` when the query's chars don't all appear in order. `rank` scores without the hit
/// list, so only tests call this.
#[cfg(test)]
pub fn score(query: &str, text: &str) -> Option<(i64, Vec<usize>)> {
    let q = Query::new(query);
    let mut s = Scratch::default();
    q.score(text, &mut s).map(|v| (v, s.hits))
}

/// The best `limit` matches of `query` among `items`, best first, and the number of matches.
/// Ties keep the shorter path, then the earlier item.
pub fn rank<S: AsRef<str> + Sync>(query: &str, items: &[S], limit: usize) -> (Vec<Match>, usize) {
    let q = Query::new(query);
    let (mut best, total) = if items.len() >= PARALLEL_FROM {
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, MAX_THREADS);
        let chunk = items.len().div_ceil(threads);
        std::thread::scope(|sc| {
            let q = &q;
            let jobs: Vec<_> =
                items.chunks(chunk).enumerate().map(|(n, part)| sc.spawn(move || best_of(q, part, n * chunk, limit))).collect();
            let mut all = Vec::with_capacity(limit * jobs.len());
            let mut total = 0;
            for job in jobs {
                let (part, n) = job.join().unwrap_or_default();
                all.extend(part);
                total += n;
            }
            (all, total)
        })
    } else {
        best_of(&q, items, 0, limit)
    };
    keep(&mut best, limit);
    best.sort_unstable_by(better);
    // Hit positions only for the matches that show.
    let mut s = Scratch::default();
    let out = best
        .into_iter()
        .map(|c| {
            q.score(items[c.index].as_ref(), &mut s);
            Match { index: c.index, score: c.score, hits: std::mem::take(&mut s.hits) }
        })
        .collect();
    (out, total)
}

/// A match before its hit positions are known.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    score: i64,
    len: usize,
    index: usize,
}

/// Best first: the higher score, then the shorter text, then the earlier item.
fn better(a: &Candidate, b: &Candidate) -> Ordering {
    b.score.cmp(&a.score).then(a.len.cmp(&b.len)).then(a.index.cmp(&b.index))
}

/// Keeps the best `limit` candidates, in no order.
fn keep(c: &mut Vec<Candidate>, limit: usize) {
    if limit > 0 && c.len() > limit {
        c.select_nth_unstable_by(limit - 1, better);
    }
    c.truncate(limit);
}

/// The best `limit` matches in `items` (numbered from `offset`), in no order, and the number
/// of matches.
fn best_of<S: AsRef<str>>(q: &Query, items: &[S], offset: usize, limit: usize) -> (Vec<Candidate>, usize) {
    let mut s = Scratch::default();
    let cap = (limit * 2).max(64);
    let mut out = Vec::with_capacity(cap.min(items.len()));
    let mut total = 0;
    for (i, it) in items.iter().enumerate() {
        let text = it.as_ref();
        let Some(score) = q.score(text, &mut s) else { continue };
        total += 1;
        if limit == 0 {
            continue;
        }
        out.push(Candidate { score, len: text.len(), index: offset + i });
        if out.len() >= cap {
            keep(&mut out, limit);
        }
    }
    keep(&mut out, limit);
    (out, total)
}

/// A query, normalized once for all the texts it scores.
struct Query {
    smart: bool,
    chars: Vec<char>,
    /// `chars` as bytes, when every char is ASCII.
    bytes: Option<Vec<u8>>,
}

/// Buffers reused from one text to the next.
#[derive(Default)]
struct Scratch {
    hits: Vec<usize>,
    chars: Vec<char>,
}

impl Query {
    fn new(query: &str) -> Query {
        let smart = query.chars().any(|c| c.is_uppercase());
        let chars: Vec<char> = query.chars().map(|c| c.norm(smart)).filter(|c| !c.is_whitespace()).collect();
        let bytes = chars.iter().all(char::is_ascii).then(|| chars.iter().map(|&c| c as u8).collect());
        Query { smart, chars, bytes }
    }

    /// The score of `text`, with the matched char indices in `s.hits`.
    fn score(&self, text: &str, s: &mut Scratch) -> Option<i64> {
        s.hits.clear();
        if self.chars.is_empty() {
            return Some(0);
        }
        if text.is_ascii() {
            // A char outside ASCII in the query never matches ASCII text.
            let q = self.bytes.as_deref()?;
            return score_units(text.as_bytes(), q, self.smart, &mut s.hits);
        }
        s.chars.clear();
        s.chars.extend(text.chars());
        score_units(&s.chars, &self.chars, self.smart, &mut s.hits)
    }
}

/// A char of text: a byte of ASCII text, or a `char` of any other text.
trait Unit: Copy + Eq {
    fn norm(self, smart: bool) -> Self;
    fn is_slash(self) -> bool;
    fn is_sep(self) -> bool;
    fn is_upper(self) -> bool;
    fn is_lower(self) -> bool;
}

impl Unit for u8 {
    fn norm(self, smart: bool) -> u8 {
        if smart { self } else { self.to_ascii_lowercase() }
    }
    fn is_slash(self) -> bool {
        self == b'/'
    }
    fn is_sep(self) -> bool {
        matches!(self, b'/' | b'_' | b'-' | b'.' | b' ')
    }
    fn is_upper(self) -> bool {
        self.is_ascii_uppercase()
    }
    fn is_lower(self) -> bool {
        self.is_ascii_lowercase()
    }
}

impl Unit for char {
    fn norm(self, smart: bool) -> char {
        if smart { self } else { self.to_ascii_lowercase() }
    }
    fn is_slash(self) -> bool {
        self == '/'
    }
    fn is_sep(self) -> bool {
        matches!(self, '/' | '_' | '-' | '.' | ' ')
    }
    fn is_upper(self) -> bool {
        self.is_uppercase()
    }
    fn is_lower(self) -> bool {
        self.is_lowercase()
    }
}

/// Scores the normalized query `q` (not empty) against `t`, with the hits in `hits`.
fn score_units<T: Unit>(t: &[T], q: &[T], smart: bool, hits: &mut Vec<usize>) -> Option<i64> {
    let name_start = t.iter().rposition(|c| c.is_slash()).map_or(0, |i| i + 1);
    // Left to right inside the file name first, then anywhere: cheap and good enough.
    if !find(t, q, name_start, smart, hits) && !find(t, q, 0, smart, hits) {
        return None;
    }
    let mut s: i64 = 0;
    let mut prev: Option<usize> = None;
    for &i in hits.iter() {
        s += 10;
        if i >= name_start {
            s += 12;
        }
        let at_word = i == 0 || t[i - 1].is_sep() || (t[i].is_upper() && t[i - 1].is_lower());
        if at_word {
            s += 18;
        }
        if let Some(p) = prev {
            if i == p + 1 {
                s += 16;
            } else {
                s -= (i - p) as i64 / 2;
            }
        }
        prev = Some(i);
    }
    if hits.first() == Some(&name_start) {
        s += 20;
    }
    // Shorter names first, then shorter paths.
    s -= (t.len() - name_start) as i64 / 2 + t.len() as i64 / 16;
    Some(s)
}

/// Finds the chars of `q` in order in `t` from `from`, the first of each, into `hits`.
fn find<T: Unit>(t: &[T], q: &[T], from: usize, smart: bool, hits: &mut Vec<usize>) -> bool {
    hits.clear();
    let mut j = 0;
    for (i, &c) in t.iter().enumerate().skip(from) {
        if c.norm(smart) == q[j] {
            hits.push(i);
            j += 1;
            if j == q.len() {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "fuzzy_tests.rs"]
mod tests;
