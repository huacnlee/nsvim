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
fn unsaved_settings_stay_when_the_file_changes() {
    let mut a = app();
    a.settings = Some(super::Settings::default());
    a.cfg.font.size = 20.0;
    let mut disk = Config::default();
    disk.indent.width = 8;
    a.event(Event::Config(Ok(disk.clone())), &mut Tx::new());
    assert_eq!(a.cfg.font.size, 20.0, "the Settings tab's change stays");
    assert_eq!(a.disk_cfg, disk);
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
