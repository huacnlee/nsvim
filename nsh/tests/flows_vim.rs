//! The app driven as a user drives it: keys through NSH's focus and key capture, the real key
//! routing and the real workers. The Neovim keymap: editing, undo, registers, tree-sitter
//! objects and the command line.

mod common;

use common::{SRC, rust_flow};
use editor::editor::{Keymap, Mode};

#[test]
fn the_view_measures_rows_and_columns() {
    let (_dir, f) = rust_flow("neovim", SRC);
    let (rows, cw) = f.read(|a| (a.ed.rows, a.m.cw));
    assert!(rows > 20, "rows come from the text area's layout: {rows}");
    assert!(cw > 5.0 && cw < 12.0, "the probe measures a char: {cw}");
}

#[test]
fn dd_then_u_deletes_and_restores_a_line() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("jdd");
    assert!(!f.text().contains("let a"));
    assert!(f.read(|a| a.ed.buf().is_dirty()));
    f.keys("u");
    assert_eq!(f.text(), SRC);
}

#[test]
fn insert_mode_types_and_escape_returns_to_normal() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("Ohi\x1b");
    assert_eq!(f.read(|a| a.ed.mode), Mode::Normal);
    assert!(f.text().starts_with("hi\nfn main"));
}

#[test]
fn yank_and_paste_go_through_the_clipboard_path() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("jyyp");
    f.settle("the paste lands", |a| a.ed.buf().text().matches("let a = 1;").count() == 2);
    assert_eq!(f.text().matches("let a = 1;").count(), 2, "{}", f.text());
}

#[test]
fn quitting_a_modified_buffer_is_refused() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("x:q\n");
    assert!(f.msg().is_some_and(|(m, error)| error && m.starts_with("E37")), "{:?}", f.msg());
    assert_eq!(f.read(|a| a.ed.docs.len()), 1);
}

#[test]
fn set_keymap_switches_to_classic_and_back() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":set keymap=classic\n");
    assert_eq!(f.read(|a| a.ed.keymap), Keymap::Classic);
    assert_eq!(f.read(|a| a.ed.mode), Mode::Insert);
    f.keys("x");
    assert!(f.text().starts_with("xfn"), "classic types at once: {}", f.text());
}

#[test]
fn tree_sitter_objects_and_motions_work_through_the_app() {
    let src = "/// Adds.\nfn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n\nfn main() {\n    let x = add(1, 2);\n}\n";
    let (_dir, mut f) = rust_flow("neovim", src);
    // From the doc comment, `]f` lands on `fn add`, a second `]f` on `fn main`, and `daf`
    // deletes it.
    f.keys("]f");
    assert_eq!(f.read(|a| a.ed.cursor().line), 1, "{:?}", f.msg());
    f.keys("]f");
    assert_eq!(f.read(|a| a.ed.cursor().line), 5, "{:?}", f.msg());
    f.keys("daf");
    assert!(!f.text().contains("fn main"), "{}", f.text());
    assert!(f.text().contains("fn add"));
    f.keys("u");
    // `dif` on the first function empties its body.
    f.keys("ggjjdif");
    assert!(f.text().contains("fn add(a: u32, b: u32) -> u32 {\n}"), "{}", f.text());
    // `cia` changes one parameter.
    f.keys("u");
    f.keys("gg0jf(lcianum: u64\x1b");
    assert!(f.text().contains("fn add(num: u64, b: u32)"), "{}", f.text());
}

#[test]
fn r_overtypes_and_the_status_line_says_replace() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("R");
    assert!(f.read(|a| editor::vim::replacing(&a.ed.vim)));
    f.ui.wait_for("text:Replace").unwrap();
    f.keys("xy\x1b");
    assert!(f.text().starts_with("xy main"), "{}", f.text());
    assert!(!f.read(|a| editor::vim::replacing(&a.ed.vim)));
    f.ui.wait_for("text:Normal").unwrap();
}

#[test]
fn set_shows_and_flips_app_options() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":set nu?\n");
    assert!(f.said("number=true"), "{:?}", f.msg());
    f.keys(":set nu!\n");
    assert_eq!(f.read(|a| a.cfg.editor.numbers.clone()), "off");
    f.keys(":set cul!\n");
    assert!(!f.read(|a| a.cfg.editor.cursorline));
    for line in [":set hlsearch\n", ":set ic\n", ":set noscs\n", ":set noligatures\n", ":set sts=2\n"] {
        f.keys(line);
        assert!(!f.msg().is_some_and(|(_, error)| error), "{line}: {:?}", f.msg());
    }
    assert!(!f.read(|a| a.cfg.font.ligatures));
    assert_eq!(f.read(|a| a.cfg.indent.width), 2);
    f.keys(":set nope?\n");
    assert!(f.msg().is_some_and(|(m, error)| error && m.contains("E518")), "{:?}", f.msg());
}

#[test]
fn ctrl_r_plus_types_the_system_clipboard() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.ui.set_clipboard(Some("from the clipboard ".into()));
    f.keys("i");
    f.press("ctrl+r");
    f.keys("+");
    f.settle("the clipboard text lands", |a| a.ed.buf().text().starts_with("from the clipboard "));
    f.keys("\x1b");
    assert!(f.text().starts_with("from the clipboard fn main()"), "{}", f.text());
}

#[test]
fn colon_w_writes_the_file_and_the_status_line_shows_it_clean() {
    let (dir, mut f) = rust_flow("neovim", SRC);
    f.keys("jx");
    f.ui.wait_for("text:main.rs [+]").unwrap();
    f.keys(":w\n");
    f.settle("the write", |a| !a.ed.buf().is_dirty());
    f.ui.wait_for("text:main.rs").unwrap();
    let on_disk = std::fs::read_to_string(dir.path().join("main.rs")).unwrap();
    assert_eq!(on_disk, SRC.replacen("    let a", "   let a", 1));
}

#[test]
fn search_finds_the_next_match_and_n_repeats_it() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("/let\n");
    assert_eq!(f.read(|a| (a.ed.cursor().line, a.ed.cursor().col)), (1, 4));
    f.keys("n");
    assert_eq!(f.read(|a| a.ed.cursor().line), 2);
    f.keys("*");
    assert_eq!(f.read(|a| a.ed.cursor().line), 1, "* searches the word under the cursor and wraps");
}

#[test]
fn a_named_register_keeps_its_text_after_other_yanks() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys("j\"ayyjyy");
    f.keys("G\"ap");
    f.settle("the paste", |a| a.ed.buf().text().ends_with("}\n    let a = 1;\n"));
}

#[test]
fn substitute_over_the_file_is_one_undo_step() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":%s/let/var/g\n");
    assert_eq!(f.text().matches("var ").count(), 2, "{}", f.text());
    f.keys("u");
    assert_eq!(f.text(), SRC);
}
