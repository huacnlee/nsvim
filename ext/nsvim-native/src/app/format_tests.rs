use editor::buffer::{Buffer, Pos};

use super::apply_edits;
use super::format::line_after;
use crate::services::lsp::TextEdit;

fn edit(sl: usize, sc: usize, el: usize, ec: usize, text: &str) -> TextEdit {
    TextEdit { start: Pos::new(sl, sc), end: Pos::new(el, ec), text: text.into() }
}

#[test]
fn edits_apply_in_any_order_as_one_undo_step() {
    let mut b = Buffer::from_str("fn  a(){\nlet x=1;\n}\n");
    // Out of order, as some servers send them.
    let edits = [edit(1, 0, 1, 0, "    "), edit(0, 2, 0, 4, " "), edit(1, 5, 1, 6, " = "), edit(0, 7, 0, 7, " ")];
    apply_edits(&mut b, &edits);
    assert_eq!(b.text(), "fn a() {\n    let x = 1;\n}\n");
    b.undo();
    assert_eq!(b.text(), "fn  a(){\nlet x=1;\n}\n", "one undo takes back every edit");
}

#[test]
fn inserts_at_one_place_keep_their_order() {
    let mut b = Buffer::from_str("ab\n");
    apply_edits(&mut b, &[edit(0, 1, 0, 1, "1"), edit(0, 1, 0, 1, "2"), edit(0, 1, 0, 2, "B")]);
    assert_eq!(b.text(), "a12B\n");
}

#[test]
fn multi_byte_text_counts_chars() {
    let mut b = Buffer::from_str("let 名前=\"😀\";\n");
    // Char columns: `=` is char 6, the emoji char 8.
    apply_edits(&mut b, &[edit(0, 6, 0, 7, " = "), edit(0, 8, 0, 9, "🙂")]);
    assert_eq!(b.text(), "let 名前 = \"🙂\";\n");
}

#[test]
fn a_crlf_file_keeps_its_line_ends() {
    let mut b = Buffer::from_str("a\r\nb\r\n");
    apply_edits(&mut b, &[edit(0, 0, 1, 0, "a\n\n")]);
    assert_eq!(b.text(), "a\n\nb\n");
    assert_eq!(b.line_ending, editor::buffer::LineEnding::CrLf);
}

#[test]
fn the_cursor_line_follows_edits_above_it() {
    // Two lines removed above line 5.
    assert_eq!(line_after(5, &[edit(1, 0, 3, 0, "")]), 3);
    // One line added above, one edit below.
    assert_eq!(line_after(5, &[edit(0, 0, 0, 0, "x\n"), edit(7, 0, 9, 0, "")]), 6);
    // An edit over lines 3..=6 that leaves one line: line 5 stays in it, on line 3.
    assert_eq!(line_after(5, &[edit(3, 0, 6, 2, "one\n")]), 4);
    assert_eq!(line_after(5, &[edit(3, 0, 6, 2, "one")]), 3);
    // Edits on the cursor's own line keep it.
    assert_eq!(line_after(5, &[edit(5, 0, 5, 4, "")]), 5);
}
