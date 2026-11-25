use std::path::PathBuf;

use editor::buffer::Buffer;
use editor::editor::{Doc, Editor, KeyCode, KeyInput};

use super::*;

fn editor_with(text: &str, file: &str) -> Editor {
    let mut ed = Editor::new(PathBuf::from("/p"));
    let mut buf = Buffer::from_str(text);
    buf.path = Some(PathBuf::from("/p").join(file));
    ed.docs.push(Doc::new(buf));
    ed
}

fn press(ed: &mut Editor, keys: &str) {
    for c in keys.chars() {
        editor::vim::handle_key(ed, KeyInput::plain(KeyCode::Char(c)));
    }
}

fn line_text(row: &Row) -> String {
    match row {
        Row::Line { segs, .. } => segs.iter().map(|s| s.text.as_str()).collect(),
        Row::Past => "~".into(),
    }
}

#[test]
fn rows_past_the_end_are_tildes() {
    let f = frame(&editor_with("one\ntwo\n", "a.txt"), 4);
    assert_eq!(f.rows.iter().map(line_text).collect::<Vec<_>>(), ["one", "two", "~", "~"]);
}

#[test]
fn code_gets_kinds_and_the_status_names_the_language() {
    let f = frame(&editor_with("fn main() {}\n", "src/main.rs"), 2);
    let Row::Line { segs, .. } = &f.rows[0] else { panic!() };
    assert_eq!(segs[0].text, "fn");
    assert_eq!(segs[0].kind, Some(Kind::Keyword));
    assert_eq!(f.status.lang, "rust");
    assert_eq!(f.status.file, "src/main.rs");
}

#[test]
fn tabs_expand_and_the_caret_counts_display_columns() {
    let mut ed = editor_with("\tx = 1\n", "a.py");
    press(&mut ed, "l");
    let f = frame(&ed, 1);
    assert_eq!(line_text(&f.rows[0]), "    x = 1");
    assert_eq!(f.caret.map(|c| (c.row, c.col)), Some((0, 4)));
}

#[test]
fn visual_mode_selects_and_changes_the_mode_pill() {
    let mut ed = editor_with("hello world\n", "a.txt");
    press(&mut ed, "vll");
    let f = frame(&ed, 1);
    let Row::Line { segs, cursorline, .. } = &f.rows[0] else { panic!() };
    assert_eq!(segs[0], Seg { text: "hel".into(), kind: None, selected: true });
    assert!(!cursorline);
    assert_eq!((f.status.mode, f.status.tone), ("VISUAL", Tone::Visual));
}

#[test]
fn the_command_line_shows_what_is_typed() {
    let mut ed = editor_with("x\n", "a.txt");
    press(&mut ed, ":wq");
    let f = frame(&ed, 1);
    assert_eq!(f.bottom, Bottom::Command { text: ":wq".into(), caret: 3 });
    assert_eq!(f.caret, None);
}

#[test]
fn clicks_map_display_columns_to_chars() {
    assert_eq!(char_at_column("\tab", 4, 2), 0);
    assert_eq!(char_at_column("\tab", 4, 5), 2);
    assert_eq!(char_at_column("ab", 4, 9), 2);
    assert_eq!(char_at_column("漢x", 4, 1), 0);
    assert_eq!(char_at_column("漢x", 4, 2), 1);
}
