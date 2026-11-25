//! Project search behind `SPC /`: ripgrep when it is installed, else a plain walk.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{Sender, channel};

use super::{Event, Out};

/// The search stops after this many matches.
const MAX_HITS: usize = 2000;

pub struct Job {
    pub pattern: String,
    pub generation: u64,
}

/// One match: the file relative to the root, a zero-based line and char column, and the line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: String,
    pub line: usize,
    pub col: usize,
    pub text: String,
}

pub fn start(root: PathBuf, out: Out) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("editor-grep".into())
        .spawn(move || {
            while let Ok(mut job) = rx.recv() {
                // Only the newest query matters while the user types.
                while let Ok(newer) = rx.try_recv() {
                    job = newer;
                }
                if job.pattern.is_empty() {
                    out.send(Event::Grep { generation: job.generation, hits: Vec::new(), error: None });
                    continue;
                }
                let (hits, error) = match ripgrep(&root, &job.pattern) {
                    Some(r) => r,
                    None => (walk_search(&root, &job.pattern), None),
                };
                out.send(Event::Grep { generation: job.generation, hits, error });
            }
        })
        .expect("start the grep worker");
    tx
}

/// `None` when `rg` isn't installed.
fn ripgrep(root: &Path, pattern: &str) -> Option<(Vec<Hit>, Option<String>)> {
    let out = Command::new("rg")
        .current_dir(root)
        .args([
            "--vimgrep",
            "--smart-case",
            "--fixed-strings",
            "--max-columns",
            "400",
            "--max-columns-preview",
            "--no-heading",
            "--color",
            "never",
            "--",
            pattern,
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let mut hits = Vec::new();
    for line in text.lines() {
        if let Some(h) = parse_vimgrep(line) {
            hits.push(h);
            if hits.len() >= MAX_HITS {
                break;
            }
        }
    }
    let error = (!out.status.success() && out.status.code() != Some(1)).then(|| String::from_utf8_lossy(&out.stderr).trim().to_string());
    Some((hits, error.filter(|e| !e.is_empty())))
}

/// `path:line:col:text`, with 1-based line and byte column.
fn parse_vimgrep(line: &str) -> Option<Hit> {
    let mut it = line.splitn(4, ':');
    let path = it.next()?.trim_start_matches("./").to_string();
    let ln: usize = it.next()?.parse().ok()?;
    let col: usize = it.next()?.parse().ok()?;
    let text = it.next().unwrap_or("").to_string();
    let char_col = text.get(..col.saturating_sub(1)).map_or(0, |s| s.chars().count());
    Some(Hit { path, line: ln.saturating_sub(1), col: char_col, text })
}

fn walk_search(root: &Path, pattern: &str) -> Vec<Hit> {
    let smart = pattern.chars().any(|c| c.is_uppercase());
    let needle = if smart { pattern.to_string() } else { pattern.to_lowercase() };
    let mut hits = Vec::new();
    for rel in super::index::walk(root) {
        let Ok(src) = std::fs::read_to_string(root.join(&rel)) else { continue };
        for (i, line) in src.lines().enumerate() {
            let hay = if smart { line.to_string() } else { line.to_lowercase() };
            if let Some(b) = hay.find(&needle) {
                hits.push(Hit { path: rel.clone(), line: i, col: hay[..b].chars().count(), text: line.to_string() });
                if hits.len() >= MAX_HITS {
                    return hits;
                }
            }
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vimgrep_lines_parse_into_zero_based_char_positions() {
        let h = parse_vimgrep("./src/main.rs:3:9:    let é = foo;").unwrap();
        assert_eq!((h.path.as_str(), h.line, h.col), ("src/main.rs", 2, 8));
        let h = parse_vimgrep("a.rs:1:7:    é foo").unwrap();
        assert_eq!(h.col, 5, "byte column 7 is char column 5 after a two-byte char");
    }
}
