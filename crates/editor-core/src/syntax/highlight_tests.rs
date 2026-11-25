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
    for file in ["a.rs", "a.py", "a.js", "a.ts", "a.tsx", "a.go", "a.c", "a.cpp", "A.java", "a.rb", "a.sh", "a.json", "a.lua", "a.md"] {
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

#[test]
fn markdown_colours_headings_code_and_inline_marks() {
    let src = "# Title\n\nSome **bold**, *soft* and `code` with a [link](https://x.dev).\n\n```rust\nfn x() {}\n```\n- item\n\n```\nfn y() {}\n```\n";
    let lines = of(src, "README.md");
    assert_eq!(kind_at(&lines, 0, 2), Some(Kind::Keyword), "the heading");
    assert_eq!(kind_at(&lines, 2, 7), Some(Kind::Keyword), "bold");
    assert_eq!(kind_at(&lines, 2, 16), Some(Kind::Type), "italics");
    assert_eq!(kind_at(&lines, 2, 28), Some(Kind::String), "a code span");
    assert_eq!(kind_at(&lines, 2, 42), Some(Kind::Function), "the link text");
    assert_eq!(kind_at(&lines, 2, 50), Some(Kind::Comment), "the link target");
    assert_eq!(kind_at(&lines, 4, 0), Some(Kind::Punct), "a code fence");
    assert_eq!(kind_at(&lines, 5, 0), Some(Kind::Keyword), "Rust in a rust block");
    assert_eq!(kind_at(&lines, 5, 3), Some(Kind::Function), "Rust in a rust block");
    assert_eq!(kind_at(&lines, 10, 0), Some(Kind::String), "a block with no language");
    assert_eq!(kind_at(&lines, 7, 0), Some(Kind::Punct), "a list marker");
    assert_eq!(kind_at(&lines, 2, 0), None, "plain text");
}
