//! The lifecycle without a window: config reloads, the Omarchy theme, closing over unsaved
//! edits, and the window's attributes from the config.

use std::path::PathBuf;

use editor::buffer::Buffer;
use editor::editor::{Doc, Keymap};

use super::App;
use super::fx::{Effect, Tx};
use crate::config::Config;
use crate::services::Event;
use crate::theme::{Omarchy, Palette};

fn app() -> App {
    App::new(Config::default(), PathBuf::from("/p"), false, None)
}

fn quits(tx: &Tx) -> bool {
    tx.effects.iter().any(|e| matches!(e, Effect::Quit))
}

fn message(app: &App) -> String {
    app.ed.msg.as_ref().map(|m| m.text.clone()).unwrap_or_default()
}

#[test]
fn a_changed_config_applies_and_the_editors_own_write_does_nothing() {
    let mut a = app();
    let mut cfg = Config::default();
    cfg.keys.map = "classic".into();
    a.event(Event::Config(Ok(cfg.clone())), &mut Tx::new());
    assert_eq!(a.ed.keymap, Keymap::Classic);
    assert_eq!(a.disk_cfg, cfg);
    assert!(message(&a).contains("config.toml changed on disk"), "{}", message(&a));
    a.ed.message("other");
    a.event(Event::Config(Ok(cfg)), &mut Tx::new());
    assert_eq!(message(&a), "other", "the same config again changes nothing");
}

#[test]
fn a_config_that_does_not_parse_keeps_the_one_in_use() {
    let mut a = app();
    a.event(Event::Config(Err("config.toml: expected `]`".into())), &mut Tx::new());
    assert_eq!(a.cfg, Config::default());
    assert!(message(&a).contains("expected"));
}

#[test]
fn a_new_omarchy_theme_recolours_the_editor() {
    let mut a = app();
    a.cfg.theme.mode = "omarchy".into();
    let before: Palette = crate::ui::palette(&a, true);
    let mut palette = crate::theme::builtin("light");
    palette.bg = crate::color::hex("#123456").unwrap();
    let o = Omarchy { name: "test".into(), palette, dir: PathBuf::from("/themes/test") };
    a.event(Event::Omarchy(Some(o)), &mut Tx::new());
    let after = crate::ui::palette(&a, true);
    assert_ne!(before.bg, after.bg);
    assert_eq!(after.bg.to_hex(), "#123456");
}

#[test]
fn closing_over_unsaved_edits_asks_once_then_quits() {
    let mut a = app();
    let mut doc = Doc::new(Buffer::from_str("x\n"));
    doc.buf.begin(Default::default());
    let at = doc.buf.len_chars();
    doc.buf.replace(at, at, "y");
    doc.buf.commit(Default::default());
    a.ed.docs.push(doc);
    // A unit test writes no session file into the user's state folder.
    a.exited = true;
    let mut tx = Tx::new();
    a.clock = 10.0;
    a.close_requested(&mut tx);
    assert!(!quits(&tx));
    assert!(message(&a).starts_with("E37"), "{}", message(&a));
    a.clock = 10.0 + super::lifecycle::CLOSE_AGAIN_SECS + 1.0;
    a.close_requested(&mut tx);
    assert!(!quits(&tx), "a close long after the warning warns again");
    a.clock += 1.0;
    a.close_requested(&mut tx);
    assert!(quits(&tx));
}

#[test]
fn the_window_follows_the_config() {
    let mut cfg = Config::default();
    let a = crate::window_attributes(&cfg);
    assert_eq!(a.transparent, Some(false));
    assert_eq!(a.close_on_press, Some(false), "the app decides about unsaved edits");
    let min = a.min_size.flatten().expect("a least size");
    assert_eq!((min.width.0, min.height.0), crate::MIN_SIZE);
    cfg.window.opacity = 90;
    assert_eq!(crate::window_attributes(&cfg).transparent, Some(true));
    assert_eq!(crate::opacity(&cfg), 0.9);
    cfg.window.opacity = 10;
    assert_eq!(crate::opacity(&cfg), 0.7, "never less than 70%");
}

#[test]
fn setup_asks_each_name_once_and_takes_the_workers_answers() {
    use crate::services::probe::Answer;
    let mut a = App::new(Config::default(), PathBuf::from("/p"), true, None);
    let super::Screen::Setup(s) = &mut a.screen else { panic!("setup shows") };
    let first = s.check_name();
    assert_eq!(first, vec!["nsvim".to_string()], "setup asks only for the command's name");
    assert!(s.check_name().is_empty(), "each name is asked once");
    let name = s.cfg.cli.name.clone();
    a.setup_answer(Answer::OnPath(vec![(name, Some("/usr/bin/editor".into()))]));
    a.setup_answer(Answer::Facts { exe: Some(PathBuf::from("/opt/editor")), bin_on_path: false, nvim_leader: Some('\\') });
    let super::Screen::Setup(s) = &a.screen else { panic!("setup shows") };
    assert_eq!(s.taken().as_deref(), Some("/usr/bin/editor"));
    assert_eq!((s.exe.clone(), s.bin_on_path, s.nvim_leader), (Some(PathBuf::from("/opt/editor")), false, Some('\\')));
}

#[test]
fn the_file_worker_copies_the_old_folders_and_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let (old, new) = (dir.path().join("editor"), dir.path().join("nsvim"));
    std::fs::create_dir_all(old.join("undo")).unwrap();
    std::fs::write(old.join("undo/1.json"), "{}").unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let mut cfg = Config::default();
    cfg.ipc.enabled = false;
    let mut a = App::new(cfg, project, false, None);
    a.exited = true;
    a.migration = crate::migrate::plan_for(&[(old.clone(), new.clone())]);
    a.setup(std::sync::Arc::new(|| {}));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !message(&a).contains("copied") && std::time::Instant::now() < deadline {
        a.poll(0.0, &mut Tx::new());
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(message(&a).contains("The old folders stay"), "{}", message(&a));
    assert!(new.join("undo/1.json").is_file());
    assert!(old.join("undo/1.json").is_file());
}
