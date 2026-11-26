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
    let path = nsvim_native::config::config_path();
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
    // These tests run from target/debug. Their setup writes to their own home as a release does.
    nsvim_native::app::setup::treat_dev_build_as_release();
    // Finishing the setup writes under $HOME. It must be the tests' own.
    let home = std::env::var("HOME").unwrap_or_default();
    assert!(home.ends_with("editor-nsh-home/home"), "HOME is not isolated: {home}");
    let dir = scratch();
    let mut app = start_with(nsvim_native::Launch {
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
        if app.shows("Setup 2/5 · Mouse") {
            app.press_key("shift+tab").unwrap();
            app.wait_for("Setup 1/5 · Keys").unwrap();
            return (app, dir);
        }
        app.wait(Duration::from_millis(100)).unwrap();
    }
    panic!("the keys never reached the setup");
}

/// The scratch folder of a test, and its turn at config.toml: each change in Settings writes it.
struct Held {
    _dir: tempfile::TempDir,
    _lock: std::sync::MutexGuard<'static, ()>,
}

/// The app on a file, with Settings open by `:Settings`.
fn settings_on(text: &str, config: Option<&str>) -> Result<(Headless, Held), String> {
    let lock = config_lock();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", text);
    let mut app = common::start_files(&[&file], config);
    app.wait_for(&format!("text:/{}/", text.lines().next().unwrap_or("")))?;
    type_text(&mut app, ":Settings\n");
    app.wait_for("id:settings")?;
    Ok((app, Held { _dir: dir, _lock: lock }))
}

fn texts(app: &Headless) -> String {
    app.screen().texts().join("\n")
}

// Setup.

#[test]
fn setup_shows_on_first_launch_and_not_otherwise() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    assert!(app.shows("Setup 1/5 · Keys"), "{:?}", app.screen().texts());
    assert!(app.shows("SETUP"));
    assert!(!app.screen().matches("id:text-area")?, "the editor waits for the setup");

    let dir = scratch();
    let mut app = start(&write(dir.path(), "a.txt", "hello\n"));
    app.wait_for("text:/hello/")?;
    assert!(!app.screen().matches("id:setup")?);
    Ok(())
}

/// `:Setup` shows setup again over the editor, from the config in use. Cancel leaves it as it
/// was; Save writes the new picks and goes back to the open file.
#[test]
fn setup_runs_again_from_the_editor_with_the_current_picks() -> Result<(), String> {
    let _lock = config_lock();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "hello\n");
    let mut app = common::start_files(&[&file], Some(&common::keymap_config("neovim")));
    app.wait_for("text:/hello/")?;
    type_text(&mut app, ":Setup\n");
    app.wait_for("How do you want to edit?")?;
    assert!(app.shows("Setup 1/5 · Keys"), "{:?}", app.screen().texts());
    assert!(app.screen().matches("label:Cancel")?, "setup run again can be left");
    // Esc on the first step leaves it too, with nothing changed.
    typ(&mut app, &["escape"]);
    app.wait_for("text:/hello/")?;
    assert!(!app.screen().matches("id:setup")?);
    app.wait_for("text:/nothing changed/")?;

    type_text(&mut app, ":Setup\n");
    app.wait_for("How do you want to edit?")?;
    app.press("label:Classic shortcuts")?;
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    assert!(!app.screen().matches("label:Open NsVim")?, "the editor is open already");
    app.press("label:Save")?;
    app.wait_for("text:/hello/")?;
    assert!(!app.screen().matches("id:setup")?);
    let cfg =
        written(&mut app, "config.toml with the classic keymap", |t| get(t, "keys", "map").and_then(|v| v.as_str()) == Some("classic"))?;
    assert_eq!(get(&cfg, "keys", "map").and_then(|v| v.as_str()), Some("classic"));
    assert!(!app.screen().matches("id:mode-badge")?, "the classic keymap shows no vim mode");
    Ok(())
}

#[test]
fn setup_steps_go_forward_and_back_by_press_and_by_key() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:Next")?;
    app.wait_for("Use the mouse?")?;
    assert!(app.shows("Setup 2/5 · Mouse"));
    typ(&mut app, &["tab"]);
    app.wait_for("Pick a look")?;
    app.press("label:Back")?;
    app.wait_for("Use the mouse?")?;
    typ(&mut app, &["tab", "tab"]);
    app.wait_for("Use NsVim from the terminal")?;
    assert!(app.shows("Setup 4/5 · Terminal"));
    typ(&mut app, &["shift+tab"]);
    app.wait_for("Pick a look")?;
    typ(&mut app, &["escape"]);
    app.wait_for("Use the mouse?")?;
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    assert!(app.shows("Setup 5/5 · Ready"));
    assert!(app.screen().matches("label:Open NsVim")?);
    assert!(!app.screen().matches("label:Next")?, "the last step has no next");
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
    assert!(app.shows("Arrows move   Enter picks   Tab next"));
    // A press picks Neovim again, and a checkbox changes its option.
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
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    for want in ["Keyboard only", "Light", "Fira Code, 16 pt", "Neovim motions, Space leader"] {
        assert!(app.shows(want), "{want}: {:?}", app.screen().texts());
    }
    // `change` goes back to the step of that row.
    app.press("label:Change mouse")?;
    app.wait_for("Use the mouse?")?;
    Ok(())
}

#[test]
fn setup_terminal_step_shows_what_runs() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    typ(&mut app, &["tab", "tab", "tab"]);
    app.wait_for("Use NsVim from the terminal")?;
    let git = |s: &nsh_testing::Screen| s.texts().iter().any(|t| t.starts_with("git config --global core.editor"));
    assert!(git(app.screen()), "git is on by default: {:?}", app.screen().texts());
    let alias = |s: &nsh_testing::Screen| s.texts().iter().any(|t| t.contains("alias nvim=") && t.contains(" vi="));
    assert!(alias(app.screen()), "nvim, vim and vi open nsvim: {:?}", app.screen().texts());
    // Setup puts no command on $PATH.
    assert!(!app.screen().texts().iter().any(|t| t.contains("ln -s") || t.contains("export PATH")), "{:?}", app.screen().texts());
    app.press("label:Use for git commit messages")?;
    app.wait_until("no git command", Duration::from_secs(2), |s| !git(s))?;
    app.press("label:Set as $EDITOR and $VISUAL")?;
    app.press("label:Open NsVim with nvim, vim and vi")?;
    app.wait_for("Nothing. NsVim stays inside its window.")?;
    Ok(())
}

#[test]
fn finishing_setup_writes_the_config_and_opens_the_editor() -> Result<(), String> {
    let _lock = config_lock();
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:Classic shortcuts")?;
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    assert!(app.shows("Classic shortcuts (Ctrl)"), "{:?}", app.screen().texts());
    // The last step says how to start nsvim: the aliases are on by default.
    app.wait_for("text:/nvim . opens a folder/")?;
    app.press("label:Open NsVim")?;
    app.wait_for("id:text-area")?;
    assert!(!app.screen().matches("id:setup")?);
    let cfg =
        written(&mut app, "config.toml with the classic keymap", |t| get(t, "keys", "map").and_then(|v| v.as_str()) == Some("classic"))?;
    assert_eq!(get(&cfg, "keys", "map").and_then(|v| v.as_str()), Some("classic"));
    // The classic keymap shows no vim mode.
    assert!(!app.screen().matches("id:mode-badge")?, "{:?}", app.screen().texts());
    // The terminal step ran what it showed, in the tests' own home: $EDITOR in the rc file.
    let rc = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("no HOME")?).join(".bashrc");
    let ran = |t: &str| t.contains("export EDITOR=") && t.contains("alias nvim=");
    app.wait_until("the rc file", Duration::from_secs(5), |_| std::fs::read_to_string(&rc).is_ok_and(|t| ran(&t)))?;
    Ok(())
}

#[test]
fn enter_on_the_last_step_opens_the_editor() -> Result<(), String> {
    let _lock = config_lock();
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:Skip")?;
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
fn a_checkbox_changes_the_app_and_config_toml_at_once() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.press("id:section:editor")?;
    app.wait_for("id:row:editor.wrap")?;
    assert_eq!(app.screen().accessibility_state("label:editor.wrap")?.checked, Some(nsh_core::Checked::False));
    app.press("label:editor.wrap")?;
    app.wait_until("wrap on", Duration::from_secs(2), |s| {
        s.accessibility_state("label:editor.wrap").is_ok_and(|a| a.checked == Some(nsh_core::Checked::True))
    })?;
    app.wait_for("editor.wrap = true")?;
    // The change is on disk with no `:w`.
    let cfg = written(&mut app, "config.toml with wrap on", |t| get(t, "editor", "wrap").and_then(|v| v.as_bool()) == Some(true))?;
    assert_eq!(get(&cfg, "editor", "wrap").and_then(|v| v.as_bool()), Some(true));
    // It is a checkbox, and its description takes the press too: pressed again, it turns off,
    // on disk too.
    assert!(app.screen().matches("role:checkbox[editor.wrap]")?, "{}", texts(&app));
    app.press("text:Soft-wrap long lines")?;
    app.wait_for("editor.wrap = false")?;
    written(&mut app, "config.toml with wrap off", |t| get(t, "editor", "wrap").and_then(|v| v.as_bool()) == Some(false))?;
    // The keys still reach the app after a press: Esc closes Settings.
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
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
fn escape_closes_settings_and_shows_the_text() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    // `:` does nothing in Settings: no command line opens, and Settings stays.
    type_text(&mut app, ":");
    app.wait(Duration::from_millis(200))?;
    assert!(app.screen().matches("id:settings")?);
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
    app.wait_for("id:text-area")?;
    app.wait_for("text:/hello/")?;
    app.wait_for("NORMAL")?;
    Ok(())
}

#[test]
fn settings_covers_the_window_and_adds_no_tab() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.wait_for("text:Settings")?;
    assert!(!app.screen().matches("role:tab[settings]")?, "Settings is not a tab");
    assert!(app.screen().matches("role:tab[a.txt]")?, "the file keeps its tab under Settings");
    // The close button closes it.
    app.press("label:Close settings")?;
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
    app.wait_for("NORMAL")?;
    Ok(())
}

#[test]
fn a_press_on_the_empty_area_keeps_settings_open() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    // A short section leaves empty space under its rows, and the header has space beside the
    // title. A press there does not close Settings.
    app.press("id:section:formatting")?;
    app.wait_for("id:row:format.line_endings")?;
    // The window is 1200 by 800 points.
    for (x, y) in [(600.0, 700.0), (600.0, 20.0)] {
        app.press_at("id:settings", common::point(x, y))?;
        app.wait(Duration::from_millis(200))?;
        assert!(app.screen().matches("id:settings")?, "a press at {x},{y} closed Settings");
    }
    Ok(())
}

// Read-only files and folders use Unix permissions.
#[cfg(unix)]
#[test]
fn a_failed_write_shows_in_the_header() -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let (mut app, _dir) = settings_on("hello\n", None)?;
    app.press("id:section:editor")?;
    app.press("label:editor.wrap")?;
    written(&mut app, "config.toml", |_| true)?;
    // config.toml and its folder refuse writes until this test ends.
    struct ReadOnly(std::path::PathBuf);
    impl Drop for ReadOnly {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
            let _ = std::fs::set_permissions(self.0.join("config.toml"), std::fs::Permissions::from_mode(0o644));
        }
    }
    let path = nsvim_native::config::config_path();
    let folder = ReadOnly(path.parent().ok_or("no folder")?.to_path_buf());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).map_err(|e| e.to_string())?;
    std::fs::set_permissions(&folder.0, std::fs::Permissions::from_mode(0o555)).map_err(|e| e.to_string())?;
    app.press("label:editor.wrap")?;
    app.wait_for("id:settings-error")?;
    assert!(app.shows("Could not save config.toml."), "{:?}", app.screen().texts());
    drop(folder);
    // A good write clears it.
    app.press("label:editor.wrap")?;
    app.wait_for_gone("id:settings-error", Duration::from_secs(5))?;
    Ok(())
}

// The theme mode keeps light, dark and system. System follows Omarchy where it runs.

#[test]
fn settings_offers_no_omarchy_theme_mode() -> Result<(), String> {
    let (app, _dir) = settings_on("hello\n", None)?;
    for mode in ["light", "dark", "system"] {
        assert!(app.screen().matches(&format!("id:theme={mode}"))?, "{mode}");
    }
    assert!(!app.screen().matches("id:theme=omarchy")?, "{}", texts(&app));
    assert!(!app.screen().matches("id:row:omarchy.palette")?);
    Ok(())
}

#[test]
fn the_palette_has_no_omarchy_theme_entry() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "hello\n");
    let mut app = common::start_files(&[&file], Some("[ipc]\nenabled = false\n[keys]\nmap = \"classic\"\n"));
    app.wait_for("text:/hello/")?;
    typ(&mut app, &["ctrl+shift+p"]);
    app.wait_for("text:Commands")?;
    // The palette lists more than fits, so filter to the themes.
    type_text(&mut app, "theme");
    app.wait_for("Theme: follow the OS")?;
    assert!(!app.shows("Theme: follow Omarchy"), "{}", texts(&app));
    Ok(())
}

#[test]
fn set_theme_omarchy_turns_into_system_and_says_so() -> Result<(), String> {
    let dir = scratch();
    let mut app = start(&write(dir.path(), "a.txt", "hello\n"));
    app.wait_for("text:/hello/")?;
    type_text(&mut app, ":set theme=omarchy\n");
    app.wait_for("theme=omarchy is now theme=system. System follows the Omarchy theme")?;
    type_text(&mut app, ":Settings\n");
    app.wait_for("id:settings")?;
    assert!(app.screen().selected("id:theme=system")?, "{}", texts(&app));
    Ok(())
}

#[test]
fn an_old_config_with_the_omarchy_mode_loads_as_system() -> Result<(), String> {
    let (app, _dir) = settings_on("hello\n", Some("[ipc]\nenabled = false\n[theme]\nmode = \"omarchy\"\n"))?;
    assert!(app.screen().selected("id:theme=system")?, "{}", texts(&app));
    // System's rows show: the OS themes and what the OS asks for now (no Omarchy theme here).
    assert!(app.screen().matches("id:row:system.now")?);
    assert!(app.screen().matches("id:row:theme.dark")? && app.screen().matches("id:row:theme.light")?);
    Ok(())
}

#[test]
fn edit_config_toml_closes_settings_and_opens_the_file() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    // The path of config.toml shows under the sections.
    app.wait_until("the path", Duration::from_secs(2), |s| s.texts().iter().any(|t| t.ends_with("nsvim/config.toml")))?;
    app.press("label:Edit config.toml")?;
    app.wait_for_gone("id:settings", Duration::from_secs(2))?;
    app.wait_for("id:text-area")?;
    // The buffer line has a tab for config.toml, and the text area shows the file.
    app.wait_until("config.toml open", Duration::from_secs(5), |s| {
        let t = s.texts();
        t.contains(&"config.toml") && t.contains(&"[theme]")
    })?;
    app.wait_for("NORMAL")?;
    Ok(())
}

#[test]
fn a_press_in_setup_gives_the_keys_back_to_the_editor() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    // A press on a card picks it. The next key goes to setup, not to the card.
    app.press("label:Classic shortcuts")?;
    app.wait_until("classic picked", Duration::from_secs(2), |s| s.selected("label:Classic shortcuts").unwrap_or(false))?;
    typ(&mut app, &["tab"]);
    app.wait_for("Use the mouse?")?;
    // The same after a checkbox: Shift+Tab goes back a step.
    app.press("label:Click places the cursor")?;
    app.wait_until("click off", Duration::from_secs(2), |s| {
        s.accessibility_state("label:Click places the cursor").is_ok_and(|a| a.checked == Some(nsh_core::Checked::False))
    })?;
    typ(&mut app, &["shift+tab"]);
    app.wait_for("How do you want to edit?")?;
    Ok(())
}

#[test]
fn the_ready_step_lists_the_tips_of_the_keymap() -> Result<(), String> {
    let (mut app, _dir) = first_launch();
    app.wait_for("How do you want to edit?")?;
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    for want in ["Find a file", "Every key, in a popup", ":Settings", "SPC"] {
        assert!(app.shows(want), "{want}: {:?}", app.screen().texts());
    }
    assert!(app.screen().matches("id:setup-summary")?);
    app.press("label:Change keys")?;
    app.wait_for("How do you want to edit?")?;
    app.press("label:Classic shortcuts")?;
    app.press("label:Skip")?;
    app.wait_for("Command palette")?;
    assert!(app.shows("Classic shortcuts (Ctrl)"), "{:?}", app.screen().texts());
    Ok(())
}

#[test]
fn hover_changes_the_other_sections_and_not_the_active_one() -> Result<(), String> {
    let (mut app, _dir) = settings_on("hello\n", None)?;
    let bg = |app: &Headless, sel: &str| app.screen().view_style(sel).ok().flatten().and_then(|s| s.background_color);
    let active = bg(&app, "id:section:appearance");
    let other = bg(&app, "id:section:font");
    app.hover("id:section:appearance")?;
    app.wait(Duration::from_millis(100))?;
    assert_eq!(bg(&app, "id:section:appearance"), active, "the active section keeps its look under the pointer");
    app.hover("id:section:font")?;
    app.wait_until("the other section takes hover", Duration::from_secs(2), |s| {
        s.view_style("id:section:font").ok().flatten().and_then(|s| s.background_color) != other
    })?;
    Ok(())
}
