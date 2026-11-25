use std::path::Path;

use super::*;

fn of(src: &str, file: &str) -> Vec<LineRuns> {
    let buf = Buffer::from_str(src);
    runs(&buf, Some(Path::new(file)), 0, buf.line_count())
}

fn kind_at(lines: &[LineRuns], line: usize, col: usize) -> Option<Kind> {
    lines[line].iter().find(|(r, _)| r.contains(&col)).map(|(_, k)| *k)
}

#[test]
fn rust_keywords_strings_and_comments_get_their_kinds() {
    let lines = of("fn main() {\n    let s = \"hi\"; // note\n}\n", "a.rs");
    assert_eq!(kind_at(&lines, 0, 0), Some(Kind::Keyword));
    assert_eq!(kind_at(&lines, 0, 3), Some(Kind::Function));
    assert_eq!(kind_at(&lines, 1, 13), Some(Kind::String));
    assert_eq!(kind_at(&lines, 1, 20), Some(Kind::Comment));
}

#[test]
fn every_language_query_compiles() {
    for file in ["a.rs", "a.py", "a.js", "a.ts", "a.tsx", "a.go", "a.c", "a.cpp", "A.java", "a.rb", "a.sh", "a.json", "a.lua"] {
        assert!(highlighted(Some(Path::new(file))), "{file}");
    }
}

#[test]
fn a_window_colours_only_its_lines() {
    let buf = Buffer::from_str("// one\nfn two() {}\n// three\n");
    let lines = runs(&buf, Some(Path::new("a.rs")), 1, 3);
    assert_eq!(lines.len(), 3);
    assert_eq!(kind_at(&lines, 0, 0), Some(Kind::Keyword));
    assert_eq!(kind_at(&lines, 1, 0), Some(Kind::Comment));
    assert!(lines[2].is_empty());
}

#[test]
fn files_with_no_grammar_have_no_runs() {
    let buf = Buffer::from_str("plain text\n");
    assert!(runs(&buf, Some(Path::new("notes.txt")), 0, 1)[0].is_empty());
    assert!(runs(&buf, None, 0, 1)[0].is_empty());
}
