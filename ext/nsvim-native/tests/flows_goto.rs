//! The Go menu through the real app: Go to Line, Go to Symbol in File, and Back and Forward on
//! the jump list. Their keys, as docs/keymaps.md lists them.

mod common;

use common::{Flow, flow, keymap_config, rust_flow, scratch, write};
use editor::buffer::Pos;
use editor::editor::Keymap;
use nsvim_native::app::{App, Overlay, PickKind};

fn picking(a: &App, kind: PickKind) -> bool {
    matches!(&a.overlay, Overlay::Pick(p) if p.kind == kind)
}

fn cursor(f: &Flow) -> Pos {
    f.read(|a| a.ed.cursor())
}

const LINES: &str = "fn one() {}\n\nfn two() {}\n\n    fn three() {}\nlast line\n";

#[test]
fn go_to_line_moves_as_you_type_enter_stays_and_esc_goes_back() {
    let (_dir, mut f) = rust_flow("classic", LINES);
    f.settle("the classic keymap", |a| a.ed.keymap == Keymap::Classic);
    // `goto::tests` checks the keys. Here the palette's command opens it, as the Go menu does.
    f.update(|a, t| a.run_command("line", t));
    f.settle("Go to Line opens", |a| picking(a, PickKind::Line));
    f.keys("5");
    assert_eq!(cursor(&f), Pos::new(4, 4), "the cursor shows line 5 at its first non-blank as it is typed");
    f.keys("x");
    assert_eq!(f.read(|a| if let Overlay::Pick(p) = &a.overlay { p.query.clone() } else { String::new() }), "5", "letters do not go in");
    f.press("escape");
    assert_eq!(cursor(&f), Pos::new(0, 0), "Esc goes back");
    assert!(f.check(|a| matches!(a.overlay, Overlay::None)));
    f.update(|a, t| a.run_command("line", t));
    f.keys("3:4\n");
    assert_eq!(cursor(&f), Pos::new(2, 3), "line:col goes to the column");
    assert!(f.check(|a| matches!(a.overlay, Overlay::None)), "Enter closes it");
    f.press("ctrl+alt+-");
    assert_eq!(cursor(&f), Pos::new(0, 0), "Back comes to the place before the jump");
    f.press("ctrl+shift+-");
    assert_eq!(cursor(&f), Pos::new(2, 3), "Forward goes to the jump again");
}

#[test]
fn go_to_line_and_the_jumps_run_from_the_leader_in_the_neovim_keymap() {
    let (_dir, mut f) = rust_flow("neovim", LINES);
    f.keys(" j");
    f.settle("SPC j opens Go to Line", |a| picking(a, PickKind::Line));
    f.keys("99\n");
    assert_eq!(cursor(&f).line, 5, "a line past the end stops at the last line");
    f.press("ctrl+o");
    assert_eq!(cursor(&f), Pos::new(0, 0), "Ctrl-o comes back, as Back does");
    f.keys(" ]");
    assert_eq!(cursor(&f).line, 5, "SPC ] goes forward");
    f.keys(" [");
    assert_eq!(cursor(&f), Pos::new(0, 0), "SPC [ goes back");
}

#[test]
fn go_to_symbol_lists_the_tree_symbols_without_a_server_and_goes_there() {
    let src = "struct Point { x: i32 }\nimpl Point {\n    fn norm(&self) -> i32 { self.x }\n}\nfn main() {}\n";
    let dir = scratch();
    // A file in a language with no server on CI: the list comes from the tree.
    let file = write(dir.path(), "shapes.rs", src);
    let mut f = flow(&file, Some(&keymap_config("classic")));
    f.settle("the classic keymap", |a| a.ed.keymap == Keymap::Classic);
    f.press("ctrl+shift+o");
    f.settle("Ctrl Shift O opens the symbols", |a| picking(a, PickKind::Symbols));
    let names = f.read(|a| if let Overlay::Pick(p) = &a.overlay { p.items.clone() } else { Vec::new() });
    assert_eq!(names, ["Point", "impl Point", "norm", "main"]);
    f.keys("norm");
    f.settle(
        "the query ranks norm first",
        |a| matches!(&a.overlay, Overlay::Pick(p) if p.matches.first().is_some_and(|m| p.items[m.index] == "norm")),
    );
    f.press("enter");
    assert_eq!(cursor(&f), Pos::new(2, 7), "Enter goes to the symbol's name");
    f.press("ctrl+alt+-");
    assert_eq!(cursor(&f), Pos::new(0, 0), "Back comes to the place before it");
}

#[test]
fn space_l_s_opens_the_symbols_of_the_file() {
    let (_dir, mut f) = rust_flow("neovim", LINES);
    f.keys(" ls");
    f.settle("SPC l s opens the symbols", |a| picking(a, PickKind::Symbols));
    let names = f.read(|a| if let Overlay::Pick(p) = &a.overlay { p.items.clone() } else { Vec::new() });
    assert!(!names.is_empty(), "the tree lists the file's symbols");
}
