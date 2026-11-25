//! Tests for Replace mode, sentences, tags, operator-pending search, `gn`, `U`, Visual dot
//! repeat, `:s///c`, marks, the jump list across files, `gf` and the tree-sitter keys.

use std::path::PathBuf;

use super::tests::{at, cur, ed, feed, text};
use super::*;
use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Doc, Mode};

#[test]
fn replace_mode_overtypes_and_backspace_restores() {
    let mut e = ed("xyz");
    feed(&mut e, "Rabc");
    assert!(replacing(&e.vim));
    assert_eq!(e.mode, Mode::Insert);
    assert_eq!(text(&e), "abc");
    feed(&mut e, "<bs><bs>");
    assert_eq!(text(&e), "ayz", "Backspace brings back what each char covered");
    feed(&mut e, "<esc>");
    assert!(!replacing(&e.vim));
    assert_eq!(cur(&e), (0, 0));
    feed(&mut e, "u");
    assert_eq!(text(&e), "xyz", "one undo step");
}

#[test]
fn replace_mode_appends_past_the_end_and_splits_lines() {
    let mut e = ed("ab");
    feed(&mut e, "Rxyz<bs><esc>");
    assert_eq!(text(&e), "xy");
    let mut e = ed("abcd");
    feed(&mut e, "lR<cr><esc>");
    assert_eq!(text(&e), "a\nbcd", "Enter splits the line without eating a char");
}

#[test]
fn replace_mode_counts_and_dot() {
    let mut e = ed("abcdefgh");
    feed(&mut e, "3Rxy<esc>");
    assert_eq!(text(&e), "xyxyxygh");
    let mut e = ed("aaaa\naaaa");
    feed(&mut e, "Rxy<esc>j0.");
    assert_eq!(text(&e), "xyaa\nxyaa");
}

#[test]
fn sentence_motions_and_objects() {
    let s = "Hello there. How are you? Fine.";
    let mut e = ed(s);
    feed(&mut e, ")");
    assert_eq!(cur(&e), (0, 13));
    feed(&mut e, ")");
    assert_eq!(cur(&e), (0, 26));
    feed(&mut e, "(");
    assert_eq!(cur(&e), (0, 13));
    feed(&mut e, "dis");
    assert_eq!(text(&e), "Hello there.  Fine.");
    let mut e = ed(s);
    at(&mut e, 0, 15);
    feed(&mut e, "das");
    assert_eq!(text(&e), "Hello there. Fine.");
    let mut e = ed(s);
    at(&mut e, 0, 28);
    feed(&mut e, "das");
    assert_eq!(text(&e), "Hello there. How are you?", "the last sentence takes the white space before it");
}

#[test]
fn sentences_stop_at_empty_lines() {
    let mut e = ed("One. Two\n\nThree.");
    feed(&mut e, "))");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, ")");
    assert_eq!(cur(&e), (2, 0));
}

#[test]
fn tag_objects_nest_and_count() {
    let s = "<div><p>hi <b>x</b></p></div>";
    let mut e = ed(s);
    at(&mut e, 0, 8);
    feed(&mut e, "dit");
    assert_eq!(text(&e), "<div><p></p></div>");
    let mut e = ed(s);
    at(&mut e, 0, 14);
    feed(&mut e, "dat");
    assert_eq!(text(&e), "<div><p>hi </p></div>");
    let mut e = ed(s);
    at(&mut e, 0, 14);
    feed(&mut e, "3dit");
    assert_eq!(text(&e), "<div></div>");
    let mut e = ed("<ul>\n  <li>a</li>\n  <br/>\n</ul>");
    at(&mut e, 1, 6);
    feed(&mut e, "2dit");
    assert_eq!(text(&e), "<ul></ul>", "self-closing tags don't count");
}

#[test]
fn operators_take_a_search_as_their_motion() {
    let mut e = ed("one two three four");
    feed(&mut e, "d/thr<cr>");
    assert_eq!(text(&e), "three four");
    assert_eq!(e.mode, Mode::Normal);
    let mut e = ed("one two three");
    at(&mut e, 0, 8);
    feed(&mut e, "c?one<cr>X<esc>");
    assert_eq!(text(&e), "Xthree");
    let mut e = ed("one two");
    feed(&mut e, "d/<esc>w");
    assert_eq!(text(&e), "one two", "Esc drops the operator");
    assert_eq!(cur(&e), (0, 4));
    assert!(e.vim.op_search.is_none());
}

#[test]
fn y_slash_yanks_up_to_the_match() {
    let mut e = ed("abc def ghi");
    feed(&mut e, "y/gh<cr>");
    assert_eq!(e.regs.unnamed.text, "abc def ");
    assert_eq!(text(&e), "abc def ghi");
}

#[test]
fn cgn_and_dot_change_match_after_match() {
    let mut e = ed("foo bar foo bar foo");
    feed(&mut e, "/foo<cr>");
    assert_eq!(cur(&e), (0, 8));
    feed(&mut e, "cgnX<esc>");
    assert_eq!(text(&e), "foo bar X bar foo");
    feed(&mut e, ".");
    assert_eq!(text(&e), "foo bar X bar X");
    feed(&mut e, ".");
    assert_eq!(text(&e), "X bar X bar X");
}

#[test]
fn gn_selects_the_match_and_dgn_deletes_it() {
    let mut e = ed("a foo b");
    feed(&mut e, "/foo<cr>0gn");
    assert_eq!(e.mode, Mode::Visual(crate::editor::VisualKind::Char));
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 2)));
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "<esc>0dgn");
    assert_eq!(text(&e), "a  b");
}

#[test]
fn big_u_restores_the_last_changed_line() {
    let mut e = ed("hello world\nnext");
    feed(&mut e, "xx");
    assert_eq!(text(&e), "llo world\nnext");
    feed(&mut e, "U");
    assert_eq!(text(&e), "hello world\nnext");
    feed(&mut e, "u");
    assert_eq!(text(&e), "llo world\nnext", "U is a change that u undoes");
}

#[test]
fn dot_repeats_visual_commands_on_the_same_size() {
    let mut e = ed("a\nb\nc\nd\ne");
    feed(&mut e, "Vjd");
    assert_eq!(text(&e), "c\nd\ne");
    feed(&mut e, ".");
    assert_eq!(text(&e), "e");
    let mut e = ed("abcdef abcdef");
    feed(&mut e, "vlld");
    assert_eq!(text(&e), "def abcdef");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "def def");
    let mut e = ed("foo foo");
    feed(&mut e, "vecX<esc>");
    assert_eq!(text(&e), "X foo");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "X X", "a Visual change repeats with its text");
    let mut e = ed("abcd");
    feed(&mut e, "vl~ll.");
    assert_eq!(text(&e), "ABCD");
    let mut e = ed("ab\ncd\nef");
    feed(&mut e, "<c-v>jrxj.");
    assert_eq!(text(&e), "xb\nxd\nxf");
}

#[test]
fn substitute_confirm_asks_for_each_match() {
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>");
    assert!(confirming(&e.vim));
    assert!(e.msg.as_ref().is_some_and(|m| m.text.starts_with("replace with b (y/n/a/q/l")));
    assert_eq!(e.doc().view.anchor, Some(Pos::new(0, 0)), "the match is selected");
    feed(&mut e, "yny");
    assert!(!confirming(&e.vim));
    assert_eq!(text(&e), "b a\nb");
    assert_eq!(e.msg.as_ref().map(|m| m.text.as_str()), Some("2 substitutions on 2 lines"));
    assert_eq!(e.mode, Mode::Normal);
    feed(&mut e, "u");
    assert_eq!(text(&e), "a a\na", "the whole session is one undo step");
}

#[test]
fn substitute_confirm_all_quit_and_last() {
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>a");
    assert_eq!(text(&e), "b b\nb");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>yq");
    assert_eq!(text(&e), "b a\na");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/gc<cr>nl");
    assert_eq!(text(&e), "a b\na");
    let mut e = ed("a a\na");
    feed(&mut e, ":%s/a/b/c<cr>yy");
    assert_eq!(text(&e), "b a\nb", "without g, one match a line");
}

#[test]
fn marks_follow_edits_above_them() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "jjma");
    feed(&mut e, "ggOnew<esc>");
    feed(&mut e, "gg'a");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, ":marks<cr>");
    assert!(e.msg.as_ref().is_some_and(|m| m.text.contains("a 4:0")));
    feed(&mut e, ":delmarks a<cr>'a");
    assert!(e.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E20")));
}

fn two_files() -> crate::editor::Editor {
    let mut e = ed("one\ntwo\nthree");
    e.docs[0].buf.path = Some(PathBuf::from("/tmp/project/a.rs"));
    let mut b = Buffer::from_str("x\ny");
    b.path = Some(PathBuf::from("/tmp/project/b.rs"));
    e.docs.push(Doc::new(b));
    e
}

#[test]
fn uppercase_marks_open_their_file() {
    let mut e = two_files();
    feed(&mut e, "jmA");
    e.active = 1;
    assert_eq!(feed(&mut e, "'A"), vec![Action::OpenAt(PathBuf::from("/tmp/project/a.rs"), Pos::new(1, 0))]);
    e.active = 0;
    feed(&mut e, "G'A");
    assert_eq!(cur(&e), (1, 0), "in its own file it is a plain jump");
}

#[test]
fn ctrl_o_returns_to_another_file() {
    let mut e = two_files();
    feed(&mut e, "G");
    e.active = 1;
    assert_eq!(feed(&mut e, "<c-o>"), vec![Action::OpenAt(PathBuf::from("/tmp/project/a.rs"), Pos::new(0, 0))]);
}

#[test]
fn gf_opens_the_file_under_the_cursor() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("x.txt"), "hi").unwrap();
    let mut e = ed("see x.txt here");
    e.docs[0].buf.path = Some(dir.path().join("main.rs"));
    at(&mut e, 0, 5);
    assert_eq!(feed(&mut e, "gf"), vec![Action::Open(dir.path().join("x.txt"))]);
    at(&mut e, 0, 11);
    assert!(feed(&mut e, "gf").is_empty());
    assert!(e.msg.as_ref().is_some_and(|m| m.text.starts_with("E447")));
}

#[test]
fn rename_and_code_action_commands() {
    let mut e = ed("x");
    assert_eq!(feed(&mut e, ":Rename new_name<cr>"), vec![Action::Rename("new_name".into())]);
    let acts = feed(&mut e, ":Rename<cr>");
    assert!(matches!(acts.as_slice(), [Action::Error(m)] if m.starts_with("E471")));
    assert_eq!(feed(&mut e, ":CodeAction<cr>"), vec![Action::CodeActions]);
}

#[test]
fn code_objects_and_motions_say_why_they_found_nothing() {
    // The buffer has no path, so no grammar covers it, with or without tree-sitter.
    let mut e = ed("fn main() {\n    let x = 1;\n}");
    at(&mut e, 1, 4);
    feed(&mut e, "daf");
    assert_eq!(text(&e), "fn main() {\n    let x = 1;\n}");
    assert!(e.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E: no")));
    e.msg = None;
    feed(&mut e, "]f");
    assert_eq!(cur(&e), (1, 4));
    assert!(e.msg.as_ref().is_some_and(|m| m.error));
    feed(&mut e, "vic");
    assert_eq!(e.mode, Mode::Visual(crate::editor::VisualKind::Char), "Visual stays as it was");
    assert_eq!(e.doc().view.anchor, Some(Pos::new(1, 4)));
    feed(&mut e, "<esc>[a]F[C");
    assert_eq!(cur(&e), (1, 4));
}

#[test]
fn vim_keys_drop_extra_cursors() {
    let mut e = ed("ab\ncd");
    e.docs[0].extra.push(crate::editor::Cursor { pos: Pos::new(1, 0), anchor: None });
    feed(&mut e, "l");
    assert!(e.doc().extra.is_empty());
}

// The tests below check keys that docs/keymaps.md lists.

#[test]
fn g_forms_columns_and_line_motions() {
    let mut e = ed("  foo bar  \nbaz qux\n\nend");
    let steps = [
        ("g_", (0, 8)),
        ("g0", (0, 0)),
        ("g$", (0, 10)),
        ("4|", (0, 3)),
        ("+", (1, 0)),
        ("-", (0, 2)),
        ("2_", (1, 0)),
        ("<cr>", (2, 0)),
        ("gk", (1, 0)),
        ("wgE", (1, 2)),
        ("<bs>", (1, 1)),
        ("<c-n>", (2, 0)),
        ("<c-p>", (1, 1)),
        ("<c-h>", (1, 0)),
        ("<c-j>", (2, 0)),
        ("<c-m>", (3, 0)),
        ("gj", (3, 0)),
        ("<home>", (3, 0)),
        ("<end>", (3, 2)),
    ];
    for (keys, want) in steps {
        feed(&mut e, keys);
        assert_eq!(cur(&e), want, "after {keys}");
    }
}

#[test]
fn star_forms_screen_lines_percent_and_scrolling() {
    let mut e = ed("foo foobar foo");
    for (keys, col) in [("*", 11), ("g#", 4), ("0g*", 4), ("$#", 0)] {
        feed(&mut e, keys);
        assert_eq!(cur(&e), (0, col), "after {keys}");
    }
    let src: String = (0..50).map(|i| format!("l{i}\n")).collect();
    let mut e = ed(&src);
    e.opts.scrolloff = 0;
    for (keys, line) in [("L", 19), ("M", 9), ("3H", 2), ("3L", 17), ("50%", 24)] {
        feed(&mut e, keys);
        assert_eq!(cur(&e).0, line, "after {keys}");
    }
    let top = |e: &crate::editor::Editor| e.doc().view.top;
    assert_eq!(top(&e), 5);
    for (keys, want) in [("<c-e>", 6), ("<c-y>", 5), ("zt", 24), ("zb", 5), ("zz", 14), ("z<cr>", 24), ("z-", 5), ("z.", 14)] {
        feed(&mut e, keys);
        assert_eq!(top(&e), want, "after {keys}");
    }
    feed(&mut e, "10zt");
    assert_eq!((cur(&e).0, top(&e)), (9, 9));
    feed(&mut e, "<c-f>");
    assert_eq!((cur(&e).0, top(&e)), (27, 27));
    feed(&mut e, "<c-b>");
    assert_eq!(top(&e), 9);
    feed(&mut e, "<pgdn>");
    assert_eq!(top(&e), 27);
    feed(&mut e, "<pgup>");
    assert_eq!(top(&e), 9);
}

#[test]
fn registers_macros_marks_and_substitute_repeats() {
    // `r` with Enter splits the line.
    let mut e = ed("ab cd");
    feed(&mut e, "lr<cr>");
    assert_eq!(text(&e), "a\n cd");
    // `&` repeats the last `:s` without its flags, `:&&` with them.
    let mut e = ed("a a\na a\na a");
    run_ex(&mut e, "s/a/b/g");
    feed(&mut e, "j:&&<cr>");
    assert_eq!(text(&e), "b b\nb b\na a");
    feed(&mut e, "j&");
    assert_eq!(text(&e), "b b\nb b\nb a");
    // `"0` keeps the last yank after a delete.
    let mut e = ed("one\ntwo");
    feed(&mut e, "yyjdd\"0p");
    assert_eq!(text(&e), "one\none\n");
    // `qA` appends to macro `a`.
    let mut e = ed("a\nb\nc");
    feed(&mut e, "qaA1<esc>q");
    feed(&mut e, "qAA2<esc>q");
    feed(&mut e, "j@a");
    assert_eq!(text(&e), "a12\nb12\nc");
    // `@b` runs the text of register `b` as keys.
    let mut e = ed("x abc");
    feed(&mut e, "\"byiww@b");
    assert_eq!(text(&e), "x bc");
    // `` `a `` goes to the mark's column, `'a` to its line's first non-blank.
    let mut e = ed("  abc\nxyz");
    feed(&mut e, "llllmaj`a");
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "j'a");
    assert_eq!(cur(&e), (0, 2));
    // Delete is `x`, and `.` repeats it.
    let mut e = ed("abc");
    feed(&mut e, "<del>.");
    assert_eq!(text(&e), "c");
}

#[test]
fn app_keys_and_prefixes_in_normal_mode() {
    let mut e = ed("abc\ndef\nghi");
    assert_eq!(feed(&mut e, "gD"), vec![Action::GotoDefinition]);
    assert_eq!(feed(&mut e, "gT"), vec![Action::PrevBuffer]);
    assert_eq!(feed(&mut e, "K"), vec![Action::Hover]);
    assert_eq!(feed(&mut e, "ZZ"), vec![Action::Quit { force: false }]);
    let acts = feed(&mut e, "xZZ");
    assert!(acts.contains(&Action::Save(None)) && acts.contains(&Action::Quit { force: false }), "{acts:?}");
    feed(&mut e, "<c-q>");
    assert_eq!(e.mode, Mode::Visual(crate::editor::VisualKind::Block));
    feed(&mut e, "<esc>3:");
    assert_eq!(e.cmd.as_ref().map(|c| c.text.as_str()), Some(".,.+2"));
    feed(&mut e, "<esc>Gggk<tab>");
    assert_eq!(cur(&e).0, 2, "Tab goes forward in the jump list, as Ctrl-i does");
}

#[test]
fn insert_mode_ctrl_keys() {
    let mut e = ed("ab");
    let steps = [
        ("\"aylA<c-r>a<esc>", "aba"),
        ("A<c-t><esc>", "    aba"),
        ("A<c-d><esc>", "aba"),
        ("A<c-h><esc>", "ab"),
        ("A<c-j>x<esc>", "ab\nx\n"),
        ("A<c-m>y<esc>", "ab\nx\ny\n"),
        ("ggi<end>Z<home>Y<del><esc>", "YbZ\nx\ny\n"),
        ("0i<tab><esc>", "    YbZ\nx\ny\n"),
        ("ji<right><left><down><up><c-c>", "    YbZ\nx\ny\n"),
        ("i<c-[>", "    YbZ\nx\ny\n"),
    ];
    for (keys, want) in steps {
        feed(&mut e, keys);
        assert_eq!(text(&e), want, "after {keys}");
        assert_eq!(e.mode, Mode::Normal, "after {keys}");
    }
}

#[test]
fn tree_sitter_motions_for_ends_classes_and_parameters() {
    let src = "struct A {\n    x: u32,\n}\n\nfn f(a: u32, b: u32) {\n    g(a, b);\n}\n";
    let mut e = ed(src);
    e.doc_mut().buf.path = Some(PathBuf::from("/tmp/editor-doc-test.rs"));
    let steps = [
        ("]f", (4, 0)),
        ("]F", (6, 0)),
        ("[c", (0, 0)),
        ("]C", (2, 0)),
        ("gg]a", (4, 5)),
        ("]a", (4, 13)),
        ("]A", (4, 18)),
        ("[a", (4, 13)),
        ("[A", (4, 10)),
        ("G[f", (4, 0)),
        ("gg]F", (6, 0)),
    ];
    for (keys, want) in steps {
        feed(&mut e, keys);
        assert_eq!(cur(&e), want, "after {keys}: {:?}", e.msg);
    }
    feed(&mut e, "]c");
    assert!(e.msg.as_ref().is_some_and(|m| m.text == "E: no next class"));
}
