//! `.editorconfig`: per-project indentation, line endings and save rules.
//! See <https://editorconfig.org>. Files are read from the file's folder up to the first
//! `root = true`, and nearer files win.

use std::path::{Path, PathBuf};

use globset::GlobBuilder;

/// What `.editorconfig` files say about one file. `None` leaves the editor's setting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorConfig {
    pub indent_spaces: Option<bool>,
    pub indent_size: Option<usize>,
    pub crlf: Option<bool>,
    pub trim_trailing_whitespace: Option<bool>,
    pub insert_final_newline: Option<bool>,
}

/// Reads every `.editorconfig` that applies to `file`. Does file I/O: call it off the UI thread.
pub fn for_file(file: &Path) -> EditorConfig {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    let mut dir = file.parent();
    while let Some(d) = dir {
        if let Ok(src) = std::fs::read_to_string(d.join(".editorconfig")) {
            let root = is_root(&src);
            files.push((d.to_path_buf(), src));
            if root {
                break;
            }
        }
        dir = d.parent();
    }
    let mut out = EditorConfig::default();
    // Farthest first, so nearer files override.
    for (dir, src) in files.iter().rev() {
        apply(&mut out, dir, src, file);
    }
    out
}

fn is_root(src: &str) -> bool {
    src.lines().take_while(|l| !l.trim_start().starts_with('[')).any(|l| {
        let (k, v) = l.split_once('=').unwrap_or(("", ""));
        k.trim().eq_ignore_ascii_case("root") && v.trim().eq_ignore_ascii_case("true")
    })
}

fn apply(out: &mut EditorConfig, dir: &Path, src: &str, file: &Path) {
    let Ok(rel) = file.strip_prefix(dir) else { return };
    let rel = rel.to_string_lossy().replace('\\', "/");
    let mut active = false;
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if let Some(pat) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            active = matches(pat, &rel);
            continue;
        }
        if !active {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_ascii_lowercase());
        let flag = || match v.as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        };
        match k.as_str() {
            "indent_style" => {
                out.indent_spaces = match v.as_str() {
                    "space" => Some(true),
                    "tab" => Some(false),
                    _ => out.indent_spaces,
                }
            }
            "indent_size" => {
                if let Ok(n) = v.parse::<usize>() {
                    out.indent_size = Some(n.clamp(1, 16));
                }
            }
            "tab_width" => {
                if out.indent_size.is_none() {
                    out.indent_size = v.parse::<usize>().ok().map(|n| n.clamp(1, 16));
                }
            }
            "end_of_line" => {
                out.crlf = match v.as_str() {
                    "crlf" => Some(true),
                    "lf" => Some(false),
                    _ => out.crlf,
                }
            }
            "trim_trailing_whitespace" => out.trim_trailing_whitespace = flag().or(out.trim_trailing_whitespace),
            "insert_final_newline" => out.insert_final_newline = flag().or(out.insert_final_newline),
            _ => {}
        }
    }
}

/// Whether section pattern `pat` matches `rel`, the file's path relative to the
/// `.editorconfig`'s folder. A pattern without `/` matches the file name in any folder.
fn matches(pat: &str, rel: &str) -> bool {
    let pat = if pat.contains('/') { pat.trim_start_matches('/').to_string() } else { format!("**/{pat}") };
    GlobBuilder::new(&pat).literal_separator(true).build().map(|g| g.compile_matcher().is_match(rel)).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearer_files_win_and_globs_match_names_anywhere() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".editorconfig"),
            "root = true\n[*]\nindent_style = space\nindent_size = 4\n[*.{js,ts}]\nindent_size = 2\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("web/src")).unwrap();
        std::fs::write(dir.path().join("web/.editorconfig"), "[Makefile]\nindent_style = tab\n").unwrap();
        let js = for_file(&dir.path().join("web/src/app.ts"));
        assert_eq!(js.indent_size, Some(2));
        assert_eq!(js.indent_spaces, Some(true));
        let mk = for_file(&dir.path().join("web/Makefile"));
        assert_eq!(mk.indent_spaces, Some(false));
        assert_eq!(for_file(&dir.path().join("main.rs")).indent_size, Some(4));
    }
}
