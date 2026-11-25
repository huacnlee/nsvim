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
    for file in [
        "a.rs",
        "a.py",
        "a.js",
        "a.ts",
        "a.tsx",
        "a.go",
        "a.c",
        "a.cpp",
        "A.java",
        "a.rb",
        "a.sh",
        "a.json",
        "a.lua",
        "a.md",
        "a.toml",
        "a.yaml",
        "a.html",
        "a.css",
        "a.xml",
        "Makefile",
        "Dockerfile",
        "a.zig",
        "a.php",
    ] {
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

#[test]
fn toml_colours_tables_keys_values_and_comments() {
    let src = "[package]\nname = \"nsvim\" # the name\nversion = 3\ndebug = true\n";
    for file in ["Cargo.toml", "Cargo.lock", "Pipfile"] {
        let lines = of(src, file);
        assert_eq!(kind_at(&lines, 0, 0), Some(Kind::Punct), "{file}: the table's bracket");
        assert_eq!(kind_at(&lines, 0, 1), Some(Kind::Type), "{file}: the table's name");
        assert_eq!(kind_at(&lines, 1, 0), Some(Kind::Property), "{file}: a key");
        assert_eq!(kind_at(&lines, 1, 8), Some(Kind::String), "{file}: a string");
        assert_eq!(kind_at(&lines, 1, 16), Some(Kind::Comment), "{file}: a comment");
        assert_eq!(kind_at(&lines, 2, 10), Some(Kind::Number), "{file}: a number");
        assert_eq!(kind_at(&lines, 3, 8), Some(Kind::Number), "{file}: a boolean");
    }
}

#[test]
fn markdown_colours_a_toml_block() {
    let lines = of("```toml\nkey = \"v\"\n```\n", "a.md");
    assert_eq!(kind_at(&lines, 1, 0), Some(Kind::Property));
    assert_eq!(kind_at(&lines, 1, 6), Some(Kind::String));
}

#[test]
fn yaml_html_css_and_xml_get_their_kinds() {
    let yaml = of("name: ci # build\nruns: 3\n", "ci.yml");
    assert_eq!(kind_at(&yaml, 0, 0), Some(Kind::Property), "a YAML key");
    assert_eq!(kind_at(&yaml, 0, 10), Some(Kind::Comment), "a YAML comment");
    assert_eq!(kind_at(&yaml, 1, 6), Some(Kind::Number), "a YAML number");

    let html = of("<a href=\"x\">link</a><!-- note -->\n", "index.html");
    assert_eq!(kind_at(&html, 0, 1), Some(Kind::Keyword), "an HTML tag");
    assert_eq!(kind_at(&html, 0, 3), Some(Kind::Macro), "an HTML attribute");
    assert_eq!(kind_at(&html, 0, 9), Some(Kind::String), "an HTML value");
    assert_eq!(kind_at(&html, 0, 13), None, "HTML text");
    assert_eq!(kind_at(&html, 0, 22), Some(Kind::Comment), "an HTML comment");

    let css = of("@media print {\n  a { color: red; }\n}\n", "site.css");
    assert_eq!(kind_at(&css, 0, 0), Some(Kind::Keyword), "a CSS at-rule");
    assert_eq!(kind_at(&css, 1, 2), Some(Kind::Keyword), "a CSS tag selector");
    assert_eq!(kind_at(&css, 1, 6), Some(Kind::Property), "a CSS property");

    let xml = of("<root id=\"1\"><!-- c --></root>\n", "pom.xml");
    assert_eq!(kind_at(&xml, 0, 1), Some(Kind::Keyword), "an XML tag");
    assert_eq!(kind_at(&xml, 0, 15), Some(Kind::Comment), "an XML comment");
}

#[test]
fn make_dockerfile_and_zig_get_their_kinds() {
    let make = of("# build\nall: main.o\n\tcc -o app main.o\n", "Makefile");
    assert_eq!(kind_at(&make, 0, 0), Some(Kind::Comment), "a Makefile comment");
    assert!(kind_at(&make, 1, 0).is_some(), "a Makefile target");

    let docker = of("FROM rust:1 AS build\n# note\nRUN cargo build\n", "Dockerfile");
    assert_eq!(kind_at(&docker, 0, 0), Some(Kind::Keyword), "a Dockerfile instruction");
    assert_eq!(kind_at(&docker, 1, 0), Some(Kind::Comment), "a Dockerfile comment");
    assert_eq!(kind_at(&docker, 2, 0), Some(Kind::Keyword), "a Dockerfile instruction");

    let zig = of("const std = @import(\"std\");\nfn add(a: u32) u32 {\n    return a;\n}\n", "a.zig");
    assert_eq!(kind_at(&zig, 0, 0), Some(Kind::Keyword), "a Zig keyword");
    assert_eq!(kind_at(&zig, 0, 21), Some(Kind::String), "a Zig string");
    assert_eq!(kind_at(&zig, 1, 3), Some(Kind::Function), "a Zig function name");
    // `lua-match?` reads as `match?`: a lower-case name is not a type.
    assert_ne!(kind_at(&zig, 2, 11), Some(Kind::Type), "a Zig variable");
}

#[test]
fn php_gets_its_kinds() {
    let php = of("<?php\nfunction add($a) { return \"x\"; } // note\n", "a.php");
    assert_eq!(kind_at(&php, 1, 0), Some(Kind::Keyword), "a PHP keyword");
    assert_eq!(kind_at(&php, 1, 9), Some(Kind::Function), "a PHP function name");
    assert_eq!(kind_at(&php, 1, 27), Some(Kind::String), "a PHP string");
    assert_eq!(kind_at(&php, 1, 36), Some(Kind::Comment), "a PHP comment");
}
