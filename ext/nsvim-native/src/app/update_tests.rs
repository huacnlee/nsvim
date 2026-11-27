use std::path::PathBuf;

use crate::app::App;
use crate::app::fx::Tx;
use crate::config::Config;
use crate::services::Event;
use crate::services::update::{Checked, Release};

fn app() -> App {
    App::new(Config::default(), PathBuf::from("/p"), false, None)
}

fn message(app: &App) -> String {
    app.ed.msg.as_ref().map(|m| m.text.clone()).unwrap_or_default()
}

fn release() -> Release {
    Release { version: "9.0.0".into(), url: "https://github.com/huacnlee/nsvim/releases/tag/v9.0.0".into() }
}

#[test]
fn a_found_release_is_announced_and_kept_for_update() {
    let mut a = app();
    a.event(Event::Update(Checked { result: Ok(Some(release())), manual: false }), &mut Tx::new());
    assert!(message(&a).contains("NsVim 9.0.0 is available"), "{}", message(&a));
    assert!(message(&a).contains(":Update"));
    assert_eq!(a.update, Some(release()));
}

#[test]
fn an_automatic_check_stays_quiet_unless_it_finds_something() {
    let mut a = app();
    a.event(Event::Update(Checked { result: Ok(None), manual: false }), &mut Tx::new());
    a.event(Event::Update(Checked { result: Err("offline".into()), manual: false }), &mut Tx::new());
    assert_eq!(message(&a), "");
    a.event(Event::Update(Checked { result: Ok(None), manual: true }), &mut Tx::new());
    assert!(message(&a).contains("is the latest version"), "{}", message(&a));
    a.event(Event::Update(Checked { result: Err("offline".into()), manual: true }), &mut Tx::new());
    assert_eq!(message(&a), "offline");
}

#[test]
fn update_without_a_known_release_checks_first() {
    let mut a = app();
    a.run_update(&mut Tx::new());
    assert!(message(&a).contains("Checking for updates"), "{}", message(&a));
}

#[test]
fn a_test_build_is_not_replaced() {
    // The test binary lives in target/, which the install script never installs to.
    let mut a = app();
    a.update = Some(release());
    let mut tx = Tx::new();
    a.run_update(&mut tx);
    assert!(message(&a).contains("not installed by the install script"), "{}", message(&a));
    assert!(tx.effects.is_empty());
}

#[test]
fn only_the_window_of_an_installed_copy_uses_the_nsh_updater() {
    use crate::app::update::native_updates;
    use std::path::Path;
    let mac = Path::new("/Users/me/Applications/NsVim.app/Contents/MacOS/nsvim");
    assert!(native_updates(Some(mac), false, "macos"));
    // The terminal editor keeps `:Update` with the install script.
    assert!(!native_updates(Some(mac), true, "macos"));
    let linux = Path::new("/home/me/.local/share/nsvim/nsvim");
    assert!(native_updates(Some(linux), false, "linux"));
    // A build, or mise's copy, updates the way it came.
    assert!(!native_updates(Some(Path::new("/work/nsvim/target/release/nsvim")), false, "linux"));
    assert!(!native_updates(Some(Path::new("/home/me/.local/share/mise/installs/nsvim/0.2.1/nsvim")), false, "linux"));
    assert!(!native_updates(None, false, "linux"));
}

#[test]
fn the_palette_checks_for_updates() {
    // The test binary is not an installed copy, so the command takes the `:Update` path.
    let mut a = app();
    let mut tx = Tx::new();
    a.run_command("update", &mut tx);
    assert!(message(&a).contains("Checking for updates"), "{}", message(&a));
    assert!(crate::app::palette::COMMANDS.iter().any(|c| c.id == "update" && c.label == "Check for updates…"));
}
