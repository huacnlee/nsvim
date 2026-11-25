//! The first-launch setup of a development build, headless. This test binary is in a cargo
//! `target` folder, and it does not call `treat_dev_build_as_release`. So the app sees a
//! development build, as `nsh run` and `target/debug/nsvim` are.

mod common;

use std::time::Duration;

use common::{CONFIG, isolate_home, scratch, start_with};

/// A development build has no terminal step. Setup writes no line to the shell's rc file, and
/// the config turns every integration off.
#[test]
fn a_development_build_has_no_terminal_step_and_writes_nothing() -> Result<(), String> {
    let exe = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()).ok_or("this binary")?;
    if !nsvim_native::app::setup::dev_build(Some(&exe)) {
        eprintln!("skipped: {} is not in a cargo target folder", exe.display());
        return Ok(());
    }
    isolate_home();
    let home = std::path::PathBuf::from(std::env::var_os("HOME").ok_or("no HOME")?);
    assert!(home.ends_with("editor-nsh-home/home"), "HOME is not isolated: {}", home.display());
    let rc = home.join(".bashrc");
    let before = std::fs::read_to_string(&rc).unwrap_or_default();
    let dir = scratch();
    let mut app = start_with(nsvim_native::Launch {
        project: Some(dir.path().to_path_buf()),
        config: Some(CONFIG.into()),
        files: Vec::new(),
        first_launch: true,
    });
    // The probe answers, and the terminal step goes away: four steps, not five.
    app.wait_for("Setup 1/4 · Keys")?;
    assert!(!app.shows("Terminal"), "{:?}", app.screen().texts());
    app.press("label:Skip")?;
    app.wait_for("Ready")?;
    app.wait_for("text:/This is a development build. The terminal step is off./")?;
    app.press("label:Open nsvim")?;
    app.wait_for("id:text-area")?;
    let path = nsvim_native::config::config_path();
    let off = |t: &toml::Table| {
        let cli = t.get("cli");
        ["vim_aliases", "editor_env", "git", "default_app"]
            .iter()
            .all(|k| cli.and_then(|c| c.get(k)).and_then(|v| v.as_bool()) == Some(false))
    };
    let read = || std::fs::read_to_string(&path).ok().and_then(|t| toml::from_str::<toml::Table>(&t).ok());
    app.wait_until("config.toml with every integration off", Duration::from_secs(5), |_| read().is_some_and(|t| off(&t)))?;
    // A worker would write the rc file soon after setup. Give it time, then look.
    app.wait(Duration::from_secs(1))?;
    assert_eq!(std::fs::read_to_string(&rc).unwrap_or_default(), before, "the rc file did not change");
    Ok(())
}
