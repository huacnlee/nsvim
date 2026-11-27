//! The find bar, driven through the real app: open it, type, count, step, replace, replace all
//! with its undo, a regular expression, and Esc. It needs no Vim syntax.

mod common;

use common::{Flow, rust_flow};
use editor::buffer::Pos;
use nsvim_native::app::App;
use nsvim_native::exports::find_of;

const SRC: &str = "let total = 1;\nlet count = total + 2;\nprint(total, count);\n";

fn count(a: &App) -> String {
    find_of(a).count
}

/// Waits until the bar's count is `want`: the keys before it reach the app first.
fn wait_count(f: &mut Flow, want: &'static str) {
    f.settle(want, move |a| count(a) == want);
}

#[test]
fn ctrl_f_finds_counts_and_steps_both_ways() {
    let (_dir, mut f) = rust_flow("classic", SRC);
    f.press("ctrl+f");
    assert!(f.check(|a| a.find.as_ref().is_some_and(|b| !b.replace && b.keys)), "Ctrl F opens the find bar");
    assert!(f.ui.screen().matches("id:find-bar").unwrap(), "the bar shows: {:?}", f.ui.screen().texts());
    // The word under the cursor is the query: `let`.
    assert_eq!(f.read(|a| a.find.as_ref().unwrap().query.clone()), "let");
    f.press("ctrl+u");
    f.keys("total");
    wait_count(&mut f, "1 of 3");
    assert!(f.read(|a| a.ed.vim.hlsearch && !a.ed.search.pattern.is_empty()), "the matches are highlighted");
    // The classic keymap selects the match.
    assert_eq!(f.read(|a| (a.ed.doc().view.anchor, a.ed.cursor())), (Some(Pos::new(0, 4)), Pos::new(0, 9)));
    assert!(f.ui.screen().matches("text:1 of 3").unwrap(), "{:?}", f.ui.screen().texts());
    f.press("enter");
    wait_count(&mut f, "2 of 3");
    f.press("enter");
    f.press("enter");
    wait_count(&mut f, "1 of 3");
    f.press("shift+enter");
    wait_count(&mut f, "3 of 3");
    // The typed keys went to the bar, not to the text.
    assert_eq!(f.text(), SRC);
    f.keys("zz");
    wait_count(&mut f, "No results");
    assert!(f.read(|a| find_of(a).none));
    f.press("escape");
    assert!(f.check(|a| a.find.is_none() && !a.ed.vim.hlsearch), "Esc closes the bar and its highlight");
    assert!(!f.ui.screen().matches("id:find-bar").unwrap());
    f.keys("x");
    assert_ne!(f.text(), SRC, "the keys go back to the text");
}

#[test]
fn match_case_and_whole_word_toggle_from_the_bar() {
    let (_dir, mut f) = rust_flow("classic", "Cat cat catalog\n");
    f.press("ctrl+f");
    f.press("ctrl+u");
    f.keys("cat");
    wait_count(&mut f, "1 of 3");
    f.ui.press("label:Match case").unwrap();
    f.settle("match case counts two", |a| find_of(a).count.ends_with("of 2") || find_of(a).count == "2 results");
    f.ui.press("label:Whole word").unwrap();
    f.settle("whole word counts one", |a| find_of(a).count.contains('1') && !find_of(a).count.contains('2'));
    f.press("alt+c");
    assert!(f.read(|a| !a.find.as_ref().unwrap().opts.case), "Alt C flips match case");
}

#[test]
fn replace_one_then_all_and_undo_all_in_one_step() {
    let (_dir, mut f) = rust_flow("classic", SRC);
    f.press("ctrl+alt+f");
    assert!(f.check(|a| a.find.as_ref().is_some_and(|b| b.replace)), "Ctrl Alt F opens the replace field");
    assert!(f.ui.screen().matches("id:find-with").unwrap());
    // The keys go to the replace field: Tab gives them to the find field.
    f.press("tab");
    f.press("ctrl+u");
    f.keys("total");
    f.press("tab");
    f.keys("sum");
    assert_eq!(f.read(|a| a.find.as_ref().unwrap().with.clone()), "sum");
    f.press("enter");
    assert_eq!(f.text(), SRC.replacen("total", "sum", 1), "Enter in the replace field replaces one");
    wait_count(&mut f, "1 of 2");
    f.ui.press("label:Replace all").unwrap();
    f.settle("replace all replaces the rest", |a| a.ed.buf().text() == SRC.replace("total", "sum"));
    assert!(f.read(|a| a.ed.buf().is_dirty()));
    f.press("escape");
    f.press("ctrl+z");
    assert_eq!(f.text(), SRC.replacen("total", "sum", 1), "one undo takes back every replace");
    f.press("ctrl+z");
    assert_eq!(f.text(), SRC);
}

#[test]
fn a_regex_with_groups_replaces_each_match() {
    let (_dir, mut f) = rust_flow("classic", "a=1\nb=22\n");
    f.press("ctrl+alt+f");
    f.press("tab");
    f.press("ctrl+u");
    f.press("alt+r");
    f.keys("(\\w)=(\\d+)");
    wait_count(&mut f, "1 of 2");
    f.press("tab");
    f.keys("$2:$1");
    f.press("ctrl+enter");
    f.settle("Ctrl Enter replaces all", |a| a.ed.buf().text() == "1:a\n22:b\n");
    wait_count(&mut f, "No results");
}

#[test]
fn the_palette_and_the_edit_menu_open_the_bar_in_the_neovim_keymap() {
    let (_dir, mut f) = rust_flow("vim", SRC);
    f.update(|a, t| a.app_menu_pick("cmd:find-in-file", t));
    assert!(f.check(|a| a.find.as_ref().is_some_and(|b| !b.replace)), "Edit › Find… opens the bar");
    f.press("escape");
    f.update(|a, t| a.run_command("replace-in-file", t));
    assert!(f.check(|a| a.find.as_ref().is_some_and(|b| b.replace)), "the palette opens Replace");
    f.press("escape");
    // `/` keeps the Vim search line.
    f.keys("/");
    assert!(f.check(|a| a.find.is_none() && a.ed.cmd.is_some()), "/ opens the search line");
}
