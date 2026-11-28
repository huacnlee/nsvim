//! The desktop editor keys of the Neovim keymap: `⌘` shortcuts on macOS, and the Insert mode
//! editing keys of the system's text fields.

use super::system;
use super::tests::{cur, ed, feed, parse_key, text};
use crate::editor::{Action, Editor, Mode, VisualKind};

/// Runs a `⌘` shortcut as macOS does, on any platform.
fn sc(e: &mut Editor, key: &str) -> Vec<Action> {
    let mut out = Vec::new();
    assert!(system::shortcut(e, parse_key(key), &mut out), "{key} is a shortcut");
    super::settle(e);
    out
}

/// Runs an Insert mode editing key as macOS (`mac`) or another platform does.
fn edit(e: &mut Editor, key: &str, mac: bool) {
    assert!(system::edit(e, parse_key(key), mac), "{key} is an editing key");
}

#[test]
fn cmd_backspace_deletes_to_the_indent_then_the_line_start() {
    let mut e = ed("    let x = 1;\nnext");
    feed(&mut e, "A");
    edit(&mut e, "d-bs", true);
    assert_eq!(text(&e), "    \nnext");
    edit(&mut e, "d-bs", true);
    assert_eq!(text(&e), "\nnext");
    feed(&mut e, "<down>");
    edit(&mut e, "d-bs", true);
    assert_eq!(text(&e), "next", "at the line start it joins with the line above");
    assert_eq!(e.mode, Mode::Insert);
    feed(&mut e, "<esc>u");
    assert_eq!(text(&e), "    let x = 1;\nnext", "the insert undoes as one step");
}

#[test]
fn cmd_delete_deletes_to_the_line_end() {
    let mut e = ed("foo bar\nbaz");
    feed(&mut e, "llli");
    edit(&mut e, "d-del", true);
    assert_eq!(text(&e), "foo\nbaz");
    edit(&mut e, "d-del", true);
    assert_eq!(text(&e), "foobaz", "at the line end it joins the next line");
}

#[test]
fn word_keys_use_alt_on_macos_and_ctrl_elsewhere() {
    for (mac, m) in [(true, "a-"), (false, "c-")] {
        let mut e = ed("foo.bar baz");
        feed(&mut e, "A");
        edit(&mut e, &format!("{m}bs"), mac);
        assert_eq!(text(&e), "foo.bar ", "mac {mac}");
        edit(&mut e, &format!("{m}left"), mac);
        assert_eq!(cur(&e), (0, 4));
        edit(&mut e, &format!("{m}del"), mac);
        assert_eq!(text(&e), "foo. ");
        edit(&mut e, &format!("{m}right"), mac);
        assert_eq!(cur(&e), (0, 5));
    }
    let mut e = ed("foo bar");
    feed(&mut e, "A");
    assert!(!system::edit(&mut e, parse_key("c-bs"), true), "Ctrl Backspace stays Vim's on macOS");
    assert!(!system::edit(&mut e, parse_key("d-bs"), false), "only macOS has the line keys");
}

#[test]
fn cmd_arrows_go_to_the_line_and_file_ends() {
    let mut e = ed("a\n  bc\nd");
    feed(&mut e, "jA");
    edit(&mut e, "d-left", true);
    assert_eq!(cur(&e), (1, 2), "first the first non-blank");
    edit(&mut e, "d-left", true);
    assert_eq!(cur(&e), (1, 0), "then the line start");
    edit(&mut e, "d-right", true);
    assert_eq!(cur(&e), (1, 4));
    edit(&mut e, "d-down", true);
    assert_eq!(cur(&e), (2, 1));
    edit(&mut e, "d-up", true);
    assert_eq!(cur(&e), (0, 0));
    edit(&mut e, "c-end", false);
    assert_eq!(cur(&e), (2, 1), "Ctrl End goes to the file end, as in Vim");
    edit(&mut e, "c-home", false);
    assert_eq!(cur(&e), (0, 0));
}

#[test]
fn cmd_z_undoes_in_every_mode() {
    let mut e = ed("one");
    feed(&mut e, "A two<esc>");
    sc(&mut e, "d-z");
    assert_eq!(text(&e), "one");
    sc(&mut e, "d-s-z");
    assert_eq!(text(&e), "one two", "⇧ ⌘ Z redoes");
    feed(&mut e, "A three");
    sc(&mut e, "d-z");
    assert_eq!(text(&e), "one two");
    assert_eq!(e.mode, Mode::Insert, "Insert mode goes on after the undo");
    feed(&mut e, "!<esc>u");
    assert_eq!(text(&e), "one two", "the typing after the undo is a step of its own");
    feed(&mut e, "v");
    sc(&mut e, "d-z");
    assert_eq!(text(&e), "one", "Visual mode undoes, not lowercases");
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn cmd_c_x_v_copy_cut_and_paste() {
    let mut e = ed("one\ntwo");
    let out = sc(&mut e, "d-c");
    assert!(out.contains(&Action::Copy("one\n".into())), "with no selection, the line: {out:?}");
    feed(&mut e, "j");
    let out = sc(&mut e, "d-v");
    assert!(out.iter().any(|a| matches!(a, Action::Paste { before: true, .. })), "{out:?}");
    feed(&mut e, "gg");
    sc(&mut e, "d-x");
    assert_eq!(text(&e), "two");
    feed(&mut e, "vl");
    let out = sc(&mut e, "d-c");
    assert!(out.contains(&Action::Copy("tw".into())), "{out:?}");
    assert_eq!(e.mode, Mode::Normal);
    feed(&mut e, "A x");
    let out = sc(&mut e, "d-v");
    assert_eq!(out, vec![Action::InsertClipboard]);
    let out = sc(&mut e, "d-c");
    assert!(out.contains(&Action::Copy("two x\n".into())));
    assert_eq!(e.mode, Mode::Insert);
}

#[test]
fn cmd_a_selects_every_line() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "i");
    sc(&mut e, "d-a");
    assert_eq!(e.mode, Mode::Visual(VisualKind::Line));
    assert_eq!(e.doc().view.anchor.map(|p| p.line), Some(0));
    assert_eq!(cur(&e).0, 2);
}

#[test]
fn cmd_line_commands_work_in_normal_visual_and_insert_mode() {
    let mut e = ed("fn a() {}\nfn b() {}\n");
    sc(&mut e, "d-/");
    assert_eq!(text(&e), "// fn a() {}\nfn b() {}\n");
    feed(&mut e, "A x");
    sc(&mut e, "d-]");
    assert_eq!(e.mode, Mode::Insert);
    feed(&mut e, "y<esc>");
    assert!(text(&e).starts_with("    // fn a() {} xy\n"), "{:?}", text(&e));
    sc(&mut e, "d-s-k");
    assert_eq!(text(&e), "fn b() {}\n");
}

#[test]
fn cmd_enter_opens_a_line() {
    let mut e = ed("a\nb");
    sc(&mut e, "d-cr");
    assert_eq!(e.mode, Mode::Insert);
    assert_eq!(text(&e), "a\n\nb");
    feed(&mut e, "x");
    sc(&mut e, "d-s-cr");
    feed(&mut e, "y<esc>");
    assert_eq!(text(&e), "a\ny\nx\nb");
}

#[test]
fn cmd_app_shortcuts_return_their_actions() {
    let mut e = ed("a");
    assert_eq!(sc(&mut e, "d-s"), vec![Action::Save(None)]);
    assert_eq!(sc(&mut e, "d-p"), vec![Action::FindFile]);
    assert_eq!(sc(&mut e, "d-b"), vec![Action::ToggleTree]);
    let mut out = Vec::new();
    assert!(!system::shortcut(&mut e, parse_key("d-j"), &mut out), "other keys go on to Vim");
}

#[test]
fn alt_arrows_move_and_copy_lines() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "<a-down>");
    assert_eq!(text(&e), "b\na\nc");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "<s-a-down>");
    assert_eq!(text(&e), "b\na\na\nc");
    assert_eq!(cur(&e), (2, 0), "the cursor goes with the copy");
    feed(&mut e, "<s-a-up>");
    assert_eq!(text(&e), "b\na\na\na\nc");
    assert_eq!(cur(&e), (2, 0));
    feed(&mut e, "uuu");
    assert_eq!(text(&e), "a\nb\nc");
    feed(&mut e, "Vj<s-a-down>");
    assert_eq!(text(&e), "a\nb\na\nb\nc");
    assert_eq!(e.mode, Mode::Normal);
}
