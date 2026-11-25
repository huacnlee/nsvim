//! Panels and popups through the real app: the leader menu, the pickers, the tree, Settings, a
//! proposal, the completion list and first-launch setup. Their keys, as docs/keymaps.md lists
//! them.

mod common;

use common::{Flow, SRC, flow, flow_files, keymap_config, rust_flow, scratch, write};
use editor::buffer::Pos;
use editor::editor::Mode;
use nsh_app_editor::app::settings::Section;
use nsh_app_editor::app::{App, Overlay, PickKind, Screen};

fn picking(a: &App, kind: PickKind) -> bool {
    matches!(&a.overlay, Overlay::Pick(p) if p.kind == kind)
}

/// The open picker's query and row, or `("closed", 0)`.
fn pick(f: &Flow) -> (String, usize) {
    f.read(|a| if let Overlay::Pick(p) = &a.overlay { (p.query.clone(), p.sel) } else { ("closed".into(), 0) })
}

/// A project with `Cargo.toml`, `src/lib.rs` and `src/main.rs`, with `Cargo.toml` open. The
/// tree's keyboard row starts on `src`, its first row.
fn tree_project() -> (tempfile::TempDir, Flow) {
    let dir = scratch();
    write(dir.path(), "src/main.rs", SRC);
    write(dir.path(), "src/lib.rs", "pub fn lib() {}\n");
    let cargo = write(dir.path(), "Cargo.toml", "[package]\nname = \"x\"\n");
    let mut f = flow(&cargo, Some(&keymap_config("neovim")));
    f.settle("the project index", |a| a.files.len() == 3);
    f.update(|a, _| a.tree.sel = Some("src".into()));
    (dir, f)
}

#[test]
fn space_opens_the_leader_menu_and_b_lists_buffers() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(" ");
    assert!(f.check(|a| matches!(a.overlay, Overlay::Leader(None))));
    f.keys("b");
    assert!(f.check(|a| picking(a, PickKind::Buffers)), "SPC b opens the buffer list");
    f.keys("\x1b");
    assert!(f.check(|a| matches!(a.overlay, Overlay::None)));
}

#[test]
fn the_leader_menu_and_the_buffer_list_show_their_rows() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(" ");
    f.ui.wait_for("text:/find file/").unwrap();
    f.ui.wait_for("text:/\\+git/").unwrap();
    f.keys("b");
    f.ui.wait_for("text:Buffers").unwrap();
    f.ui.wait_for("text:/main\\.rs/").unwrap();
    f.keys("\x1b");
    f.ui.wait_for_gone("text:Buffers", std::time::Duration::from_secs(2)).unwrap();
}

#[test]
fn colon_settings_opens_the_tab_and_w_writes_the_config() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":Settings\n");
    assert!(f.read(|a| a.on_settings), "{:?}", f.msg());
    // The first row is the theme; `l` moves it from dark to system.
    f.keys("l");
    assert_eq!(f.read(|a| a.cfg.theme.mode.clone()), "system");
    assert!(f.read(|a| a.cfg != a.disk_cfg));
    f.keys(":w\n");
    assert!(f.read(|a| a.cfg == a.disk_cfg), "`:w` in Settings writes config.toml");
    f.keys(":q\n");
    assert!(!f.read(|a| a.on_settings));
}

#[test]
fn the_settings_tab_shows_its_sections_and_rows() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":Settings\n");
    f.ui.wait_for("text:appearance").unwrap();
    f.ui.wait_for("text:/keys & mouse/").unwrap();
    f.keys("J");
    f.ui.wait_for("text:/JetBrains Mono/").unwrap();
}

#[test]
fn space_e_focuses_the_tree_and_space_e_again_hides_it() {
    let (_dir, mut f) = tree_project();
    f.keys(" e");
    assert!(f.read(|a| a.tree.show && a.tree.focus));
    f.keys("l");
    assert!(f.read(|a| a.tree.open.contains("src")), "l opens the folder");
    f.keys("j");
    assert_eq!(f.read(|a| a.tree.sel.clone()).as_deref(), Some("src/lib.rs"));
    f.keys(" e");
    assert!(f.read(|a| !a.tree.show && !a.tree.focus));
}

#[test]
fn the_tree_shows_folders_and_files() {
    let (_dir, mut f) = tree_project();
    f.ui.wait_for("id:tree >> text:src").unwrap();
    f.ui.wait_for("id:tree >> text:Cargo.toml").unwrap();
    f.keys(" el");
    f.ui.wait_for("id:tree >> text:lib.rs").unwrap();
}

/// The pickers: Ctrl n, p, j, k, Tab and Shift Tab move, Ctrl w and Ctrl u edit the query,
/// Enter opens the row.
#[test]
fn picker_keys_move_edit_and_pick() {
    let dir = scratch();
    let files: Vec<_> = ["test-main.rs", "test-two.rs", "test-three.rs"].iter().map(|n| write(dir.path(), n, "x\n")).collect();
    let paths: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();
    let mut f = flow_files(&paths, Some(&keymap_config("neovim")));
    f.settle("three buffers", |a| a.ed.docs.len() == 3);
    let order = f.read(|a| a.ed.docs.iter().map(|d| d.buf.path.clone().unwrap()).collect::<Vec<_>>());
    assert_eq!(order, files, "the buffers open in order");
    f.update(|a, _| a.ed.active = 0);
    f.keys(" b");
    for (k, sel) in [("ctrl+n", 1), ("ctrl+p", 0), ("ctrl+j", 1), ("ctrl+k", 0)] {
        f.press(k);
        assert_eq!(pick(&f).1, sel, "{k}");
    }
    for k in ["tab", "tab", "shift+tab"] {
        f.press(k);
    }
    assert_eq!(pick(&f).1, 1, "Tab and Shift+Tab move");
    f.keys("test-t");
    f.press("ctrl+w");
    assert_eq!(pick(&f).0, "test-");
    f.press("ctrl+u");
    f.keys("three\n");
    assert_eq!((pick(&f).0, f.read(|a| a.ed.active)), ("closed".to_string(), 2));
}

/// The tree: o opens a folder, G and g go to the ends, h goes to the parent and closes it, q
/// leaves.
#[test]
fn tree_keys_move_open_and_close() {
    let (_dir, mut f) = tree_project();
    f.keys(" e");
    for (keys, want) in [("o", "src"), ("G", "Cargo.toml"), ("g", "src"), ("jh", "src")] {
        f.keys(keys);
        assert_eq!(f.read(|a| a.tree.sel.clone()).as_deref(), Some(want), "tree {keys}");
    }
    f.keys("hq");
    assert!(f.read(|a| !a.tree.open.contains("src") && !a.tree.focus));
}

/// Settings: J, K, Tab, Shift+Tab and 1 to 5 pick a section. j and k move. h, l, Enter and
/// Space change.
#[test]
fn settings_keys_pick_sections_and_change_rows() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let section = |f: &Flow| f.read(|a| a.settings.as_ref().map(|s| s.section));
    f.keys(":Settings\nJ");
    assert_eq!(section(&f), Some(Section::Font));
    f.keys("K");
    f.press("tab");
    f.press("shift+tab");
    assert_eq!(section(&f), Some(Section::Appearance));
    f.keys("3l");
    assert_eq!(f.read(|a| a.cfg.editor.numbers.clone()), "relative");
    f.keys("hj\n");
    assert_eq!(f.read(|a| (a.cfg.editor.numbers.clone(), a.cfg.editor.cursor.clone())), ("hybrid".into(), "bar".into()));
    // Space is the leader here too: it opens the menu and changes no option.
    f.keys(" ");
    assert!(f.check(|a| matches!(a.overlay, Overlay::Leader(None))));
    assert_eq!(f.read(|a| a.cfg.editor.cursor.clone()), "bar");
    f.keys("\x1b");
}

/// A proposal: Tab and ] go to the next change, [ to the one before, g to the top. Esc rejects,
/// Enter accepts.
#[test]
fn proposal_keys_move_between_changes_and_answer() {
    let src: String = (0..60).map(|i| format!("line {i}\n")).collect();
    let (_dir, mut f) = rust_flow("neovim", &src);
    let new = src.replace("line 2\n", "two\n").replace("line 50\n", "fifty\n");
    assert!(f.ask("propose", serde_json::json!({ "text": new })).ok);
    let top = |f: &Flow| f.read(|a| a.proposals[0].top);
    let first = top(&f);
    f.press("tab");
    assert!(top(&f) > first);
    f.keys("[");
    assert_eq!(top(&f), first);
    f.keys("]cg");
    assert_eq!(top(&f), 0);
    f.keys("\x1b");
    assert!(f.read(|a| a.proposals.is_empty()) && f.text() == src);
    assert!(f.ask("propose", serde_json::json!({ "text": new })).ok);
    f.keys("\n");
    assert!(f.text().contains("fifty"));
}

/// The completion list: Ctrl+N, Ctrl+P and Shift+Tab move, Tab takes an item, Ctrl+E closes
/// it.
#[test]
fn completion_keys_move_take_and_close() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    let list = |a: &mut App, _: &mut nsh_app_editor::app::fx::Tx| {
        let items =
            ["foo", "fob"].map(|l| nsh_app_editor::services::lsp::Item { label: l.into(), detail: "".into(), kind: "", insert: l.into() });
        a.completion =
            Some(nsh_app_editor::app::lsp::Completion { start: Pos::new(0, 0), items: items.to_vec(), shown: vec![0, 1], sel: 0 });
    };
    f.keys("ggOfo");
    f.update(list);
    f.press("ctrl+n");
    f.press("ctrl+p");
    f.press("shift+tab");
    f.press("tab");
    assert!(f.text().starts_with("fob\n"), "{}", f.text());
    f.update(list);
    f.press("ctrl+e");
    assert!(f.read(|a| a.completion.is_none() && a.ed.mode == Mode::Insert));
}

/// Setup: j, k, h, l and arrows move, Enter and Space pick, Tab and Shift+Tab change the step,
/// Esc goes back.
#[test]
fn setup_keys_move_pick_and_change_the_step() {
    let dir = scratch();
    let mut f = common::flow_with(nsh_app_editor::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(common::CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    let st = |f: &Flow| f.read(|a| if let Screen::Setup(s) = &a.screen { (s.step, s.focus, s.vim()) } else { (0, 0, true) });
    f.keys("jkljh");
    f.press("down");
    f.keys("k\n");
    assert_eq!(st(&f), (1, 1, false), "Enter picks the classic keymap");
    f.keys("k ");
    assert_eq!(st(&f), (1, 0, true), "Space picks the Neovim keymap");
    f.press("tab");
    assert_eq!(st(&f).0, 2);
    f.press("shift+tab");
    f.press("tab");
    f.keys("\x1b");
    assert_eq!(st(&f).0, 1);
}

#[test]
fn setup_shows_its_steps() {
    let dir = scratch();
    let mut f = common::flow_with(nsh_app_editor::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(common::CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    f.ui.wait_for("text:/Neovim/").unwrap();
    f.keys("\n");
    f.ui.wait_for("text:/classic|Neovim/").unwrap();
}

/// The look step: three cards, Light, Dark and System, with no separate Omarchy card. The
/// cards in a row have one height, whichever is picked.
#[test]
fn the_look_cards_in_a_row_have_one_height() {
    let dir = scratch();
    let mut f = common::flow_with(nsh_app_editor::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(common::CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    f.update(|a, _| a.setup_go(2));
    f.ui.wait_for("text:Pick a look").unwrap();
    assert_eq!(f.ui.screen().count("role:radio[Omarchy]").unwrap(), 0, "System covers Omarchy");
    let height = |f: &Flow, name: &str| {
        let l = f.ui.screen().layout(&format!("role:radio[{name}]")).unwrap().expect("the card is laid out");
        (l.bounds.origin.y.0, l.bounds.size.height.0)
    };
    // Pick each look in turn: the picked card has a thicker border.
    for pick in ["System", "Light", "Dark"] {
        f.ui.press(&format!("role:radio[{pick}]")).unwrap();
        let cards: Vec<_> = ["Light", "Dark", "System"].iter().map(|n| height(&f, n)).collect();
        assert!(cards.iter().all(|c| *c == cards[0]), "{pick} picked: {cards:?}");
    }
}
