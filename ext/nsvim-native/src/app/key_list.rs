//! The key list (`F1`, `SPC ?`, `:Keys`): every key in `docs/keymaps.md`, with search. The
//! list comes from the doc at build time, so the two never disagree.

use crate::fuzzy::Match;

const DOC: &str = include_str!("../../../../docs/keymaps.md");

/// The widest key column, in chars. A longer key pushes its description to the right.
const KEY_COLUMN: usize = 26;

/// One row: the keys, what they do, and the section of the doc they come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRow {
    pub keys: String,
    pub does: String,
    pub section: String,
}

/// The rows for one keymap: its own sections and the ones for both keymaps.
pub fn rows(vim: bool) -> Vec<KeyRow> {
    let (other, this) = if vim { ("Classic keymap", "Neovim keymap") } else { ("Neovim keymap", "Classic keymap") };
    let mut out = Vec::new();
    let mut top = "";
    let mut section = "";
    let mut header: Option<Vec<String>> = None;
    for line in DOC.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            (top, section, header) = (h, h, None);
            continue;
        }
        if let Some(h) = line.strip_prefix("### ") {
            (section, header) = (h, None);
            continue;
        }
        if !line.starts_with('|') {
            header = None;
            continue;
        }
        let cells = cells(line);
        if header.is_none() {
            header = Some(cells);
            continue;
        }
        let head = header.as_ref().and_then(|h| h.first()).cloned().unwrap_or_default();
        // Only the classic keymap opens the command palette, so its commands are not keys in Neovim.
        let palette = vim && section == "Command palette";
        if cells.iter().all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':')) || top == other || top == "Mouse" || palette {
            continue;
        }
        let mut push = |keys: String, does: &str| {
            let does = does.trim();
            if does.ends_with(other) {
                return;
            }
            let does = does.strip_suffix(this).map(|d| d.trim_end().trim_end_matches('.')).unwrap_or(does);
            out.push(KeyRow { keys, does: does.to_string(), section: section.to_string() });
        };
        let col = |i: usize| cells.get(i).map(String::as_str).unwrap_or("");
        match head.as_str() {
            // Text objects: one row for the object around, one for inside it.
            "Object" => {
                push(col(1).to_string(), &format!("{}, around", col(0)));
                push(col(2).to_string(), &format!("{}, inner", col(0)));
            }
            "Option" => {
                let keys = if col(1).is_empty() { format!(":set {}", col(0)) } else { format!(":set {}, {}", col(0), col(1)) };
                push(keys, col(2));
            }
            _ => push(col(0).to_string(), &cells[1..].join(" · ")),
        }
    }
    out
}

/// The picker's lines: the keys in a column, what they do, then a tab and the section.
pub fn items(vim: bool) -> Vec<String> {
    let rows = rows(vim);
    let width = rows.iter().map(|r| r.keys.chars().count()).filter(|&n| n <= KEY_COLUMN).max().unwrap_or(0);
    rows.iter().map(|r| format!("{:<width$}  {}\t{}", r.keys, r.does, r.section)).collect()
}

/// The rows that hold every word of `query`, in the order of the doc. Loose fuzzy matching
/// finds most rows for a word such as `tree`, so this search takes whole words. Case counts
/// only when the query has an uppercase letter, as with `smartcase`.
pub fn filter(query: &str, items: &[String]) -> Vec<Match> {
    let case = query.chars().any(char::is_uppercase);
    let fold = |s: &str| if case { s.to_string() } else { s.to_lowercase() };
    let words: Vec<String> = query.split_whitespace().map(fold).collect();
    let mut out = Vec::new();
    'rows: for (index, item) in items.iter().enumerate() {
        let hay = fold(item);
        let mut hits = Vec::new();
        for w in &words {
            let Some(b) = hay.find(w.as_str()) else { continue 'rows };
            let start = hay[..b].chars().count();
            hits.extend(start..start + w.chars().count());
        }
        out.push(Match { index, score: 0, hits });
    }
    out
}

/// The cells of a table row, as plain text.
fn cells(line: &str) -> Vec<String> {
    let inner = line.trim().trim_start_matches('|');
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut out = Vec::new();
    let mut cell = String::new();
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' if chars.get(i + 1) == Some(&'|') => {
                cell.push('|');
                i += 2;
            }
            // A code span: a run of backticks, then text up to a run of the same length.
            '`' => {
                let run = chars[i..].iter().take_while(|&&c| c == '`').count();
                let body = i + run;
                let close = (body..chars.len())
                    .find(|&j| chars[j..].iter().take_while(|&&c| c == '`').count() == run && (j == 0 || chars[j - 1] != '`'));
                match close {
                    Some(j) => {
                        let text: String = chars[body..j].iter().collect();
                        let text = text.replace("\\|", "|");
                        let text =
                            if text.starts_with(' ') && text.ends_with(' ') && text.len() > 2 { &text[1..text.len() - 1] } else { &text };
                        cell.push_str(text);
                        i = j + run;
                    }
                    None => {
                        cell.extend(&chars[i..body]);
                        i = body;
                    }
                }
            }
            '|' => {
                out.push(plain_links(cell.trim()));
                cell.clear();
                i += 1;
            }
            c => {
                cell.push(c);
                i += 1;
            }
        }
    }
    out.push(plain_links(cell.trim()));
    out
}

/// `[text](url)` becomes `text`.
fn plain_links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(open) = rest.find('[') {
        let Some(mid) = rest[open..].find("](").map(|m| open + m) else { break };
        let Some(end) = rest[mid..].find(')').map(|e| mid + e) else { break };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..mid]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_spans_escaped_pipes_and_links_become_plain_text() {
        assert_eq!(cells("| `j` `↓`, `k` `↑` | Move down, up |"), vec!["j ↓, k ↑", "Move down, up"]);
        assert_eq!(cells("| `{N}\\|` | Column `N` |"), vec!["{N}|", "Column N"]);
        assert_eq!(cells("| Quotes | ``a\" a' a` `` | ``i\" i' i` `` |"), vec!["Quotes", "a\" a' a`", "i\" i' i`"]);
        assert_eq!(cells("| `theme` | | see [the docs](configuration.md) |"), vec!["theme", "", "see the docs"]);
    }

    #[test]
    fn each_keymap_lists_its_own_keys_and_the_shared_ones() {
        let vim = rows(true);
        let classic = rows(false);
        let has = |rows: &[KeyRow], keys: &str| rows.iter().any(|r| r.keys == keys);
        assert!(has(&vim, "SPC f") && !has(&classic, "SPC f"), "the leader is Neovim only");
        assert!(has(&classic, "Ctrl D") && !has(&vim, "Ctrl D"), "classic rows stay in classic");
        assert!(!has(&vim, "Ctrl P, Ctrl ⇧ P, Ctrl ⇧ F, Ctrl B, Ctrl Tab, Ctrl Q"), "a row marked classic is left out of Neovim");
        assert!(has(&vim, "j ↓, k ↑") && has(&classic, "j ↓, k ↑"), "the tree is in both");
        let aw = vim.iter().find(|r| r.keys == "aw aW").expect("text objects");
        assert_eq!(aw.does, "Word, WORD, around");
        assert!(vim.iter().any(|r| r.keys.starts_with(":set number, nu")), "options");
        let colon = vim.iter().find(|r| r.section == "File tree" && r.keys == ":").expect("tree :");
        assert_eq!(colon.does, "Open the command line", "the keymap note is cut");
        // Backticks that stay are keys, such as `` `{mark} ``. A code span left whole would start and end with one.
        let spans: Vec<&str> =
            vim.iter().map(|r| r.keys.as_str()).filter(|k| k.len() > 2 && k.starts_with('`') && k.ends_with('`')).collect();
        assert!(spans.is_empty(), "{spans:?}");
        assert!(vim.len() > 300, "{}", vim.len());
    }

    #[test]
    fn search_takes_every_word_and_counts_case_only_with_an_uppercase_letter() {
        let items: Vec<String> = ["j ↓  Move down\tFile tree", "G  Go to the last line\tMotions", "g  Go to the first row\tFile tree"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let found = |q: &str| filter(q, &items).iter().map(|m| m.index).collect::<Vec<_>>();
        assert_eq!(found("tree"), vec![0, 2], "the section is part of the row");
        assert_eq!(found("tree first"), vec![2], "every word must be there");
        assert_eq!(found("go"), vec![1, 2]);
        assert_eq!(found("move"), vec![0]);
        assert!(found("MOVE").is_empty(), "an uppercase letter makes case count");
        assert_eq!(filter("down", &items)[0].hits, vec![10, 11, 12, 13], "hits are char indices, so the arrow counts as one");
    }
}
