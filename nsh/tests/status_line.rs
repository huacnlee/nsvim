//! The status line and the command line in the window (`ui/status-line.tsx` and
//! `ui/command-line.tsx`): what Rust gives them shows, and their presses act.

mod common;

use std::time::Duration;

use common::{flow, isolate_home, keymap_config, scratch, start_files, typ, type_text, write};
use nsh_app_editor::app::DocMeta;
use nsh_app_editor::app::review::Hunk;
use nsh_app_editor::exports::language_name;
use nsh_app_editor::services::lsp::Diag;
use nsh_testing::Headless;

const WAIT: Duration = Duration::from_secs(5);

/// What the view `sel` reads: its label, else its texts.
fn name(s: &nsh_testing::Screen, sel: &str) -> Option<String> {
    s.accessibility(Some(sel)).ok().flatten().map(|n| n.name)
}

/// Waits until the view `sel` reads `want`.
fn wait_text(app: &mut Headless, sel: &str, want: &str) -> Result<(), String> {
    app.wait_until(&format!("`{sel}` reads `{want}`"), WAIT, |s| name(s, sel).as_deref() == Some(want))
}

/// The background of the view `sel`.
fn background(app: &Headless, sel: &str) -> Option<u32> {
    app.screen().view_style(sel).unwrap().and_then(|s| s.background_color)
}

#[test]
fn the_status_line_counts_diagnostics_and_outside_changes() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.rs", "fn a() {}\n");
    let mut f = flow(&a, None);
    wait_text(&mut f.ui, "id:language", "Rust")?;
    wait_text(&mut f.ui, "id:encoding", "UTF-8 · LF")?;
    assert!(!f.ui.screen().matches("id:errors")?, "no errors, no count");
    assert!(!f.ui.screen().matches("id:outside")?);
    let path = a.clone();
    f.update(move |app, _| {
        let diag = |severity| Diag { line: 0, col: 0, end_line: 0, end_col: 1, severity, message: String::new() };
        let meta = app.meta.entry(path.clone()).or_insert_with(DocMeta::default);
        meta.diags = vec![diag(1), diag(1), diag(2)];
        app.outside.push(Hunk { path, line: 0, added: 1, removed: 0, preview: String::new(), at: 0.0 });
    });
    wait_text(&mut f.ui, "id:errors", "E 2")?;
    wait_text(&mut f.ui, "id:warnings", "W 1")?;
    wait_text(&mut f.ui, "id:outside", "1")?;
    assert!(f.ui.screen().matches("id:outside-icon")?, "an icon goes before the count");
    Ok(())
}

#[test]
fn the_position_takes_the_colour_of_the_mode() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], None);
    wait_text(&mut app, "id:position", "1:1")?;
    let normal = background(&app, "id:position-fill");
    assert_eq!(normal, background(&app, "id:mode-badge"));
    typ(&mut app, &["i"]);
    wait_text(&mut app, "id:mode", "INSERT")?;
    let insert = background(&app, "id:mode-badge");
    app.wait_until("the position takes the insert colour", WAIT, |s| {
        s.view_style("id:position-fill").unwrap().and_then(|v| v.background_color) == insert
    })?;
    assert_ne!(insert, normal);
    Ok(())
}

#[test]
fn the_status_line_text_follows_the_configured_size() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let config = format!("{}[ui]\nsize = 15.0\n", keymap_config("vim"));
    let mut app = start_files(&[&a], Some(&config));
    wait_text(&mut app, "id:file", "a.txt")?;
    app.wait_until("the status text is half a point under 15", WAIT, |s| {
        s.span_style("id:status-line >> text:a.txt").is_ok_and(|t| (t.font_size.0 - 14.5).abs() < 0.01)
    })?;
    let cmd = app.screen().span_style("id:command-line >> text:/1L/")?;
    assert!((cmd.font_size.0 - 12.5).abs() < 0.01, "the command line keeps its size: {}", cmd.font_size.0);
    Ok(())
}

#[test]
fn the_write_button_writes_config_and_gives_the_keys_back() -> Result<(), String> {
    isolate_home();
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut f = flow(&a, None);
    type_text(&mut f.ui, ":Settings\n");
    wait_text(&mut f.ui, "id:mode", "SETTINGS")?;
    // The theme mode starts with a capital letter: `System · Tokyo Night`.
    f.ui.wait_for("id:status-line >> text:/^[A-Z][a-z]+ · /")?;
    f.update(|app, _| app.cfg.ui.size += 1.0);
    wait_text(&mut f.ui, "id:file", "config.toml [+]")?;
    f.ui.press("label:Write config.toml")?;
    wait_text(&mut f.ui, "id:file", "config.toml")?;
    assert!(f.check(|app| app.cfg == app.disk_cfg), "the settings are written");
    // The press gives the keys back to the editor.
    type_text(&mut f.ui, ":");
    wait_text(&mut f.ui, "id:command-input", ":")?;
    Ok(())
}

#[test]
fn the_keys_button_gives_the_keys_back() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], None);
    app.wait_for("label:Show the keys")?;
    app.press("label:Show the keys")?;
    wait_text(&mut app, "id:pending", "<Space>")?;
    typ(&mut app, &["escape"]);
    app.wait_for("label:Show the keys")?;
    typ(&mut app, &["i"]);
    wait_text(&mut app, "id:mode", "INSERT")?;
    Ok(())
}

#[test]
fn a_search_shows_its_prompt_and_its_text() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], None);
    wait_text(&mut app, "id:position", "1:1")?;
    type_text(&mut app, "/alp");
    wait_text(&mut app, "id:command-input", "/alp")?;
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:command-caret", WAIT)?;
    type_text(&mut app, "?ph");
    wait_text(&mut app, "id:command-input", "?ph")?;
    Ok(())
}

#[test]
fn the_status_line_names_languages_and_line_endings_as_people_write_them() -> Result<(), String> {
    assert_eq!(language_name("cpp"), "C++");
    assert_eq!(language_name("javascript"), "JavaScript");
    assert_eq!(language_name("text"), "Plain text");
    let dir = scratch();
    let a = write(dir.path(), "a.md", "# title\r\nbody\r\n");
    let mut app = start_files(&[&a], None);
    wait_text(&mut app, "id:language", "Markdown")?;
    wait_text(&mut app, "id:encoding", "UTF-8 · CRLF")?;
    Ok(())
}
