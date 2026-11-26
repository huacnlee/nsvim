use std::path::PathBuf;

use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, Editor, KeyCode, KeyInput, Keymap};

use crate::services::lsp::Diag;

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

/// Wide chars get their own pieces, which the view spaces out to two cells each.
#[test]
fn wide_chars_get_their_own_pieces() {
    let f = frame(&editor_with("ab大家，cd\n", "a.txt"), 1);
    let Row::Line { segs, .. } = &f.rows[0] else { panic!() };
    let pieces: Vec<(&str, bool)> = segs.iter().map(|s| (s.text.as_str(), s.wide)).collect();
    assert_eq!(pieces, [("ab", false), ("大家，", true), ("cd", false)]);
}

#[test]
fn visual_mode_selects_and_changes_the_mode_pill() {
    let mut ed = editor_with("hello world\n", "a.txt");
    press(&mut ed, "vll");
    let f = frame(&ed, 1);
    let Row::Line { segs, cursorline, .. } = &f.rows[0] else { panic!() };
    assert_eq!(segs[0], Seg { text: "hel".into(), selected: true, ..Seg::default() });
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

fn key(ed: &mut Editor, code: KeyCode) {
    editor::vim::handle_key(ed, KeyInput::plain(code));
}

fn segs(row: &Row) -> &[Seg] {
    match row {
        Row::Line { segs, .. } => segs,
        Row::Past => &[],
    }
}

/// The text of the segments of `row` that `pick` takes, joined.
fn picked(row: &Row, pick: impl Fn(&Seg) -> bool) -> String {
    segs(row).iter().filter(|s| pick(s)).map(|s| s.text.as_str()).collect()
}

#[test]
fn search_matches_are_marked_until_nohlsearch() {
    let mut ed = editor_with("foo bar\nbar foo\n", "a.txt");
    press(&mut ed, "/bar");
    key(&mut ed, KeyCode::Enter);
    let f = frame(&ed, 2);
    assert_eq!(picked(&f.rows[0], |s| s.matched), "bar");
    assert_eq!(picked(&f.rows[1], |s| s.matched), "bar");
    press(&mut ed, ":noh");
    key(&mut ed, KeyCode::Enter);
    let f = frame(&ed, 2);
    assert_eq!(picked(&f.rows[0], |s| s.matched), "");
}

#[test]
fn extra_cursors_show_carets_and_their_selections() {
    let mut ed = editor_with("one two\nthree four\n", "a.txt");
    ed.set_keymap(Keymap::Classic);
    ed.doc_mut().extra.push(editor::editor::Cursor { pos: Pos::new(1, 2), anchor: None });
    ed.doc_mut().extra.push(editor::editor::Cursor { pos: Pos::new(0, 7), anchor: Some(Pos::new(0, 4)) });
    let f = frame(&ed, 2);
    let Row::Line { carets, .. } = &f.rows[1] else { panic!() };
    assert_eq!(carets, &[2]);
    let Row::Line { carets, .. } = &f.rows[0] else { panic!() };
    assert_eq!(carets, &[7]);
    assert_eq!(picked(&f.rows[0], |s| s.selected), "two");
    assert_eq!(f.caret.map(|c| (c.row, c.col, c.shape)), Some((0, 0, Shape::Bar)));
}

#[test]
fn long_lines_wrap_into_rows_and_clicks_find_their_chars() {
    let mut ed = editor_with("abcdefghijklmnopqrstuvwxy\nz\n", "a.txt");
    ed.opts.wrap = true;
    ed.cols = 10;
    press(&mut ed, "$");
    let f = frame(&ed, 5);
    let texts: Vec<String> = f.rows.iter().map(line_text).collect();
    assert_eq!(texts, ["abcdefghij", "klmnopqrst", "uvwxy", "z", "~"]);
    let numbers: Vec<String> =
        f.rows.iter().take(4).map(|r| if let Row::Line { number, .. } = r { number.clone() } else { String::new() }).collect();
    assert_eq!(numbers, ["1", "", "", "1"]);
    assert_eq!(f.caret.map(|c| (c.row, c.col)), Some((2, 4)));
    assert_eq!(pos_at(&ed, 1, 3.2), Some(Pos::new(0, 13)));
    assert_eq!(pos_at(&ed, 3, 0.0), Some(Pos::new(1, 0)));
    ed.opts.wrap = false;
    assert_eq!(
        frame(&ed, 2).rows.iter().map(line_text).collect::<Vec<_>>(),
        ["abcdefghijkl", "z"],
        "only the columns that fit, and a little more"
    );
}

#[test]
fn wrap_breaks_on_display_columns() {
    assert_eq!(wrap_starts(&columns("abcdef", 4), 3), [0, 3]);
    assert_eq!(wrap_starts(&columns("a\tb", 4), 4), [0, 2]);
    assert_eq!(wrap_starts(&columns("漢漢漢", 4), 4), [0, 2]);
    assert_eq!(wrap_starts(&columns("", 4), 4), [0]);
}

#[test]
fn whitespace_shows_leading_dots_and_arrows() {
    let ed = editor_with("  x y\n\tz\n", "a.txt");
    let look = Look { whitespace: true, ..Look::default() };
    let f = frame_with(&ed, &look, 2);
    assert_eq!(line_text(&f.rows[0]), "··x y");
    assert_eq!(picked(&f.rows[0], |s| s.ws), "··");
    assert_eq!(line_text(&f.rows[1]), "→   z");
    assert_eq!(line_text(&frame(&ed, 1).rows[0]), "  x y");
}

#[test]
fn diagnostics_underline_their_range_and_sign_the_gutter() {
    let ed = editor_with("let x = 1;\nlet y = 2;\n", "a.txt");
    let diags = [
        Diag { line: 0, col: 4, end_line: 0, end_col: 5, severity: 1, message: "unused x\nmore".into() },
        Diag { line: 1, col: 4, end_line: 1, end_col: 5, severity: 2, message: "unused y".into() },
    ];
    let look = Look { diags: &diags, ..Look::default() };
    let f = frame_with(&ed, &look, 2);
    assert_eq!(picked(&f.rows[0], |s| s.diag == Some(Level::Error)), "x");
    let Row::Line { sign, after, .. } = &f.rows[0] else { panic!() };
    assert_eq!(*sign, Some(Sign::Diag(Level::Error)));
    assert_eq!(after, &Some(After::Diag(Level::Error, "unused x".into())));
    let Row::Line { sign, .. } = &f.rows[1] else { panic!() };
    assert_eq!(*sign, Some(Sign::Diag(Level::Warning)));
}

#[test]
fn git_signs_and_outside_changes_mark_the_gutter() {
    use crate::services::git::{Sign as GitSign, SignKind};
    let ed = editor_with("a\nb\nc\nd\n", "a.txt");
    let signs = [
        GitSign { line: 0, kind: SignKind::Added },
        GitSign { line: 1, kind: SignKind::Modified },
        GitSign { line: 2, kind: SignKind::Deleted },
    ];
    let look = Look { signs: &signs, outside: &[3], ..Look::default() };
    let f = frame_with(&ed, &look, 4);
    let got: Vec<Option<Sign>> = f.rows.iter().map(|r| if let Row::Line { sign, .. } = r { *sign } else { None }).collect();
    assert_eq!(got, [Some(Sign::Added), Some(Sign::Modified), Some(Sign::Deleted), Some(Sign::Outside)]);
    assert_eq!(Sign::Added.glyph(), "▎");
}

#[test]
fn numbers_follow_the_option_and_the_ruler_shows() {
    let mut ed = editor_with("a\nb\nc\n", "a.txt");
    press(&mut ed, "j");
    let nums = |f: &Frame| -> Vec<String> {
        f.rows.iter().map(|r| if let Row::Line { number, .. } = r { number.clone() } else { String::new() }).collect()
    };
    assert_eq!(nums(&frame(&ed, 3)), ["1", "2", "1"]);
    let f = frame_with(&ed, &Look { numbers: "absolute", ruler: 80, ..Look::default() }, 3);
    assert_eq!(nums(&f), ["1", "2", "3"]);
    assert_eq!(f.ruler, Some(80));
    let f = frame_with(&ed, &Look { numbers: "off", ..Look::default() }, 3);
    assert_eq!(nums(&f), ["", "", ""]);
    assert_eq!(f.gutter, 2);
}

#[test]
fn a_note_tints_its_lines_and_shows_its_text() {
    let ed = editor_with("a\nb\nc\n", "a.txt");
    let look = Look { note: Some(NoteMark { first: 1, last: 2, text: "look here" }), ..Look::default() };
    let f = frame_with(&ed, &look, 3);
    let tints: Vec<bool> = f.rows.iter().map(|r| matches!(r, Row::Line { tint: true, .. })).collect();
    assert_eq!(tints, [false, true, true]);
    let Row::Line { after, .. } = &f.rows[1] else { panic!() };
    assert_eq!(after, &Some(After::Note("look here".into())));
}

#[test]
fn visual_line_fills_whole_lines_and_the_tree_hides_the_caret() {
    let mut ed = editor_with("ab\ncd\n", "a.txt");
    press(&mut ed, "V");
    let f = frame(&ed, 2);
    assert!(matches!(f.rows[0], Row::Line { full: true, .. }));
    assert!(matches!(f.rows[1], Row::Line { full: false, .. }));
    assert_eq!(f.caret.map(|c| (c.shape, c.visual, c.ch)), Some((Shape::Block, true, 'a')));
    assert_eq!(frame_with(&ed, &Look { hide_caret: true, ..Look::default() }, 2).caret, None);
}
