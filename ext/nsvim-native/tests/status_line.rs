//! The status line and the command line in the window (`src/status-line.tsx` and
//! `src/command-line.tsx`): what Rust gives them shows, and their presses act.

mod common;

use std::time::Duration;

use common::{flow, keymap_config, scratch, start_files, typ, type_text, write};
use nsh_testing::Headless;
use nsvim_native::app::DocMeta;
use nsvim_native::app::review::Hunk;
use nsvim_native::exports::language_name;
use nsvim_native::services::lsp::Diag;

const WAIT: Duration = Duration::from_secs(5);

/// What the view `sel` reads: its label, else its texts.
fn name(s: &nsh_testing::Screen, sel: &str) -> Option<String> {
    s.accessibility(Some(sel)).ok().flatten().map(|n| n.name)
}

/// Waits until the view `sel` reads `want`.
fn wait_text(app: &mut Headless, sel: &str, want: &str) -> Result<(), String> {
    app.wait_until(&format!("`{sel}` reads `{want}`"), WAIT, |s| name(s, sel).as_deref() == Some(want))
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
fn the_mode_badge_is_not_bold() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], None);
    wait_text(&mut app, "id:mode", "NORMAL")?;
    // Capitals on a strong fill are enough. Bold on top of them is too loud.
    let style = app.screen().span_style("id:mode-badge >> text:NORMAL")?;
    assert!(style.font_weight < 700, "weight {}", style.font_weight);
    assert!(style.letter_spacing.0 > 0.0, "spacing {}", style.letter_spacing.0);
    Ok(())
}

/// Waits until the view `sel` reads `want`, after the figure spaces that pad it.
fn wait_padded(app: &mut Headless, sel: &str, want: &str) -> Result<(), String> {
    app.wait_until(&format!("`{sel}` reads `{want}`"), WAIT, |s| name(s, sel).is_some_and(|n| n.trim_start() == want))
}

/// The width of the view `sel`.
fn width(app: &Headless, sel: &str) -> f64 {
    app.screen().layout(sel).unwrap().unwrap().bounds.size.width.0 as f64
}

#[test]
fn the_position_is_quiet_text_like_the_other_parts() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], None);
    wait_padded(&mut app, "id:position", "1:1")?;
    assert!(!app.screen().matches("id:position-fill")?, "no fill behind the position");
    let text = |app: &Headless, sel: &str| app.screen().span_style(&format!("id:status-line >> text:{sel}")).unwrap();
    let lang_name = name(app.screen(), "id:language").unwrap().trim().to_string();
    let (pos, lang) = (text(&app, "/1:1/"), text(&app, &lang_name));
    assert_eq!(pos.color, lang.color, "the position takes the colour of the language");
    assert_eq!(pos.font_weight, lang.font_weight, "and its weight");
    typ(&mut app, &["i"]);
    wait_text(&mut app, "id:mode", "INSERT")?;
    assert_eq!(text(&app, "/1:1/").color, lang.color, "the mode does not colour the position");
    Ok(())
}

#[test]
fn the_position_keeps_its_width_when_the_column_gets_a_digit() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "short\na line with more than ten characters\n");
    let mut app = start_files(&[&a], None);
    wait_padded(&mut app, "id:position", "1:1")?;
    typ(&mut app, &["shift+4"]);
    wait_padded(&mut app, "id:position", "1:5")?;
    let narrow = (width(&app, "id:position"), width(&app, "id:language"));
    type_text(&mut app, "j");
    wait_padded(&mut app, "id:position", "2:5")?;
    typ(&mut app, &["shift+4"]);
    wait_padded(&mut app, "id:position", "2:38")?;
    let wide = (width(&app, "id:position"), width(&app, "id:language"));
    assert!((narrow.0 - wide.0).abs() < 0.5, "the position keeps its width: {narrow:?} then {wide:?}");
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
    wait_padded(&mut app, "id:position", "1:1")?;
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

#[test]
fn the_status_line_names_toml_and_lock_files() -> Result<(), String> {
    assert_eq!(language_name("toml"), "TOML");
    assert_eq!(language_name("yaml"), "YAML");
    assert_eq!(language_name("makefile"), "Makefile");
    assert_eq!(language_name("dockerfile"), "Dockerfile");
    let dir = scratch();
    let lock = write(dir.path(), "Cargo.lock", "[[package]]\nname = \"a\"\n");
    let mut app = start_files(&[&lock], None);
    wait_text(&mut app, "id:language", "TOML")?;
    Ok(())
}
