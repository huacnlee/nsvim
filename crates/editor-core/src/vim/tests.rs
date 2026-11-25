//! Normal and Insert mode tests, and the helpers the other test files share.

use std::path::PathBuf;

use super::*;
use crate::buffer::{Buffer, Pos};
use crate::editor::{Action, Doc, Editor, KeyCode, KeyInput, Keymap, Mode};

/// An editor with one document holding `text`, the cursor at the start.
pub(crate) fn ed(text: &str) -> Editor {
    let mut e = Editor::new(PathBuf::from("/tmp/project"));
    e.docs.push(Doc::new(Buffer::from_str(text)));
    e.active = 0;
    e.rows = 20;
    e.opts.auto_close = false;
    e
}

fn parse_key(name: &str) -> KeyInput {
    let (mut ctrl, mut shift, mut alt) = (false, false, false);
    let mut rest = name;
    loop {
        if let Some(r) = rest.strip_prefix("c-") {
            ctrl = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("s-") {
            shift = true;
            rest = r;
        } else if let Some(r) = rest.strip_prefix("a-") {
            alt = true;
            rest = r;
        } else {
            break;
        }
    }
    let code = match rest {
        "esc" => KeyCode::Esc,
        "cr" | "enter" => KeyCode::Enter,
        "bs" => KeyCode::Backspace,
        "tab" => KeyCode::Tab,
        "del" => KeyCode::Delete,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pgup" => KeyCode::PageUp,
        "pgdn" => KeyCode::PageDown,
        "lt" => KeyCode::Char('<'),
        "space" => KeyCode::Char(' '),
        "f12" => KeyCode::F(12),
        s => KeyCode::Char(s.chars().next().unwrap()),
    };
    KeyInput { code, ctrl, alt, shift, cmd: ctrl }
}

/// Keys in Vim notation: plain chars, and `<esc>`, `<cr>`, `<c-r>`, `<s-left>`, `<a-up>` ...
pub(crate) fn keys(s: &str) -> Vec<KeyInput> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '<' && it.peek().is_some_and(|n| n.is_ascii_alphabetic()) {
            let name: String = it.by_ref().take_while(|c| *c != '>').collect();
            out.push(parse_key(&name));
        } else {
            out.push(KeyInput::ch(c));
        }
    }
    out
}

/// Types `s` and answers clipboard pastes from the unnamed register, as the app would when the
/// system clipboard holds what the editor copied last.
pub(crate) fn feed(e: &mut Editor, s: &str) -> Vec<Action> {
    let mut all = Vec::new();
    for k in keys(s) {
        let acts = match e.keymap {
            Keymap::Vim => handle_key(e, k),
            Keymap::Classic => crate::classic::handle_key(e, k),
        };
        for a in acts {
            if let Action::Paste { before, count } = a {
                let r = e.regs.unnamed.clone();
                paste_text(e, &r.text, r.linewise, before, count);
            }
            all.push(a);
        }
    }
    all
}

pub(crate) fn text(e: &Editor) -> String {
    e.buf().text()
}

pub(crate) fn cur(e: &Editor) -> (usize, usize) {
    let p = e.cursor();
    (p.line, p.col)
}

pub(crate) fn at(e: &mut Editor, line: usize, col: usize) {
    e.doc_mut().view.cursor = Pos::new(line, col);
    e.doc_mut().view.want_col = col;
}

#[test]
fn w_e_b_follow_vim_word_classes() {
    let mut e = ed("foo.bar baz\n  qux");
    feed(&mut e, "w");
    assert_eq!(cur(&e), (0, 3), "w stops at punctuation");
    feed(&mut e, "w");
    assert_eq!(cur(&e), (0, 4));
    feed(&mut e, "w");
    assert_eq!(cur(&e), (0, 8));
    feed(&mut e, "w");
    assert_eq!(cur(&e), (1, 2), "w crosses the line end to the first word");
    feed(&mut e, "b");
    assert_eq!(cur(&e), (0, 8));
    feed(&mut e, "0e");
    assert_eq!(cur(&e), (0, 2));
    feed(&mut e, "e");
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, "0W");
    assert_eq!(cur(&e), (0, 8), "W skips the punctuation inside a WORD");
    feed(&mut e, "ge");
    assert_eq!(cur(&e), (0, 6));
}

#[test]
fn w_stops_on_an_empty_line() {
    let mut e = ed("a\n\nb");
    feed(&mut e, "w");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "w");
    assert_eq!(cur(&e), (2, 0));
    feed(&mut e, "b");
    assert_eq!(cur(&e), (1, 0));
}

#[test]
fn cw_changes_to_the_word_end_but_dw_takes_the_blank() {
    let mut e = ed("foo bar baz");
    feed(&mut e, "cwX<esc>");
    assert_eq!(text(&e), "X bar baz");
    let mut e = ed("foo bar baz");
    feed(&mut e, "dw");
    assert_eq!(text(&e), "bar baz");
    let mut e = ed("foo bar baz");
    feed(&mut e, "ceY<esc>");
    assert_eq!(text(&e), "Y bar baz");
    let mut e = ed("a foo bar");
    at(&mut e, 0, 2);
    feed(&mut e, "c2wZ<esc>");
    assert_eq!(text(&e), "a Z");
}

#[test]
fn dw_on_the_last_word_of_a_line_does_not_join() {
    let mut e = ed("one two\nthree");
    at(&mut e, 0, 4);
    feed(&mut e, "dw");
    assert_eq!(text(&e), "one \nthree");
    let mut e = ed("x\n\ny");
    at(&mut e, 1, 0);
    feed(&mut e, "dw");
    assert_eq!(text(&e), "x\ny", "dw on an empty line deletes it");
}

#[test]
fn counts_multiply_and_repeat() {
    let mut e = ed("a b c d e f");
    feed(&mut e, "3dw");
    assert_eq!(text(&e), "d e f");
    let mut e = ed("a b c d e f");
    feed(&mut e, "d3w");
    assert_eq!(text(&e), "d e f");
    let mut e = ed("a b c d e f");
    feed(&mut e, "2d2w");
    assert_eq!(text(&e), "e f");
    let mut e = ed("1\n2\n3\n4\n5\n");
    feed(&mut e, "3j");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, "gg2dd");
    assert_eq!(text(&e), "3\n4\n5\n");
}

#[test]
fn dd_p_and_big_p_put_whole_lines() {
    let mut e = ed("one\ntwo\nthree\n");
    feed(&mut e, "ddp");
    assert_eq!(text(&e), "two\none\nthree\n");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "ggyyP");
    assert_eq!(text(&e), "two\ntwo\none\nthree\n");
    let mut e = ed("a\nb");
    feed(&mut e, "jddP");
    assert_eq!(text(&e), "b\na");
    let mut e = ed("a\nb");
    feed(&mut e, "yyjp");
    assert_eq!(text(&e), "a\nb\na\n", "a put below a last line without a newline ends in one");
}

#[test]
fn yank_copies_to_the_clipboard() {
    let mut e = ed("hello world");
    let acts = feed(&mut e, "yiw");
    assert!(acts.contains(&Action::Copy("hello".into())));
    assert_eq!(e.regs.unnamed.text, "hello");
    let acts = feed(&mut e, "\"ayw");
    assert!(!acts.iter().any(|a| matches!(a, Action::Copy(_))), "a named register stays out of the clipboard");
    assert_eq!(e.regs.named[&'a'].text, "hello ");
}

#[test]
fn p_asks_the_app_for_the_clipboard() {
    let mut e = ed("abc");
    feed(&mut e, "yl");
    let acts = handle_key(&mut e, KeyInput::ch('p'));
    assert_eq!(acts, vec![Action::Paste { before: false, count: 1 }]);
    assert_eq!(text(&e), "abc", "nothing changes until the clipboard answers");
    paste_text(&mut e, "XY", false, false, 2);
    assert_eq!(text(&e), "aXYXYbc");
    assert_eq!(cur(&e), (0, 4));
}

#[test]
fn registers_append_with_uppercase() {
    let mut e = ed("one two");
    feed(&mut e, "\"ayw");
    feed(&mut e, "w\"Ayw");
    assert_eq!(e.regs.named[&'a'].text, "one two");
    feed(&mut e, "0\"ap");
    assert_eq!(text(&e), "oone twone two");
}

#[test]
fn ciw_ci_paren_and_da_quote() {
    let mut e = ed("let foo = bar;");
    at(&mut e, 0, 5);
    feed(&mut e, "ciwbaz<esc>");
    assert_eq!(text(&e), "let baz = bar;");
    let mut e = ed("f(a, (b), c)");
    at(&mut e, 0, 3);
    feed(&mut e, "ci(x<esc>");
    assert_eq!(text(&e), "f(x)");
    let mut e = ed("f(a, (b), c)");
    at(&mut e, 0, 6);
    feed(&mut e, "di(");
    assert_eq!(text(&e), "f(a, (), c)");
    let mut e = ed("say \"hi there\" now");
    at(&mut e, 0, 7);
    feed(&mut e, "da\"");
    assert_eq!(text(&e), "say now");
    let mut e = ed("say \"hi there\" now");
    feed(&mut e, "ci\"yo<esc>");
    assert_eq!(text(&e), "say \"yo\" now", "the first quoted string after the cursor");
}

#[test]
fn inner_braces_across_lines_keep_the_brace_lines() {
    let mut e = ed("fn f() {\n    a;\n    b;\n}\n");
    at(&mut e, 1, 4);
    feed(&mut e, "di{");
    assert_eq!(text(&e), "fn f() {\n}\n");
}

#[test]
fn dot_repeats_a_change_with_its_text() {
    let mut e = ed("foo bar baz");
    feed(&mut e, "ciwxx<esc>");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "xx xx baz");
    feed(&mut e, "w.");
    assert_eq!(text(&e), "xx xx xx");
    let mut e = ed("a b c d e");
    feed(&mut e, "dw..");
    assert_eq!(text(&e), "d e");
    let mut e = ed("1\n2\n3\n4\n5\n6\n");
    feed(&mut e, "dd2.");
    assert_eq!(text(&e), "4\n5\n6\n", "a count replaces the old one");
}

#[test]
fn an_insert_session_undoes_as_one_step() {
    let mut e = ed("abc");
    feed(&mut e, "ifoo<cr>bar<esc>");
    assert_eq!(text(&e), "foo\nbarabc");
    feed(&mut e, "u");
    assert_eq!(text(&e), "abc");
    feed(&mut e, "<c-r>");
    assert_eq!(text(&e), "foo\nbarabc");
    let mut e = ed("abc");
    feed(&mut e, "3ix<esc>");
    assert_eq!(text(&e), "xxxabc");
    feed(&mut e, "u");
    assert_eq!(text(&e), "abc");
}

#[test]
fn insert_mode_keeps_and_adds_indent() {
    let mut e = ed("    if x {");
    feed(&mut e, "A<cr>y<esc>");
    assert_eq!(text(&e), "    if x {\n        y");
    let mut e = ed("  a");
    feed(&mut e, "ob<esc>");
    assert_eq!(text(&e), "  a\n  b");
    feed(&mut e, "o<esc>");
    assert_eq!(text(&e), "  a\n  b\n\n", "an unused auto-indent goes, the empty line stays");
    assert_eq!(cur(&e), (2, 0));
}

#[test]
fn insert_backspace_and_auto_close() {
    let mut e = ed("");
    e.opts.auto_close = true;
    feed(&mut e, "ifoo(x<esc>");
    assert_eq!(text(&e), "foo(x)");
    let mut e = ed("");
    e.opts.auto_close = true;
    feed(&mut e, "i(<bs><esc>");
    assert_eq!(text(&e), "");
    let mut e = ed("        x");
    feed(&mut e, "I<bs><esc>");
    assert_eq!(text(&e), "    x", "backspace steps back one indent level");
    let mut e = ed("ab\ncd");
    feed(&mut e, "ji<bs><esc>");
    assert_eq!(text(&e), "abcd");
}

#[test]
fn enter_at_the_end_of_a_file_makes_a_real_line() {
    let mut e = ed("a");
    feed(&mut e, "A<cr>b<esc>");
    assert_eq!(text(&e), "a\nb\n");
    assert_eq!(cur(&e), (1, 0));
    let mut e = ed("");
    feed(&mut e, "i<cr><cr>x<esc>");
    assert_eq!(text(&e), "\n\nx\n");
}

#[test]
fn ctrl_w_and_ctrl_u_in_insert() {
    let mut e = ed("");
    feed(&mut e, "ifoo bar<c-w>baz<esc>");
    assert_eq!(text(&e), "foo baz");
    let mut e = ed("  ");
    feed(&mut e, "Afoo bar<c-u>x<esc>");
    assert_eq!(text(&e), "  x");
}

#[test]
fn o_and_big_o_with_counts() {
    let mut e = ed("a");
    feed(&mut e, "2ob<esc>");
    assert_eq!(text(&e), "a\nb\nb\n");
    let mut e = ed("a");
    feed(&mut e, "Oz<esc>");
    assert_eq!(text(&e), "z\na");
}

#[test]
fn macros_record_and_replay() {
    let mut e = ed("1\n2\n3\n4\n");
    feed(&mut e, "qaA!<esc>jq");
    assert_eq!(e.vim.recording(), None);
    feed(&mut e, "2@a");
    assert_eq!(text(&e), "1!\n2!\n3!\n4\n");
    feed(&mut e, "@@");
    assert_eq!(text(&e), "1!\n2!\n3!\n4!\n");
}

#[test]
fn j_joins_with_one_space() {
    let mut e = ed("a\n    b\nc");
    feed(&mut e, "J");
    assert_eq!(text(&e), "a b\nc");
    assert_eq!(cur(&e), (0, 1));
    let mut e = ed("a\nb\nc\nd");
    feed(&mut e, "3J");
    assert_eq!(text(&e), "a b c\nd");
    let mut e = ed("a\n  b");
    feed(&mut e, "gJ");
    assert_eq!(text(&e), "a  b");
}

#[test]
fn percent_jumps_between_brackets() {
    let mut e = ed("if (a[1]) {\n}");
    feed(&mut e, "%");
    assert_eq!(cur(&e), (0, 8));
    feed(&mut e, "%");
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, "$%");
    assert_eq!(cur(&e), (1, 0));
    let mut e = ed("x (y) z");
    feed(&mut e, "d%");
    assert_eq!(text(&e), " z", "d% runs from the cursor to the bracket's match");
    let mut e = ed("x (y) z");
    feed(&mut e, "f(d%");
    assert_eq!(text(&e), "x  z");
}

#[test]
fn f_and_t_repeat_with_semicolon_and_comma() {
    let mut e = ed("a,b,c,d");
    feed(&mut e, "f,");
    assert_eq!(cur(&e), (0, 1));
    feed(&mut e, ";");
    assert_eq!(cur(&e), (0, 3));
    feed(&mut e, ",");
    assert_eq!(cur(&e), (0, 1));
    feed(&mut e, "0t,");
    assert_eq!(cur(&e), (0, 0));
    feed(&mut e, ";");
    assert_eq!(cur(&e), (0, 2), "a repeated till moves on to the next char");
    let mut e = ed("a,b,c");
    feed(&mut e, "dt,");
    assert_eq!(text(&e), ",b,c");
    feed(&mut e, "$dF,");
    assert_eq!(text(&e), ",bc");
}

#[test]
fn paragraph_motions_and_d_brace() {
    let mut e = ed("a\nb\n\nc\nd\n\ne");
    feed(&mut e, "}");
    assert_eq!(cur(&e), (2, 0));
    feed(&mut e, "}");
    assert_eq!(cur(&e), (5, 0));
    feed(&mut e, "{");
    assert_eq!(cur(&e), (2, 0));
    let mut e = ed("a\nb\n\nc");
    feed(&mut e, "d}");
    assert_eq!(text(&e), "\nc", "d}} from a line start is linewise up to the blank line");
}

#[test]
fn x_s_big_d_big_c_tilde_and_r() {
    let mut e = ed("hello");
    feed(&mut e, "x");
    assert_eq!(text(&e), "ello");
    feed(&mut e, "$x");
    assert_eq!(text(&e), "ell");
    assert_eq!(cur(&e), (0, 2));
    let mut e = ed("hello world");
    at(&mut e, 0, 5);
    feed(&mut e, "D");
    assert_eq!(text(&e), "hello");
    let mut e = ed("hello");
    feed(&mut e, "3~");
    assert_eq!(text(&e), "HELlo");
    feed(&mut e, "0rx");
    assert_eq!(text(&e), "xELlo");
    feed(&mut e, "2rz");
    assert_eq!(text(&e), "zzLlo");
    let mut e = ed("");
    feed(&mut e, "sabc<esc>");
    assert_eq!(text(&e), "abc", "s on an empty line still inserts");
}

#[test]
fn case_and_comment_operators() {
    let mut e = ed("hello world");
    feed(&mut e, "gUiw");
    assert_eq!(text(&e), "HELLO world");
    feed(&mut e, "guu");
    assert_eq!(text(&e), "hello world");
    let mut e = ed("a\n  b\nc");
    feed(&mut e, "gcj");
    assert_eq!(text(&e), "// a\n//   b\nc");
    feed(&mut e, "gcc");
    assert_eq!(text(&e), "a\n//   b\nc");
}

#[test]
fn shift_lines_with_counts() {
    let mut e = ed("a\nb\nc");
    feed(&mut e, "2>>");
    assert_eq!(text(&e), "    a\n    b\nc");
    feed(&mut e, "<lt>j");
    assert_eq!(text(&e), "a\nb\nc");
}

#[test]
fn ctrl_a_and_ctrl_x_change_numbers() {
    let mut e = ed("x = 41;");
    feed(&mut e, "<c-a>");
    assert_eq!(text(&e), "x = 42;");
    assert_eq!(cur(&e), (0, 5));
    feed(&mut e, "50<c-x>");
    assert_eq!(text(&e), "x = -8;");
}

#[test]
fn search_star_and_n() {
    let mut e = ed("foo bar\nbaz foo\nfoo");
    feed(&mut e, "/foo<cr>");
    assert_eq!(cur(&e), (1, 4));
    feed(&mut e, "n");
    assert_eq!(cur(&e), (2, 0));
    feed(&mut e, "n");
    assert_eq!(cur(&e), (0, 0));
    assert!(e.msg.as_ref().is_some_and(|m| m.text.contains("BOTTOM")));
    feed(&mut e, "N");
    assert_eq!(cur(&e), (2, 0));
    let mut e = ed("ab abc ab");
    feed(&mut e, "*");
    assert_eq!(cur(&e), (0, 7), "* matches whole words");
    feed(&mut e, "/zzz<cr>");
    assert!(e.msg.as_ref().is_some_and(|m| m.error && m.text.starts_with("E486")));
}

#[test]
fn marks_and_jumps() {
    let mut e = ed("a\nb\nc\nd");
    feed(&mut e, "jmaG");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, "'a");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "''");
    assert_eq!(cur(&e), (3, 0));
    feed(&mut e, "d'a");
    assert_eq!(text(&e), "a");
}

#[test]
fn leader_and_app_keys_return_actions() {
    let mut e = ed("x");
    assert_eq!(feed(&mut e, " "), vec![Action::LeaderMenu]);
    assert_eq!(feed(&mut e, "gd"), vec![Action::GotoDefinition]);
    assert_eq!(feed(&mut e, "gt"), vec![Action::NextBuffer]);
    assert_eq!(feed(&mut e, "ZQ"), vec![Action::Quit { force: true }]);
    e.vim.leader = '\\';
    assert_eq!(feed(&mut e, "\\"), vec![Action::LeaderMenu]);
}

#[test]
fn ctrl_w_moves_to_the_tree_and_l_stays_in_the_text() {
    let mut e = ed("x");
    for keys in ["<c-w>h", "<c-w>w", "<c-w>p", "<c-w><c-w>", "<c-w><c-h>"] {
        assert_eq!(feed(&mut e, keys), vec![Action::FocusTree], "{keys}");
    }
    assert_eq!(feed(&mut e, "<c-w>l"), vec![], "the text is the right window");
    assert_eq!(text(&e), "x");
    assert_eq!(cur(&e), (0, 0));
}

#[test]
fn showcmd_lists_pending_keys() {
    let mut e = ed("x");
    feed(&mut e, "\"a2d");
    assert_eq!(showcmd(&e.vim), "\"a2d");
    feed(&mut e, "<esc>");
    assert_eq!(showcmd(&e.vim), "");
}

#[test]
fn ctrl_o_runs_one_normal_command() {
    let mut e = ed("abc def");
    feed(&mut e, "i<c-o>wX<esc>");
    assert_eq!(text(&e), "abc Xdef");
}

#[test]
fn word_range_for_double_click() {
    let b = Buffer::from_str("foo.bar baz");
    assert_eq!(word_range_at(&b, Pos::new(0, 5)), (4, 7));
    assert_eq!(word_range_at(&b, Pos::new(0, 3)), (3, 4));
}

#[test]
fn unicode_columns_are_chars() {
    let mut e = ed("héllo wörld");
    feed(&mut e, "wx");
    assert_eq!(text(&e), "héllo örld");
    feed(&mut e, "0ea!<esc>");
    assert_eq!(text(&e), "héllo! örld");
}

#[test]
fn empty_buffer_does_not_panic() {
    let mut e = ed("");
    feed(&mut e, "xdddwcwfoo<esc>uu<c-r>pP>>J~");
    let mut e = ed("");
    feed(&mut e, "vjd");
    feed(&mut e, "<c-v>jIx<esc>");
    assert_eq!(text(&e), "x");
    let mut e = Editor::new(PathBuf::from("/"));
    assert!(feed(&mut e, "ddx").is_empty());
    assert_eq!(feed(&mut e, ":q<cr>"), vec![Action::Quit { force: false }]);
    assert_eq!(e.mode, Mode::Normal);
}

#[test]
fn ctrl_d_and_ctrl_u_move_half_a_screen() {
    let lines: String = (0..100).map(|i| format!("{i}\n")).collect();
    let mut e = ed(&lines);
    e.rows = 20;
    feed(&mut e, "<c-d>");
    assert_eq!(cur(&e).0, 10);
    feed(&mut e, "<c-u>");
    assert_eq!(cur(&e).0, 0);
    feed(&mut e, "50Gzt");
    assert_eq!(e.doc().view.top, 49 - e.opts.scrolloff);
}

#[test]
fn cc_keeps_the_indent_and_dot_repeats_append() {
    let mut e = ed("    old\nb");
    feed(&mut e, "ccnew<esc>");
    assert_eq!(text(&e), "    new\nb");
    let mut e = ed("a\nb\nc");
    feed(&mut e, "A;<esc>j.j.");
    assert_eq!(text(&e), "a;\nb;\nc;");
}

#[test]
fn black_hole_and_neovim_y() {
    let mut e = ed("keep this");
    feed(&mut e, "yiw");
    let acts = feed(&mut e, "w\"_dw");
    assert!(acts.is_empty());
    assert_eq!(e.regs.unnamed.text, "keep");
    let mut e = ed("hello world");
    at(&mut e, 0, 6);
    feed(&mut e, "Y");
    assert_eq!(e.regs.unnamed.text, "world", "Y yanks to the line end, as in Neovim");
}

#[test]
fn undo_puts_the_cursor_back() {
    let mut e = ed("one\ntwo\nthree");
    feed(&mut e, "jjdd");
    assert_eq!(cur(&e), (1, 0));
    feed(&mut e, "u");
    assert_eq!(text(&e), "one\ntwo\nthree");
    assert_eq!(cur(&e), (2, 0));
    let acts = feed(&mut e, "uu");
    assert!(acts.is_empty());
    assert_eq!(e.msg.as_ref().map(|m| m.text.as_str()), Some("Already at oldest change"));
}

#[test]
fn gi_returns_to_the_last_insert() {
    let mut e = ed("abc\ndef");
    feed(&mut e, "jAx<esc>gg");
    feed(&mut e, "giy<esc>");
    assert_eq!(text(&e), "abc\ndefxy");
}
