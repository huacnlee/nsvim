//! Zoom, Reopen Closed Tab and the Edit menu's line commands, without a window.

use std::path::PathBuf;

use editor::buffer::{Buffer, Pos};
use editor::editor::{Doc, KeyCode, KeyInput, Keymap, Mode};

use super::App;
use super::fx::Tx;
use super::view_cmds::Zoom;

fn app(keymap: Keymap, text: &str) -> App {
    let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
    let mut a = App::new(cfg, PathBuf::from("/project"), false, None);
    a.screen = super::Screen::Editor;
    a.ed.keymap = keymap;
    a.ed.mode = if keymap == Keymap::Vim { Mode::Normal } else { Mode::Insert };
    a.ed.docs.push(Doc::new(Buffer::from_str(text)));
    a.ed.active = 0;
    a
}

fn text(a: &App) -> String {
    a.ed.buf().slice(0, a.ed.buf().len_chars())
}

fn mod_key(c: char, shift: bool) -> KeyInput {
    let mac = cfg!(target_os = "macos");
    KeyInput { code: KeyCode::Char(c), ctrl: !mac, alt: false, shift, cmd: true }
}

#[test]
fn zoom_steps_one_point_scales_the_interface_and_writes_the_config() {
    let mut a = app(Keymap::Vim, "x\n");
    a.zoom(Zoom::In);
    assert_eq!((a.cfg.font.size, a.cfg.ui.size), (14.0, 13.5));
    assert_eq!(a.disk_cfg.font.size, 14.0, "config.toml gets the change at once");
    a.zoom(Zoom::Out);
    assert_eq!((a.cfg.font.size, a.cfg.ui.size), (13.0, 12.5));
    a.zoom(Zoom::In);
    a.zoom(Zoom::In);
    a.zoom(Zoom::Reset);
    let def = crate::config::Config::default();
    assert_eq!((a.cfg.font.size, a.cfg.ui.size), (def.font.size, def.ui.size));
    assert_eq!(a.disk_cfg.font.size, def.font.size);
}

#[test]
fn zoom_stops_at_8_and_40_points() {
    let mut a = app(Keymap::Vim, "x\n");
    a.cfg.font.size = 39.0;
    for _ in 0..3 {
        a.zoom(Zoom::In);
    }
    assert_eq!(a.cfg.font.size, 40.0);
    assert!(a.cfg.ui.size <= 40.0);
    assert!(a.ed.msg.as_ref().is_some_and(|m| m.text.contains("largest")));
    a.cfg.font.size = 9.0;
    for _ in 0..3 {
        a.zoom(Zoom::Out);
    }
    assert_eq!(a.cfg.font.size, 8.0);
    assert!(a.cfg.ui.size >= 8.0);
}

#[test]
fn zoom_keys_work_in_every_keymap_and_leave_ctrl_minus_to_back_on_macos() {
    for keymap in [Keymap::Vim, Keymap::Classic] {
        let mut a = app(keymap, "x\n");
        a.key(mod_key('=', false), &mut Tx::new());
        assert_eq!(a.cfg.font.size, 14.0, "{keymap:?}: Mod =");
        a.key(mod_key('+', true), &mut Tx::new());
        assert_eq!(a.cfg.font.size, 15.0, "{keymap:?}: Mod +");
        a.key(mod_key('-', false), &mut Tx::new());
        assert_eq!(a.cfg.font.size, 14.0, "{keymap:?}: Mod -");
        a.key(mod_key('0', false), &mut Tx::new());
        assert_eq!(a.cfg.font.size, 13.0, "{keymap:?}: Mod 0");
        assert_eq!(text(&a), "x\n", "a zoom key types nothing");
    }
    if cfg!(target_os = "macos") {
        let ctrl_minus = KeyInput { code: KeyCode::Char('-'), ctrl: true, alt: false, shift: false, cmd: false };
        assert_eq!(Zoom::from_key(&ctrl_minus), None);
    }
}

#[test]
fn closed_tabs_reopen_last_first_and_pass_over_open_and_gone_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = |n: &str| {
        let p = dir.path().join(n);
        std::fs::write(&p, "xyz\n").unwrap();
        p
    };
    let (a_txt, b_txt) = (path("a.txt"), path("b.txt"));
    let mut a = app(Keymap::Vim, "");
    a.ed.docs.clear();
    for p in [&a_txt, &b_txt] {
        a.ed.open(p).unwrap();
    }
    a.ed.set_cursor(Pos::new(0, 1));
    a.ed.close(1);
    a.ed.close(0);
    let names: Vec<_> = a.ed.closed.iter().map(|(p, at)| (p.file_name().unwrap().to_owned(), *at)).collect();
    assert_eq!(names, vec![("b.txt".into(), Pos::new(0, 1)), ("a.txt".into(), Pos::new(0, 0))], "the last closed is last");
    // a.txt is open again, and a gone file is passed over: b.txt is next.
    a.ed.open(&a_txt).unwrap();
    a.ed.closed.insert(1, (dir.path().join("gone.txt"), Pos::new(0, 0)));
    a.reopen_closed();
    assert!(a.ed.closed.is_empty(), "a.txt and gone.txt are passed over, b.txt is taken");
    a.reopen_closed();
    assert!(a.ed.msg.as_ref().is_some_and(|m| m.text.contains("no closed tab")));
}

#[test]
fn the_closed_list_keeps_the_last_20() {
    let mut a = app(Keymap::Vim, "");
    a.ed.docs.clear();
    for i in 0..25 {
        let mut b = Buffer::from_str("x\n");
        b.path = Some(PathBuf::from(format!("/p/{i}.txt")));
        a.ed.docs.push(Doc::new(b));
        a.ed.close(0);
    }
    assert_eq!(a.ed.closed.len(), editor::editor::CLOSED_KEPT);
    assert_eq!(a.ed.closed.last().unwrap().0, PathBuf::from("/p/24.txt"));
    assert_eq!(a.ed.closed[0].0, PathBuf::from("/p/5.txt"));
}

#[test]
fn the_edit_menu_line_items_run_in_both_keymaps() {
    for keymap in [Keymap::Vim, Keymap::Classic] {
        let mut a = app(keymap, "one\ntwo\n");
        let mut tx = Tx::new();
        a.app_menu_pick("edit:comment", &mut tx);
        assert_eq!(text(&a), "// one\ntwo\n", "{keymap:?}: Toggle Comment");
        a.app_menu_pick("edit:line-down", &mut tx);
        assert_eq!(text(&a), "two\n// one\n", "{keymap:?}: Move Line Down");
        assert_eq!(a.ed.cursor().line, 1, "{keymap:?}: the cursor goes with the line");
        a.app_menu_pick("edit:line-up", &mut tx);
        assert_eq!(text(&a), "// one\ntwo\n", "{keymap:?}: Move Line Up");
        a.app_menu_pick("edit:indent", &mut tx);
        assert!(text(&a).starts_with(char::is_whitespace), "{keymap:?}: Indent");
        a.app_menu_pick("edit:outdent", &mut tx);
        assert_eq!(text(&a), "// one\ntwo\n", "{keymap:?}: Outdent");
        a.app_menu_pick("edit:delete-line", &mut tx);
        assert_eq!(text(&a), "two\n", "{keymap:?}: Delete Line");
        a.app_menu_pick("undo", &mut tx);
        assert_eq!(text(&a), "// one\ntwo\n", "{keymap:?}: one undo step per command");
        if keymap == Keymap::Vim {
            assert_eq!(a.ed.mode, Mode::Normal);
        }
    }
}

#[test]
fn a_visual_selection_takes_its_whole_lines_and_ends() {
    let mut a = app(Keymap::Vim, "one\ntwo\nthree\n");
    a.ed.mode = Mode::Visual(editor::editor::VisualKind::Char);
    a.ed.doc_mut().view.anchor = Some(Pos::new(0, 1));
    a.ed.set_cursor(Pos::new(1, 0));
    a.app_menu_pick("edit:comment", &mut Tx::new());
    assert_eq!(text(&a), "// one\n// two\nthree\n");
    assert_eq!(a.ed.mode, Mode::Normal);
    assert_eq!(a.ed.doc().view.anchor, None);
}
