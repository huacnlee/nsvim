//! Classic keymap tests. They run with Ctrl as the shortcut modifier, as on Linux.

use crate::editor::{Action, Keymap, Mode};
use crate::vim::tests::{cur, ed, feed, text};

fn classic(s: &str) -> crate::editor::Editor {
    let mut e = ed(s);
    e.set_keymap(Keymap::Classic);
    e
}

#[test]
fn typing_inserts_and_undoes_as_one_run() {
    let mut e = classic("");
    feed(&mut e, "hello");
    assert_eq!(text(&e), "hello");
    assert_eq!(e.mode, Mode::Insert);
    feed(&mut e, "<left> x");
    assert_eq!(text(&e), "hell xo");
    feed(&mut e, "<c-z>");
    assert_eq!(text(&e), "hello", "the run after the arrow undoes alone");
    feed(&mut e, "<c-z>");
    assert_eq!(text(&e), "");
    feed(&mut e, "<c-s-z>");
    assert_eq!(text(&e), "hello");
}

#[test]
fn shift_select_then_typing_replaces() {
    let mut e = classic("hello world");
    feed(&mut e, "<s-right><s-right><s-right><s-right><s-right>");
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(0));
    feed(&mut e, "bye");
    assert_eq!(text(&e), "bye world");
    assert_eq!(e.doc().view.anchor, None);
    feed(&mut e, "<s-home><bs>");
    assert_eq!(text(&e), " world");
}

#[test]
fn ctrl_d_selects_the_word_then_adds_the_next_match() {
    let mut e = classic("foo bar foo");
    feed(&mut e, "<c-d>");
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(0));
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, "<c-d>");
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(0), "the first selection stays");
    let c = e.doc().extra[0];
    assert_eq!((c.anchor.map(|p| p.col), c.pos.col), (Some(8), 11));
}

#[test]
fn copy_cut_and_paste_whole_lines_without_a_selection() {
    let mut e = classic("one\ntwo\n");
    let acts = feed(&mut e, "<c-c>");
    assert!(acts.contains(&Action::Copy("one\n".into())));
    feed(&mut e, "<down><c-v>");
    assert_eq!(text(&e), "one\none\ntwo\n", "a copied line pastes above the cursor's line");
    feed(&mut e, "<c-x>");
    assert_eq!(text(&e), "one\none\n");
}

#[test]
fn paste_replaces_the_selection() {
    let mut e = classic("abc");
    feed(&mut e, "<s-right><c-c><end><s-left><c-v>");
    assert_eq!(text(&e), "aba");
    assert_eq!(cur(&e), (0, 3));
}

#[test]
fn alt_arrows_move_lines() {
    let mut e = classic("a\nb\nc");
    feed(&mut e, "<a-down>");
    assert_eq!(text(&e), "b\na\nc");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "<a-up><a-up>");
    assert_eq!(text(&e), "a\nb\nc");
}

#[test]
fn app_shortcuts() {
    let mut e = classic("x");
    assert_eq!(feed(&mut e, "<c-s>"), vec![Action::Save(None)]);
    assert_eq!(feed(&mut e, "<c-n>"), vec![Action::NewBuffer]);
    assert_eq!(feed(&mut e, "<c-p>"), vec![Action::FindFile]);
    assert_eq!(feed(&mut e, "<c-s-p>"), vec![Action::Palette]);
    assert_eq!(feed(&mut e, "<c-b>"), vec![Action::ToggleTree]);
    assert_eq!(feed(&mut e, "<f12>"), vec![Action::GotoDefinition]);
    assert_eq!(feed(&mut e, "<c-tab>"), vec![Action::NextBuffer]);
    assert_eq!(feed(&mut e, "<c-s-tab>"), vec![Action::PrevBuffer]);
}

#[test]
fn find_opens_the_search_line_and_selects_the_match() {
    let mut e = classic("one two one");
    feed(&mut e, "<c-f>");
    assert_eq!(e.mode, crate::editor::Mode::Command);
    feed(&mut e, "two<cr>");
    assert_eq!(e.mode, Mode::Insert);
    assert_eq!(e.doc().view.anchor.map(|p| p.col), Some(4));
    assert_eq!(cur(&e), (0, 7));
}

#[test]
fn comment_indent_and_delete_line() {
    let mut e = classic("a\nb");
    feed(&mut e, "<c-/>");
    assert_eq!(text(&e), "// a\nb");
    feed(&mut e, "<c-/><c-]>");
    assert_eq!(text(&e), "    a\nb");
    feed(&mut e, "<c-[>");
    assert_eq!(text(&e), "a\nb");
    feed(&mut e, "<c-s-k>");
    assert_eq!(text(&e), "b");
}

#[test]
fn enter_keeps_indent_and_backspace_joins() {
    let mut e = classic("    x");
    feed(&mut e, "<end><cr>y");
    assert_eq!(text(&e), "    x\n    y");
    feed(&mut e, "<home><home><bs>");
    assert_eq!(text(&e), "    x    y");
}

#[test]
fn word_moves_with_ctrl_off_macos() {
    if cfg!(target_os = "macos") {
        return;
    }
    let mut e = classic("foo bar baz");
    feed(&mut e, "<c-right>");
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, "<c-right><c-left>");
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "<c-bs>");
    assert_eq!(text(&e), "bar baz");
}

#[test]
fn redo_select_all_home_end_tab_and_find_keys() {
    use crate::buffer::Pos;
    let mut e = classic("ab\n  cd");
    feed(&mut e, "x<c-z><c-y>");
    assert_eq!(text(&e), "xab\n  cd", "Ctrl+Y redoes");
    feed(&mut e, "<c-a>");
    assert_eq!((e.doc().view.anchor, cur(&e)), (Some(Pos::new(0, 0)), (1, 4)));
    feed(&mut e, "<tab>");
    assert_eq!(text(&e), "    xab\n      cd", "Tab indents a selection over several lines");
    feed(&mut e, "<s-tab>");
    assert_eq!(text(&e), "xab\n  cd");
    feed(&mut e, "<esc>");
    assert_eq!(e.doc().view.anchor, None, "Esc drops the selection");
    for (keys, want) in [("<c-home>", (0, 0)), ("<c-end>", (1, 4)), ("<home>", (1, 2)), ("<home>", (1, 0)), ("<s-end>", (1, 4))] {
        feed(&mut e, keys);
        assert_eq!(cur(&e), want, "after {keys}");
    }
    assert_eq!(e.doc().view.anchor, Some(Pos::new(1, 0)));
    if !cfg!(target_os = "macos") {
        feed(&mut e, "<c-home><c-del>");
        assert_eq!(text(&e), "\n  cd", "Ctrl+Delete deletes the word after the cursor");
    }
    feed(&mut e, "<c-a-f>");
    assert_eq!((e.mode, e.cmd.as_ref().map(|c| c.text.as_str())), (Mode::Command, Some("%s/")));
    feed(&mut e, "<esc>");
    assert_eq!(feed(&mut e, "<c-s-f>"), vec![Action::GrepProject]);
    assert_eq!(feed(&mut e, "<c-w>"), vec![Action::CloseBuffer]);
    assert_eq!(feed(&mut e, "<c-,>"), vec![Action::Settings]);
    let src: String = (0..50).map(|_| "x\n").collect();
    let mut e = classic(&src);
    feed(&mut e, "<pgdn>");
    assert_eq!(cur(&e).0, 19);
    feed(&mut e, "<s-pgup>");
    assert_eq!((cur(&e).0, e.doc().view.anchor.map(|p| p.line)), (0, Some(19)));
}
