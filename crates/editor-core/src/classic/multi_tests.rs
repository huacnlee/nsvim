//! Multiple-cursor tests for the classic keymap, with Ctrl as the shortcut modifier.

use crate::buffer::Pos;
use crate::editor::{Action, Cursor, Keymap, Reg};
use crate::vim::tests::{cur, ed, feed, text};

fn classic(s: &str) -> crate::editor::Editor {
    let mut e = ed(s);
    e.set_keymap(Keymap::Classic);
    e
}

fn extra(e: &crate::editor::Editor) -> Vec<(usize, usize)> {
    e.doc().extra.iter().map(|c| (c.pos.line, c.pos.col)).collect()
}

#[test]
fn alt_ctrl_down_adds_cursors_that_type_together() {
    let mut e = classic("aa\nbb\ncc");
    feed(&mut e, "<c-a-down><c-a-down>");
    assert_eq!(extra(&e), vec![(1, 0), (2, 0)]);
    feed(&mut e, "X");
    assert_eq!(text(&e), "Xaa\nXbb\nXcc");
    assert_eq!(cur(&e), (0, 1));
    assert_eq!(extra(&e), vec![(1, 1), (2, 1)]);
    feed(&mut e, "<bs>");
    assert_eq!(text(&e), "aa\nbb\ncc");
    feed(&mut e, "<right>-");
    assert_eq!(text(&e), "a-a\nb-b\nc-c", "arrows move every cursor");
    feed(&mut e, "<c-z>");
    assert_eq!(text(&e), "aa\nbb\ncc", "undo takes back one typing run");
    assert!(e.doc().extra.is_empty(), "undo leaves one cursor");
}

#[test]
fn enter_and_delete_at_every_cursor() {
    let mut e = classic("  ab\n  cd");
    feed(&mut e, "<end><c-a-down><left><cr>");
    assert_eq!(text(&e), "  a\n  b\n  c\n  d", "Enter keeps each line's indent");
    let mut e = classic("ab\ncd");
    feed(&mut e, "<c-a-down><del>");
    assert_eq!(text(&e), "b\nd");
}

#[test]
fn ctrl_d_adds_a_cursor_on_the_next_match() {
    let mut e = classic("foo bar foo bar foo");
    feed(&mut e, "<c-d>");
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 0)));
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, "<c-d><c-d>");
    assert_eq!(extra(&e), vec![(0, 11), (0, 19)]);
    feed(&mut e, "<c-d>");
    assert_eq!(extra(&e).len(), 2, "every match is taken, so nothing more is added");
    feed(&mut e, "x");
    assert_eq!(text(&e), "x bar x bar x");
    feed(&mut e, "<c-z>");
    assert_eq!(text(&e), "foo bar foo bar foo");
}

#[test]
fn ctrl_shift_l_selects_every_match() {
    let mut e = classic("ab ab ab");
    feed(&mut e, "<c-s-l>");
    assert_eq!(extra(&e).len(), 2);
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 0)), "the match under the cursor stays the main one");
    feed(&mut e, "<bs>");
    assert_eq!(text(&e), "  ");
}

#[test]
fn copy_joins_selections_and_paste_splits_lines() {
    let mut e = classic("foo bar foo");
    feed(&mut e, "<c-d><c-d>");
    let acts = feed(&mut e, "<c-c>");
    assert!(acts.contains(&Action::Copy("foo\nfoo".into())));
    let mut e = classic("aa\nbb");
    feed(&mut e, "<c-a-down>");
    e.regs.unnamed = Reg { text: "1\n2".into(), linewise: false };
    feed(&mut e, "<c-v>");
    assert_eq!(text(&e), "1aa\n2bb", "one clipboard line per cursor");
    e.regs.unnamed = Reg { text: "z".into(), linewise: false };
    feed(&mut e, "<c-v>");
    assert_eq!(text(&e), "1zaa\n2zbb");
}

#[test]
fn copy_and_cut_whole_lines_without_selections() {
    let mut e = classic("one\ntwo\nthree");
    feed(&mut e, "<c-a-down>");
    let acts = feed(&mut e, "<c-x>");
    assert!(acts.contains(&Action::Copy("one\ntwo\n".into())));
    assert_eq!(text(&e), "three");
    assert!(e.doc().extra.is_empty());
}

#[test]
fn comment_and_delete_lines_take_every_cursor() {
    let mut e = classic("a\nb\nc");
    e.docs[0].extra.push(Cursor { pos: Pos::new(2, 0), anchor: None });
    feed(&mut e, "<c-/>");
    assert_eq!(text(&e), "// a\nb\n// c");
    let mut e = classic("a\nb\nc\nd");
    e.docs[0].extra.push(Cursor { pos: Pos::new(2, 0), anchor: None });
    feed(&mut e, "<c-s-k>");
    assert_eq!(text(&e), "b\nd");
}

#[test]
fn escape_and_other_keys_collapse_to_one_cursor() {
    let mut e = classic("aa\nbb");
    feed(&mut e, "<c-a-down><esc>");
    assert!(e.doc().extra.is_empty());
    feed(&mut e, "<c-a-down><c-a>");
    assert!(e.doc().extra.is_empty(), "select all works on one cursor");
}

#[test]
fn cursors_that_meet_merge() {
    let mut e = classic("ab");
    e.docs[0].extra.push(Cursor { pos: Pos::new(0, 1), anchor: None });
    feed(&mut e, "<home>");
    assert!(e.doc().extra.is_empty());
}

#[test]
fn alt_ctrl_up_adds_cursors_above_and_tab_indents_at_each() {
    let mut e = classic("aa\nbb\ncc");
    feed(&mut e, "<c-end><c-a-up><c-a-up>");
    assert_eq!(extra(&e), vec![(1, 2), (0, 2)]);
    feed(&mut e, "X<tab>");
    assert_eq!(text(&e), "aaX    \nbbX    \nccX    ");
}
