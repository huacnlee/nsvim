//! Tests for the keys and commands the key audit found broken: registers `1`-`9`, `-`, `.`, `:`
//! and `/`, `:set` with `?` and `!`, `ignorecase` and `smartcase`, `:e` with unsaved changes,
//! `:b N` and `:b name`, the `;` range separator, Visual block on the last line, `Ctrl r +`, and
//! the classic keymap's `Ctrl Q` and `:s///c` prompt.

use std::path::PathBuf;

use super::tests::{at, cur, ed, feed, text};
use super::*;
use crate::buffer::Buffer;
use crate::editor::{Action, Doc, Keymap, Mode};

#[test]
fn deletes_fill_the_numbered_and_small_delete_registers() {
    let mut e = ed("one\ntwo\nthree\nfour\n");
    feed(&mut e, "dd");
    feed(&mut e, "dd");
    assert_eq!(e.reg(Some('1')).unwrap().text, "two\n", "the newest line delete is 1");
    assert_eq!(e.reg(Some('2')).unwrap().text, "one\n", "the one before moves to 2");
    feed(&mut e, "x");
    assert_eq!(e.reg(Some('-')).unwrap().text, "t", "a delete inside a line goes to -");
    assert_eq!(e.reg(Some('1')).unwrap().text, "two\n", "and leaves 1 alone");
    feed(&mut e, "G\"2p");
    assert!(text(&e).ends_with("four\none\n"), "{:?}", text(&e));
    feed(&mut e, "yy");
    assert_eq!(e.reg(Some('1')).unwrap().text, "two\n", "a yank never shifts 1-9");
    assert!(e.reg(Some('9')).is_none(), "an unused numbered register is empty");
}

#[test]
fn the_dot_colon_and_slash_registers_hold_the_last_insert_command_and_search() {
    let mut e = ed("alpha\nbeta\n");
    feed(&mut e, "ihello <bs><esc>");
    assert_eq!(e.reg(Some('.')).unwrap().text, "hello");
    feed(&mut e, ":2<cr>");
    assert_eq!(e.reg(Some(':')).unwrap().text, "2");
    feed(&mut e, "/beta<cr>");
    assert_eq!(e.reg(Some('/')).unwrap().text, "beta");
    feed(&mut e, "o<c-r>.<esc>");
    assert_eq!(e.buf().line(2), "hello", "Ctrl r . types the last insert");
}

#[test]
fn ctrl_r_plus_asks_the_app_for_the_clipboard() {
    let mut e = ed("x\n");
    let acts = feed(&mut e, "i<c-r>+");
    assert!(acts.contains(&Action::InsertClipboard), "{acts:?}");
    assert_eq!(e.mode, Mode::Insert, "still typing");
    type_text(&mut e, "clip");
    feed(&mut e, "<esc>");
    assert_eq!(text(&e), "clipx\n");
    feed(&mut e, "u");
    assert_eq!(text(&e), "x\n", "the pasted text undoes with the insert");
}

#[test]
fn set_flips_and_shows_core_options_and_hands_the_others_to_the_app() {
    let mut e = ed("x\n");
    feed(&mut e, ":set wrap!<cr>");
    assert!(e.opts.wrap);
    feed(&mut e, ":set wrap!<cr>");
    assert!(!e.opts.wrap);
    feed(&mut e, ":set ts?<cr>");
    assert_eq!(e.msg.as_ref().unwrap().text, "  ts=4");
    let acts = feed(&mut e, ":set nu?<cr>");
    assert!(acts.contains(&Action::SetOption("number".into(), "?".into())), "{acts:?}");
    let acts = feed(&mut e, ":set cul!<cr>");
    assert!(acts.contains(&Action::SetOption("cursorline".into(), "!".into())), "{acts:?}");
    feed(&mut e, ":set ts!<cr>");
    assert!(e.msg.as_ref().unwrap().error, "a number cannot flip");
    feed(&mut e, ":set sts=2<cr>");
    assert_eq!(e.opts.tab_width, 2);
}

#[test]
fn ignorecase_and_smartcase_decide_how_search_treats_case() {
    let mut e = ed("Foo\nfoo\n");
    feed(&mut e, "/foo<cr>");
    assert_eq!(cur(&e), (1, 0), "smartcase: a lowercase pattern ignores case, and the next match is line 2");
    at(&mut e, 1, 0);
    feed(&mut e, ":set noignorecase<cr>/Foo<cr>");
    assert_eq!(cur(&e), (0, 0));
    at(&mut e, 0, 0);
    feed(&mut e, ":set noic<cr>/foo<cr>");
    assert_eq!(cur(&e), (1, 0), "case counts");
    at(&mut e, 1, 0);
    feed(&mut e, ":set ic noscs<cr>/FOO<cr>");
    assert_eq!(cur(&e), (0, 0), "ignorecase without smartcase ignores an uppercase letter too");
    feed(&mut e, ":set hls!<cr>");
    assert!(!e.vim.hlsearch);
}

#[test]
fn e_without_bang_keeps_unsaved_changes() {
    let mut e = ed("x\n");
    feed(&mut e, "dd");
    let acts = feed(&mut e, ":e<cr>");
    assert!(!acts.contains(&Action::Reload), "{acts:?}");
    assert!(e.msg.as_ref().unwrap().text.starts_with("E37"));
    let acts = feed(&mut e, ":e!<cr>");
    assert!(acts.contains(&Action::Reload));
}

#[test]
fn b_takes_a_buffer_number_or_part_of_a_name() {
    let mut e = ed("x\n");
    e.doc_mut().buf.path = Some(PathBuf::from("/p/src/main.rs"));
    for name in ["/p/src/lib.rs", "/p/tests/lib_test.rs"] {
        let mut b = Buffer::from_str("y\n");
        b.path = Some(PathBuf::from(name));
        e.docs.push(Doc::new(b));
    }
    assert!(feed(&mut e, ":b 2<cr>").contains(&Action::Open(PathBuf::from("/p/src/lib.rs"))));
    assert!(feed(&mut e, ":b main<cr>").contains(&Action::Open(PathBuf::from("/p/src/main.rs"))));
    assert!(feed(&mut e, ":b lib.rs<cr>").contains(&Action::Open(PathBuf::from("/p/src/lib.rs"))), "a whole name wins over a part");
    feed(&mut e, ":b lib<cr>");
    assert!(e.msg.as_ref().unwrap().text.starts_with("E93"));
    feed(&mut e, ":b nothing<cr>");
    assert!(e.msg.as_ref().unwrap().text.starts_with("E94"));
    feed(&mut e, ":b 9<cr>");
    assert!(e.msg.as_ref().unwrap().text.starts_with("E86"));
    assert!(feed(&mut e, ":b<cr>").contains(&Action::PickBuffer), "with no argument, the picker");
}

#[test]
fn a_semicolon_range_counts_from_its_first_line() {
    let src = "a\nb\nc\nd\ne\nf\n";
    let mut e = ed(src);
    feed(&mut e, ":3;+1d<cr>");
    assert_eq!(text(&e), "a\nb\ne\nf\n", "lines 3 and 4");
    let mut e = ed(src);
    feed(&mut e, ":3,+1d<cr>");
    assert_eq!(text(&e), "a\nd\ne\nf\n", "a comma counts from the cursor: lines 2 and 3");
}

#[test]
fn block_change_reaches_an_empty_last_line() {
    let mut e = ed("ab\ncd");
    feed(&mut e, "<c-v>jC!<esc>");
    assert_eq!(text(&e), "!\n!");
    let mut e = ed("ab\ncd");
    feed(&mut e, "<c-v>j$c-<esc>");
    assert_eq!(text(&e), "-\n-");
}

#[test]
fn classic_ctrl_q_quits_and_the_substitute_prompt_takes_its_keys() {
    let mut e = ed("a a\n");
    e.set_keymap(Keymap::Classic);
    assert!(feed(&mut e, "<c-q>").contains(&Action::QuitAll { force: false }));
    let acts = run_ex(&mut e, "s/a/b/gc");
    assert!(acts.is_empty(), "{acts:?}");
    assert!(e.vim.confirm.is_some(), "the prompt is up");
    feed(&mut e, "yn");
    assert_eq!(text(&e), "b a\n", "y replaced the first, n skipped the second");
    assert!(e.vim.confirm.is_none());
    assert_eq!(e.mode, Mode::Insert, "back to typing");
}
