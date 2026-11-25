//! Panels and popups through the real app: the leader menu, the pickers, the tree, Settings, a
//! proposal, the completion list and first-launch setup. Their keys, as docs/keymaps.md lists
//! them.

mod common;

use common::{Flow, SRC, flow, flow_files, keymap_config, rust_flow, scratch, write};
use editor::buffer::Pos;
use editor::editor::Mode;
use nsvim_native::app::settings::Section;
use nsvim_native::app::{App, Overlay, PickKind, Screen};

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
    f.ui.wait_for("text:/Find file…/").unwrap();
    f.ui.wait_for("text:/\\+Git/").unwrap();
    f.keys("b");
    f.ui.wait_for("text:Buffers").unwrap();
    f.ui.wait_for("text:/main\\.rs/").unwrap();
    f.keys("\x1b");
    f.ui.wait_for_gone("text:Buffers", std::time::Duration::from_secs(2)).unwrap();
}

#[test]
fn colon_settings_opens_it_and_a_change_is_on_disk_at_once() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":Settings\n");
    assert!(f.read(|a| a.on_settings), "{:?}", f.msg());
    // The first row is the theme. `l` moves it from dark, the last, round to system.
    f.keys("l");
    assert_eq!(f.read(|a| a.cfg.theme.mode.clone()), "system");
    let path = nsvim_native::config::config_path();
    f.settle("config.toml has the change with no :w", move |_| {
        std::fs::read_to_string(&path).is_ok_and(|t| t.contains("mode = \"system\""))
    });
    // `:` does nothing here. Esc closes Settings.
    f.keys(":");
    assert!(f.read(|a| a.on_settings && a.ed.mode != Mode::Command));
    f.keys("\x1b");
    assert!(!f.read(|a| a.on_settings));
}

#[test]
fn settings_shows_its_sections_and_rows() {
    let (_dir, mut f) = rust_flow("neovim", SRC);
    f.keys(":Settings\n");
    f.ui.wait_for("text:Appearance").unwrap();
    f.ui.wait_for("text:/Keys & mouse/").unwrap();
    f.keys("J");
    f.ui.wait_for("text:/JetBrains Mono/").unwrap();
}

#[test]
fn ctrl_w_moves_the_keyboard_between_the_tree_and_the_text() {
    let (_dir, mut f) = tree_project();
    let before = f.text();
    f.press("ctrl+w");
    f.keys("h");
    assert!(f.read(|a| a.tree.show && a.tree.focus), "Ctrl-w h gives the tree the keyboard");
    f.press("ctrl+w");
    f.keys("h");
    assert!(f.read(|a| a.tree.focus), "Ctrl-w h in the tree keeps it there");
    f.press("ctrl+w");
    f.keys("l");
    assert!(f.read(|a| a.tree.show && !a.tree.focus), "Ctrl-w l gives the text the keyboard");
    f.press("ctrl+w");
    f.press("ctrl+w");
    assert!(f.read(|a| a.tree.focus), "Ctrl-w Ctrl-w goes to the tree");
    f.press("ctrl+w");
    f.press("ctrl+w");
    assert!(f.read(|a| !a.tree.focus), "and back to the text");
    assert_eq!(f.text(), before, "no key edits the file");
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
    // Space is the leader: it changes no option, and no menu opens over Settings.
    f.keys(" ");
    assert!(f.read(|a| matches!(a.overlay, Overlay::None) && a.on_settings));
    assert_eq!(f.read(|a| a.cfg.editor.cursor.clone()), "bar");
    f.keys("\x1b");
    assert!(!f.read(|a| a.on_settings), "Esc closes Settings");
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
    let list = |a: &mut App, _: &mut nsvim_native::app::fx::Tx| {
        let items =
            ["foo", "fob"].map(|l| nsvim_native::services::lsp::Item { label: l.into(), detail: "".into(), kind: "", insert: l.into() });
        a.completion = Some(nsvim_native::app::lsp::Completion { start: Pos::new(0, 0), items: items.to_vec(), shown: vec![0, 1], sel: 0 });
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
    let mut f = common::flow_with(nsvim_native::Launch {
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
    let mut f = common::flow_with(nsvim_native::Launch {
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
    let mut f = common::flow_with(nsvim_native::Launch {
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

/// The terminal step offers `nvim`, `vim` and `vi` aliases, shows the line it adds, and drops it when
/// the switch goes off.
#[test]
fn the_terminal_step_offers_vim_and_vi_aliases() {
    // This test runs from target/debug. A development build has no terminal step.
    nsvim_native::app::setup::treat_dev_build_as_release();
    let dir = scratch();
    let mut f = common::flow_with(nsvim_native::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(common::CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    f.update(|a, _| a.setup_go(3));
    f.ui.wait_for("text:Use nsvim from the terminal").unwrap();
    f.ui.wait_for("label:Open nsvim with vim and vi").unwrap();
    // The step builds again when the probe answers. Press after it.
    f.settle("the probe answers", |a| match &a.screen {
        Screen::Setup(s) => s.exe.is_some(),
        _ => false,
    });
    let alias = |s: &nsh_testing::Screen| s.texts().iter().any(|t| t.contains("alias vim=") && !t.contains("nvim=") && t.contains(" vi="));
    assert!(alias(f.ui.screen()), "the alias line shows: {:?}", f.ui.screen().texts());
    let on = |f: &common::Flow| {
        f.read(|a| match &a.screen {
            Screen::Setup(s) => s.cfg.cli.vim_aliases,
            _ => true,
        })
    };
    // Under load, a press can reach the row while the step builds again. Press again only
    // while the switch is still on, so a second press never turns it back on.
    for _ in 0..3 {
        f.ui.press("label:Open nsvim with vim and vi").unwrap();
        let t = std::time::Instant::now();
        while on(&f) && t.elapsed() < std::time::Duration::from_secs(1) {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        if !on(&f) {
            break;
        }
    }
    assert!(!on(&f), "the press turns the aliases off");
    f.ui.wait_until("no alias line", std::time::Duration::from_secs(2), |s| !alias(s)).unwrap();
}

/// A long project path over a narrow tree stays inside its header, with padding before the
/// divider. The header is as wide as the tree, and the full path is its accessibility name.
#[test]
fn a_long_project_path_fits_over_a_narrow_tree() {
    let dir = scratch();
    let deep = "a-long-folder-name-for-the-test/another-long-folder-name/and-one-more-long-name/project";
    let file = write(&dir.path().join(deep), "main.rs", SRC);
    let mut f = flow(&file, None);
    f.wait_open(&file);
    f.update(|a, _| {
        a.tree.show = true;
        a.tree.width = 160.0;
    });
    let full = f.read(|a| nsvim_native::app::tilde(&a.ed.root));
    assert!(full.len() > 60, "the path is long: {full}");
    f.ui.wait_until("the header is as wide as the tree", std::time::Duration::from_secs(5), |s| {
        s.layout("id:project").unwrap().is_some_and(|l| (l.bounds.size.width.0 - 160.0).abs() < 1.0)
    })
    .unwrap();
    let h = f.ui.screen().layout("id:project").unwrap().unwrap().bounds;
    let text = f.ui.screen().layout("text:/one-more-long-name.project$/").unwrap().expect("the path shows").bounds;
    let right = |b: nsh_engine::Bounds<nsh_engine::Points>| b.origin.x.0 + b.size.width.0;
    assert!(text.origin.x.0 >= h.origin.x.0 + 11.0, "padding before the path");
    assert!(right(text) <= right(h) - 11.0, "the path ends before the padding");
    let name = f.ui.screen().accessibility(Some("id:project")).unwrap().map(|n| n.name);
    assert_eq!(name.as_deref(), Some(full.as_str()));
}
