//! Settings and the first-launch setup, headless: each test drives the real view with presses
//! and keys and reads what shows, the app's echo on the command line and the written config.

mod common;

use std::sync::Mutex;
use std::time::Duration;

use common::{CONFIG, isolate_home, scratch, start, start_with, typ, type_text, write};
use nsh_testing::Headless;

/// Tests that write config.toml share one file, so they take turns.
static CONFIG_FILE: Mutex<()> = Mutex::new(());

fn config_lock() -> std::sync::MutexGuard<'static, ()> {
    CONFIG_FILE.lock().unwrap_or_else(|e| e.into_inner())
}

/// The config.toml the app writes, parsed, once `pred` holds for it.
fn written(app: &mut Headless, what: &str, pred: impl Fn(&toml::Table) -> bool) -> Result<toml::Table, String> {
    isolate_home();
    let path = nsh_app_editor::config::config_path();
    let read = || std::fs::read_to_string(&path).ok().and_then(|t| toml::from_str::<toml::Table>(&t).ok());
    app.wait_until(what, Duration::from_secs(5), |_| read().is_some_and(|t| pred(&t)))?;
    read().ok_or_else(|| "no config.toml".into())
}

fn get<'a>(t: &'a toml::Table, section: &str, key: &str) -> Option<&'a toml::Value> {
    t.get(section)?.get(key)
}

/// The app on its first launch, in an empty project. Keep the folder while the app runs.
fn first_launch() -> (Headless, tempfile::TempDir) {
    isolate_home();
    // Finishing the setup writes under $HOME. It must be the tests' own.
    let home = std::env::var("HOME").unwrap_or_default();
    assert!(home.ends_with("editor-nsh-home/home"), "HOME is not isolated: {home}");
    let dir = scratch();
    let mut app = start_with(nsh_app_editor::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    app.wait_for("How do you want to edit?").unwrap();
    // The view that takes the keys gets the focus once the window shows. Until then a key goes
    // nowhere, so Tab is tried until it moves on, and Shift+Tab comes back.
    for _ in 0..50 {
        app.press_key("tab").unwrap();
        if app.shows("setup 2/5 · mouse") {
            app.press_key("shift+tab").unwrap();
            app.wait_for("setup 1/5 · keys").unwrap();
            return (app, dir);
        }
        app.wait(Duration::from_millis(100)).unwrap();
    }
    panic!("the keys never reached the setup");
}

/// The app on a file, with Settings open by `:Settings`.
fn settings_on(text: &str, config: Option<&str>) -> Result<(Headless, tempfile::TempDir), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", text);
    let mut app = common::start_files(&[&file], config);
    app.wait_for(&format!("text:/{}/", text.lines().next().unwrap_or("")))?;
    type_text(&mut app, ":Settings\n");
    app.wait_for("id:settings")?;
    Ok((app, dir))
}

fn texts(app: &Headless) -> String {
    app.screen().texts().join("\n")
}

// Setup.

#[test]
fn setup_shows_on_first_launch_and_not_otherwise() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    assert!(app.shows("setup 1/5 · keys"), "{:?}", app.screen().texts());
    assert!(app.shows("SETUP"));
    assert!(!app.screen().matches("id:text-area")?, "the editor waits for the setup");

    let dir = scratch();
    let mut app = start(&write(dir.path(), "a.txt", "hello\n"));
    app.wait_for("text:/hello/")?;
    assert!(!app.screen().matches("id:setup")?);
    Ok(())
}

#[test]
fn setup_steps_go_forward_and_back_by_press_and_by_key() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:next →")?;
    app.wait_for("Use the mouse?")?;
    assert!(app.shows("setup 2/5 · mouse"));
    typ(&mut app, &["tab"]);
    app.wait_for("Pick a look")?;
    app.press("label:back")?;
    app.wait_for("Use the mouse?")?;
    typ(&mut app, &["tab", "tab"]);
    app.wait_for("Use editor from the terminal")?;
    assert!(app.shows("setup 4/5 · terminal"));
    typ(&mut app, &["shift+tab"]);
    app.wait_for("Pick a look")?;
    typ(&mut app, &["escape"]);
    app.wait_for("Use the mouse?")?;
    app.press("label:skip")?;
    app.wait_for("Ready")?;
    assert!(app.shows("setup 5/5 · ready"));
    assert!(app.screen().matches("label:open editor →")?);
    assert!(!app.screen().matches("label:next →")?, "the last step has no next");
    Ok(())
}

#[test]
fn setup_keys_step_picks_a_keymap_by_press_and_by_key() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    assert!(app.screen().selected("label:Neovim motions")?, "Neovim is the default");
    assert!(app.shows("Use Space as the leader key"));
    // l moves the focus to the classic card, Enter picks it.
    typ(&mut app, &["l", "enter"]);
    app.wait_until("classic picked", Duration::from_secs(2), |s| s.selected("label:Classic shortcuts").unwrap_or(false))?;
    assert!(!app.screen().selected("label:Neovim motions")?);
    app.wait_for("Add cursors with Alt-click")?;
    assert!(app.shows("arrows move   enter pick   tab next"));
    // A press picks Neovim again, and a switch changes its option.
    app.press("label:Neovim motions")?;
    app.wait_for("Use Space as the leader key")?;
    assert!(app.shows("SPC f"), "Space is the leader: {:?}", app.screen().texts());
    app.press("label:Use Space as the leader key")?;
    app.wait_for("\\ f")?;
    // The platform choice changes the keys the classic card shows.
    app.press("id:platform:macOS")?;
    app.wait_until("macOS keys", Duration::from_secs(2), |s| s.shows("⌘ S"))?;
    Ok(())
}

#[test]
fn setup_mouse_and_look_steps_change_the_picks() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    typ(&mut app, &["tab"]);
    app.wait_for("Use the mouse?")?;
    assert!(app.shows("Click places the cursor"), "mouse on shows what it can do");
    app.press("label:Keyboard only")?;
    app.wait_for_gone("Click places the cursor", Duration::from_secs(2))?;
    typ(&mut app, &["tab"]);
    app.wait_for("Pick a look")?;
    app.press("label:Light")?;
    app.wait_until("light picked", Duration::from_secs(2), |s| s.selected("label:Light").unwrap_or(false))?;
    app.press("id:font:Fira Code")?;
    app.press("id:size:16")?;
    app.wait_until("16 pt picked", Duration::from_secs(2), |s| s.selected("id:size:16").unwrap_or(false))?;
    assert!(app.screen().selected("id:font:Fira Code")?);
    app.press("label:skip")?;
    app.wait_for("Ready")?;
    for want in ["keyboard only", "light", "Fira Code, 16 pt", "Neovim motions, Space leader"] {
        assert!(app.shows(want), "{want}: {:?}", app.screen().texts());
    }
    // `change` goes back to the step of that row.
    app.press("label:change mouse")?;
    app.wait_for("Use the mouse?")?;
    Ok(())
}

#[test]
fn setup_terminal_step_shows_what_runs() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    typ(&mut app, &["tab", "tab", "tab"]);
    app.wait_for("Use editor from the terminal")?;
    let git = |s: &nsh_testing::Screen| s.texts().iter().any(|t| t.starts_with("git config --global core.editor"));
    assert!(git(app.screen()), "git is on by default: {:?}", app.screen().texts());
    assert!(app.shows("command name"), "the command's name shows while the command is on");
    app.press("label:Use for git commit messages")?;
    app.wait_until("no git command", Duration::from_secs(2), |s| !git(s))?;
    app.press("label:Add a terminal command")?;
    app.wait_for_gone("command name", Duration::from_secs(2))?;
    app.press("label:Set as $EDITOR and $VISUAL")?;
    app.wait_for("nothing. editor stays inside its window")?;
    Ok(())
}

#[test]
fn finishing_setup_writes_the_config_and_opens_the_editor() -> Result<(), String> {
    let _lock = config_lock();
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:Classic shortcuts")?;
    app.press("label:skip")?;
    app.wait_for("Ready")?;
    assert!(app.shows("Classic shortcuts (Ctrl)"), "{:?}", app.screen().texts());
    app.press("label:open editor →")?;
    app.wait_for("id:text-area")?;
    assert!(!app.screen().matches("id:setup")?);
    app.wait_for("INSERT")?;
    let cfg =
        written(&mut app, "config.toml with the classic keymap", |t| get(t, "keys", "map").and_then(|v| v.as_str()) == Some("classic"))?;
    assert_eq!(get(&cfg, "keys", "map").and_then(|v| v.as_str()), Some("classic"));
    // The terminal step ran what it showed, in the tests' own home: $EDITOR in the rc file.
    let rc = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("no HOME")?).join(".bashrc");
    app.wait_until("the rc file", Duration::from_secs(5), |_| std::fs::read_to_string(&rc).is_ok_and(|t| t.contains("export EDITOR=")))?;
    Ok(())
}

#[test]
fn enter_on_the_last_step_opens_the_editor() -> Result<(), String> {
    let _lock = config_lock();
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:skip")?;
    app.wait_for("Ready")?;
    // The focus starts on the first `change`. The last choice opens the editor.
    typ(&mut app, &["j", "j", "j", "j", "j", "enter"]);
    app.wait_for("id:text-area")?;
    app.wait_for("NORMAL")?;
    Ok(())
}

// Settings.

#[test]
fn settings_opens_with_each_section() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    assert!(app.screen().selected("id:section:appearance")?);
    assert!(app.screen().matches("id:row:theme")?);
    for (section, row) in [
        ("font", "font.family"),
        ("editor", "editor.numbers"),
        ("formatting", "format.trim_whitespace"),
        ("keys & mouse", "keys.map"),
        ("appearance", "window.opacity"),
    ] {
        app.press(&format!("id:section:{section}"))?;
        app.wait_for(&format!("id:row:{row}"))?;
        assert!(app.screen().selected(&format!("id:section:{section}"))?, "{section}");
    }
    // Keys pick sections too: 3 is the editor section, J the next one.
    typ(&mut app, &["3"]);
    app.wait_for("id:row:editor.wrap")?;
    typ(&mut app, &["shift+j"]);
    app.wait_for("id:row:format.final_newline")?;
    Ok(())
}

#[test]
fn a_switch_changes_the_app_and_w_writes_the_config() -> Result<(), String> {
    let _lock = config_lock();
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.press("id:section:editor")?;
    app.wait_for("id:row:editor.wrap")?;
    assert_eq!(app.screen().accessibility_state("label:editor.wrap")?.checked, Some(nsh_core::Checked::False));
    app.press("label:editor.wrap")?;
    app.wait_until("wrap on", Duration::from_secs(2), |s| {
        s.accessibility_state("label:editor.wrap").is_ok_and(|a| a.checked == Some(nsh_core::Checked::True))
    })?;
    app.wait_for("editor.wrap = true")?;
    // The keys still reach the app after a press: `:w` writes config.toml.
    type_text(&mut app, ":w\n");
    let cfg = written(&mut app, "config.toml with wrap on", |t| get(t, "editor", "wrap").and_then(|v| v.as_bool()) == Some(true))?;
    assert_eq!(get(&cfg, "editor", "wrap").and_then(|v| v.as_bool()), Some(true));
    // Pressed again, it turns off.
    app.press("label:editor.wrap")?;
    app.wait_for("editor.wrap = false")?;
    Ok(())
}

#[test]
fn theme_mode_and_theme_cards_change_the_theme() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    assert!(app.screen().selected("id:theme=dark")?, "{}", texts(&app));
    assert!(app.screen().matches("id:row:theme.dark")?);
    app.press("id:theme=light")?;
    app.wait_for("id:row:theme.light")?;
    assert!(!app.screen().matches("id:row:theme.dark")?, "light mode lists only light themes");
    assert!(app.screen().selected("id:theme=light")?);
    app.wait_for("text:theme.mode = \"light\"")?;
    app.press("label:Gruvbox Light")?;
    app.wait_until("the card picked", Duration::from_secs(2), |s| s.selected("id:theme.light=gruvbox-light").unwrap_or(false))?;
    app.wait_for("text:theme.light = \"gruvbox-light\"")?;
    // System mode lists both, and says what the OS asks for now.
    app.press("id:theme=system")?;
    app.wait_for("id:row:system.now")?;
    assert!(app.screen().matches("id:row:theme.dark")? && app.screen().matches("id:row:theme.light")?);
    Ok(())
}

#[test]
fn keymap_change_in_settings_takes_effect_on_keys() -> Result<(), String> {
    let (mut app, _dir) = settings_on("abc\n", None)?;
    app.press("id:section:keys & mouse")?;
    app.wait_for("id:row:keys.map")?;
    app.press("id:keys.map=classic")?;
    app.wait_for("text:keys.map = \"classic\"")?;
    // Classic keys: Ctrl+W closes Settings, and a letter types itself.
    typ(&mut app, &["ctrl+w"]);
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
    typ(&mut app, &["x"]);
    app.wait_for("text:/xabc/")?;
    Ok(())
}

#[test]
fn steps_and_the_font_select_change_the_font() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.press("id:section:font")?;
    app.wait_for("id:font.size=value")?;
    assert!(app.shows("13 pt"), "{:?}", app.screen().texts());
    app.press("id:font.size+")?;
    app.wait_for("14 pt")?;
    app.wait_for("font.size = 14.0")?;
    app.press("label:Decrease font.size")?;
    app.press("label:Decrease font.size")?;
    app.wait_for("12 pt")?;
    // The select lists the fonts. A pick changes the family.
    app.press("label:font.family")?;
    app.wait_for("text:Fira Code")?;
    app.press("text:Fira Code")?;
    app.wait_for("text:font.family = \"Fira Code\"")?;
    Ok(())
}

#[test]
fn keys_move_through_the_rows_and_change_them() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    typ(&mut app, &["3"]);
    app.wait_for("id:row:editor.numbers")?;
    assert!(app.screen().selected("id:editor.numbers=hybrid")?);
    // l changes the row the keys are on: the numbers, from hybrid to relative.
    typ(&mut app, &["l"]);
    app.wait_until("relative", Duration::from_secs(2), |s| s.selected("id:editor.numbers=relative").unwrap_or(false))?;
    // j goes to the cursor row. Enter changes it to the next shape.
    typ(&mut app, &["j", "enter"]);
    app.wait_until("bar", Duration::from_secs(2), |s| s.selected("id:editor.cursor=bar").unwrap_or(false))?;
    Ok(())
}

#[test]
fn the_preview_follows_the_options() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.wait_for("id:settings-preview")?;
    assert!(app.screen().matches(r#"text:/theme\.mode = "dark"/"#)?, "{:?}", app.screen().texts());
    app.press("id:section:editor")?;
    app.wait_for("id:row:editor.whitespace")?;
    let dots = |s: &nsh_testing::Screen, n: usize| s.texts().iter().any(|t| t.starts_with(&format!("{}let mut pos", "·".repeat(n))));
    assert!(!dots(app.screen(), 4), "no dots before: {:?}", app.screen().texts());
    app.press("label:editor.whitespace")?;
    app.wait_until("four dots", Duration::from_secs(2), |s| dots(s, 4))?;
    app.press("id:indent.width=2")?;
    app.wait_until("two dots", Duration::from_secs(2), |s| dots(s, 2) && !dots(s, 4))?;
    // The numbers follow too. Only absolute numbers show the last line as 12.
    app.press("id:editor.numbers=off")?;
    app.wait_until("no numbers", Duration::from_secs(2), |s| !s.shows("12"))?;
    app.press("id:editor.numbers=absolute")?;
    app.wait_for("12")?;
    Ok(())
}

#[test]
fn closing_settings_returns_to_the_text() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    assert!(!app.screen().matches("id:text-area")?);
    type_text(&mut app, ":q\n");
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
    app.wait_for("id:text-area")?;
    app.wait_for("text:/hello/")?;
    Ok(())
}
