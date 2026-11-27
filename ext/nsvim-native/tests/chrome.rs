//! The chrome, headless: the buffer line and its tabs, the status line, the command line and
//! the file tree.

mod common;

use std::path::Path;
use std::time::Duration;

use common::{scratch, start, start_files, typ, type_text, write};
use nsh_engine::{Point, Points};
use nsh_testing::{DragOptions, Headless};
use nsvim_native::app::{App, DocMeta};
use nsvim_native::config::Config;
use nsvim_native::exports::{KeyHint, MessageTone, ModeColor, command_of, position_of, status_of};
use nsvim_native::services::lsp::Diag;

const WAIT: Duration = Duration::from_secs(5);

/// The background of the view `sel`.
fn background(app: &Headless, sel: &str) -> Option<u32> {
    app.screen().view_style(sel).unwrap().and_then(|s| s.background_color)
}

/// What the view `sel` reads: its label, else its texts.
fn name(s: &nsh_testing::Screen, sel: &str) -> Option<String> {
    s.accessibility(Some(sel)).ok().flatten().map(|n| n.name)
}

/// Waits until the view `sel` reads `want`.
fn wait_text(app: &mut Headless, sel: &str, want: &str) -> Result<(), String> {
    app.wait_until(&format!("`{sel}` reads `{want}`"), WAIT, |s| name(s, sel).as_deref() == Some(want))
}

/// Whether a view matches `sel`.
fn has(app: &Headless, sel: &str) -> bool {
    app.screen().matches(sel).unwrap()
}

/// The width of the view `sel`.
fn width(app: &Headless, sel: &str) -> f64 {
    app.screen().layout(sel).unwrap().map(|l| l.bounds.size.width.0 as f64).unwrap_or(0.0)
}

#[test]
fn tabs_show_each_file_and_a_click_switches() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    assert_eq!(app.screen().count("role:tab")?, 2, "{:?}", app.screen().texts());
    assert!(app.screen().selected("role:tab[b.txt]")?);
    assert!(!app.screen().selected("role:tab[a.txt]")?);
    app.press("role:tab[a.txt]")?;
    app.wait_for("text:/alpha/")?;
    assert!(app.screen().selected("role:tab[a.txt]")?);
    assert!(!app.screen().selected("role:tab[b.txt]")?);
    wait_text(&mut app, "id:file", "a.txt")?;
    Ok(())
}

#[test]
fn the_buffer_line_shows_the_project_folder_over_the_tree() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    let name = dir.path().file_name().unwrap().to_string_lossy().into_owned();
    app.wait_until("the project folder", WAIT, |s| s.texts().iter().any(|t| t.ends_with(&name)))?;
    assert!((width(&app, "id:project") - width(&app, "id:tree")).abs() < 1.0, "the folder is as wide as the tree");
    type_text(&mut app, ":Tree\n");
    app.wait_for_gone("id:project", WAIT)?;
    Ok(())
}

/// There is no title bar row: the tab bar is the top of the window, and the project folder over
/// the tree is in line with it.
#[test]
fn the_tabs_are_the_top_of_the_window() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    app.wait_for("id:project")?;
    let top = |app: &Headless, sel: &str| app.screen().layout(sel).unwrap().map(|l| l.bounds.origin.y.0);
    assert_eq!(top(&app, "role:tablist"), Some(0.0), "the tab bar is at the top");
    assert_eq!(top(&app, "id:project"), Some(0.0), "the project folder is at the top");
    // The text starts right under the tab bar.
    let text = app.screen().layout("id:text-area").unwrap().ok_or("no text area")?.bounds;
    assert_eq!(text.origin.y.0, 36.0, "the text is under the tab bar");
    type_text(&mut app, ":Tree\n");
    app.wait_for_gone("id:project", WAIT)?;
    assert_eq!(top(&app, "role:tablist"), Some(0.0), "the tab bar stays at the top without the tree");
    Ok(())
}

/// The tab that shows has a raised background. The other tabs show the background of
/// the line.
#[test]
fn the_tab_that_shows_has_its_own_background() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    let line = background(&app, "role:tablist");
    assert!(line.is_some(), "the line has a background");
    app.wait_until("the tab of b.txt has its own background", WAIT, |s| {
        let bg = |sel: &str| s.view_style(sel).unwrap().and_then(|st| st.background_color);
        bg("role:tab[b.txt]").is_some() && bg("role:tab[b.txt]") != line && bg("role:tab[a.txt]") != bg("role:tab[b.txt]")
    })?;
    let on = background(&app, "role:tab[b.txt]");
    assert_eq!(background(&app, "role:tab[a.txt]"), None, "the other tab shows the line's background");
    app.press("role:tab[a.txt]")?;
    app.wait_until("the background goes to the tab of a.txt", WAIT, |s| {
        s.view_style("role:tab[a.txt]").unwrap().and_then(|st| st.background_color) == on
    })?;
    assert_eq!(background(&app, "role:tab[b.txt]"), None, "the other tab loses the background");
    Ok(())
}

#[test]
fn a_dirty_file_shows_a_dot_on_its_tab_and_a_plus_in_the_status_line() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    wait_text(&mut app, "id:tab-mark-0", "Close")?;
    assert!(app.shows("a.txt"));
    typ(&mut app, &["x"]);
    wait_text(&mut app, "id:tab-mark-0", "Unsaved changes")?;
    app.wait_for("text:a.txt [+]")?;
    typ(&mut app, &["u"]);
    wait_text(&mut app, "id:tab-mark-0", "Close")?;
    Ok(())
}

#[test]
fn the_close_button_closes_a_clean_tab_and_asks_about_a_dirty_one() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    typ(&mut app, &["x"]);
    wait_text(&mut app, "id:tab-mark-1", "Unsaved changes")?;
    app.answer_dialogs_myself();
    app.press("id:tab-close-1")?;
    let d = app.pending_dialog().ok_or("a dirty tab asks in a dialog")?;
    assert_eq!(d.title.as_deref(), Some("Save changes to \"b.txt\"?"));
    app.answer_dialog(nsh_testing::DialogAnswer::Press("Cancel".into()))?;
    assert!(has(&app, "role:tab[b.txt]"), "cancel keeps the dirty tab open");
    app.press("id:tab-close-0")?;
    app.wait_for_gone("role:tab[a.txt]", WAIT)?;
    assert_eq!(app.screen().count("role:tab")?, 1);
    app.press("id:tab-close-0")?;
    app.answer_dialog(nsh_testing::DialogAnswer::Press("Don't save".into()))?;
    app.wait_for_gone("role:tab[b.txt]", WAIT)?;
    assert_eq!(std::fs::read_to_string(&b).unwrap(), "beta\n", "don't save writes nothing");
    Ok(())
}

#[test]
fn the_status_line_follows_the_mode() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    wait_text(&mut app, "id:mode", "NORMAL")?;
    assert_eq!(name(app.screen(), "id:language").as_deref(), Some("Plain text"));
    assert_eq!(name(app.screen(), "id:position").as_deref(), Some("1:1"));
    let normal = background(&app, "id:mode-badge");
    typ(&mut app, &["i"]);
    wait_text(&mut app, "id:mode", "INSERT")?;
    wait_text(&mut app, "id:message", "-- INSERT --")?;
    assert_ne!(background(&app, "id:mode-badge"), normal, "insert has its own colour");
    typ(&mut app, &["escape", "l"]);
    wait_text(&mut app, "id:position", "1:2")?;
    typ(&mut app, &["shift+v"]);
    wait_text(&mut app, "id:mode", "V-LINE")?;
    wait_text(&mut app, "id:message", "-- VISUAL LINE --")?;
    Ok(())
}

#[test]
fn the_classic_keymap_shows_no_mode_and_a_long_position() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start_files(&[&a], Some("[keys]\nmap = \"classic\"\n[ipc]\nenabled = false\n"));
    app.wait_for("text:/alpha/")?;
    wait_text(&mut app, "id:position", "Ln 1, Col 1")?;
    assert!(!has(&app, "id:mode"));
    assert!(!has(&app, "id:command-line"), "the classic keymap has no command line until it has something to say");
    Ok(())
}

#[test]
fn the_command_line_shows_what_is_typed_with_a_caret() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    type_text(&mut app, ":set");
    wait_text(&mut app, "id:command-input", ":set")?;
    assert!(has(&app, "id:command-caret"));
    wait_text(&mut app, "id:mode", "COMMAND")?;
    typ(&mut app, &["left"]);
    wait_text(&mut app, "id:command-input", ":se")?;
    wait_text(&mut app, "id:command-after", "t")?;
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:command-caret", WAIT)?;
    wait_text(&mut app, "id:mode", "NORMAL")?;
    Ok(())
}

#[test]
fn an_unknown_command_shows_an_error() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    app.wait_for("id:message")?;
    type_text(&mut app, ":nosuchcommand\n");
    app.wait_until("the error", WAIT, |s| name(s, "id:error").is_some_and(|n| n.contains("nosuchcommand")))?;
    assert!(!has(&app, "id:message"), "{:?}", app.screen().texts());
    Ok(())
}

#[test]
fn the_tree_opens_and_closes_a_folder() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    write(dir.path(), "src/lib.rs", "pub fn x() {}\n");
    let mut app = start(&a);
    app.wait_for("role:treeitem[src]")?;
    assert!(has(&app, "role:treeitem[src] >> id:tree-closed"), "a closed folder shows a right chevron");
    assert!(!has(&app, "role:treeitem[a.txt] >> id:tree-closed"), "a file shows no chevron");
    assert!(!has(&app, "role:treeitem[lib.rs]"));
    app.press("role:treeitem[src]")?;
    app.wait_for("role:treeitem[lib.rs]")?;
    assert!(has(&app, "role:treeitem[src] >> id:tree-open"), "an open folder shows a down chevron");
    assert!(!has(&app, "role:treeitem[src] >> id:tree-closed"));
    assert!(app.screen().accessibility_state("role:treeitem[src]")?.expanded == Some(true));
    app.press("role:treeitem[src]")?;
    app.wait_for_gone("role:treeitem[lib.rs]", WAIT)?;
    Ok(())
}

#[test]
fn a_click_on_a_file_in_the_tree_opens_it() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    write(dir.path(), "notes.md", "# notes\n");
    let mut app = start(&a);
    app.wait_for("role:treeitem[notes.md]")?;
    app.wait_until("the open file shows in the tree", WAIT, |s| s.selected("role:treeitem[a.txt]").unwrap_or(false))?;
    app.press("role:treeitem[notes.md]")?;
    app.wait_for("text:/# notes/")?;
    app.wait_for("role:tab[notes.md]")?;
    assert!(app.screen().selected("role:tab[notes.md]")?);
    app.wait_until("the tree marks the new file", WAIT, |s| s.selected("role:treeitem[notes.md]").unwrap_or(false))?;
    // The click gives the keys back to the editor.
    typ(&mut app, &["i"]);
    wait_text(&mut app, "id:mode", "INSERT")?;
    Ok(())
}

#[test]
fn keys_in_the_tree_open_a_file() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "notes.txt", "notes\n");
    write(dir.path(), "src/lib.rs", "pub fn lib() {}\n");
    let mut app = start(&a);
    app.wait_for("role:treeitem[src]")?;
    typ(&mut app, &["space", "e"]);
    app.wait_until("the keyboard row", WAIT, |s| s.selected("role:treeitem[notes.txt]").unwrap_or(false))?;
    typ(&mut app, &["k"]);
    app.wait_until("the keyboard moves up", WAIT, |s| s.selected("role:treeitem[src]").unwrap_or(false))?;
    typ(&mut app, &["l"]);
    app.wait_for("role:treeitem[lib.rs]")?;
    typ(&mut app, &["j", "enter"]);
    app.wait_for("text:/pub fn lib/")?;
    app.wait_for("role:tab[lib.rs]")?;
    Ok(())
}

#[test]
fn the_wheel_scrolls_the_tree() -> Result<(), String> {
    let dir = scratch();
    let mut files = Vec::new();
    for i in 0..80 {
        files.push(write(dir.path(), &format!("file{i:02}.txt"), "x\n"));
    }
    let mut app = start(&files[0]);
    app.wait_for("role:treeitem[file00.txt]")?;
    assert!(!has(&app, "role:treeitem[file79.txt]"));
    // The file list and the reveal of the open file can come after the first row shows. A long
    // turn clamps at the end, so turn until each end shows.
    wheel_until(&mut app, 22.0 * 200.0, "role:treeitem[file79.txt]")?;
    wheel_until(&mut app, -22.0 * 200.0, "role:treeitem[file00.txt]")?;
    // Now the tree is still: ten rows of wheel scroll ten rows.
    app.wheel("id:tree", Point::new(Points(0.0), Points(22.0 * 10.0)))?;
    app.wait_for_gone("role:treeitem[file00.txt]", WAIT)?;
    app.wait_for("role:treeitem[file10.txt]")?;
    Ok(())
}

/// Turns the wheel over the tree by `dy` until `sel` shows.
fn wheel_until(app: &mut Headless, dy: f32, sel: &str) -> Result<(), String> {
    let end = std::time::Instant::now() + WAIT;
    loop {
        app.wheel("id:tree", Point::new(Points(0.0), Points(dy)))?;
        if app.wait_for_within(sel, std::time::Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        if std::time::Instant::now() > end {
            return Err(format!("{sel} never showed: {:?}", app.screen().texts()));
        }
    }
}

#[test]
fn a_drag_on_the_tree_edge_changes_its_width() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("id:tree")?;
    let before = width(&app, "id:tree");
    assert!((before - 260.0).abs() < 1.0, "{before}");
    app.drag("id:tree-edge", Point::new(Points(60.0), Points(0.0)), DragOptions::default())?;
    app.wait_until("the tree grows", WAIT, |s| {
        s.layout("id:tree").unwrap().is_some_and(|l| (l.bounds.size.width.0 as f64 - 320.0).abs() < 2.0)
    })?;
    Ok(())
}

#[test]
fn the_status_line_shows_the_branch_and_the_tree_shows_git_status() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    write(dir.path(), "src/new.rs", "fn x() {}\n");
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(dir.path()).output().map(|o| o.status.success()).unwrap_or(false)
    };
    let commit = ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", "start"];
    if !git(&["init", "-q", "-b", "trunk"]) || !git(&commit) {
        eprintln!("no git: skipped");
        return Ok(());
    }
    let mut app = start(&a);
    wait_text(&mut app, "id:branch", "trunk")?;
    assert!(has(&app, "id:branch-icon"), "a branch icon goes before the name");
    app.wait_for("role:treeitem[a.txt] >> text:U")?;
    app.wait_for("role:treeitem[src] >> id:tree-changes")?;
    assert!(!has(&app, "role:treeitem[a.txt] >> id:tree-changes"), "a file shows its letter, not the square");
    Ok(())
}

#[test]
fn line_blame_shows_who_changed_the_cursor_line_after_its_text() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\nbeta\n");
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(dir.path()).output().map(|o| o.status.success()).unwrap_or(false)
    };
    let commit = ["-c", "user.name=Ada", "-c", "user.email=a@a", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "Add the words"];
    if !git(&["init", "-q"]) || !git(&["add", "a.txt"]) || !git(&commit) {
        eprintln!("no git: skipped");
        return Ok(());
    }
    let mut app = start(&a);
    app.wait_for("text:alpha")?;
    // Line blame is on by default. Only the cursor's line shows it, with the commit's summary.
    app.wait_for("text:/Ada, (just now|1 minute ago) · Add the words$/")?;
    assert!(!has(&app, "text:/Ada, (just now|1 minute ago)$/"), "the other lines show no blame");
    type_text(&mut app, " gb");
    app.wait_until("the blame hides", WAIT, |s| !s.matches("text:/Add the words$/").unwrap())?;
    Ok(())
}

fn app_with_file(root: &Path, text: &str) -> App {
    let mut app = App::new(Config::default(), root.to_path_buf(), false, None);
    let mut buf = editor::buffer::Buffer::from_str(text);
    buf.path = Some(root.join("a.rs"));
    app.ed.docs.push(editor::editor::Doc::new(buf));
    app.ed.active = 0;
    app.ed.rows = 10;
    app
}

#[test]
fn the_status_line_counts_errors_and_warnings_and_shows_the_scroll() {
    let dir = scratch();
    let mut app = app_with_file(dir.path(), &"x\n".repeat(100));
    let diag = |severity| Diag { line: 0, col: 0, end_line: 0, end_col: 1, severity, message: String::new() };
    let meta = DocMeta { diags: vec![diag(1), diag(1), diag(2), diag(3)], ..DocMeta::default() };
    app.meta.insert(dir.path().join("a.rs"), meta);
    let st = status_of(&app);
    assert_eq!((st.errors, st.warnings), (2, 1));
    assert_eq!(st.file, "a.rs");
    assert_eq!(position_of(&app).scroll, "Top");
    assert_eq!((st.badge.as_deref(), st.mode_color), (Some("NORMAL"), ModeColor::Primary));
    app.ed.doc_mut().view.cursor.line = 49;
    assert_eq!(position_of(&app).scroll, "50%");
    app.ed.doc_mut().view.cursor.line = 5;
    assert_eq!(position_of(&app).scroll, "\u{2007}6%", "a figure space keeps the percent three wide");
    app.ed.doc_mut().view.cursor.line = 100;
    assert_eq!(position_of(&app).scroll, "Bot");
}

/// Figure spaces pad the position to the longest line number of the file and three digits of
/// column, so it keeps its width while the cursor moves.
#[test]
fn the_position_keeps_one_width_as_the_cursor_moves() {
    let dir = scratch();
    let mut app = app_with_file(dir.path(), &"x\n".repeat(120));
    let fs = '\u{2007}';
    let at = |app: &mut App, line: usize, col: usize| {
        app.ed.doc_mut().view.cursor.line = line;
        app.ed.doc_mut().view.cursor.col = col;
        position_of(app).pos
    };
    // 121 lines: three digits of line, three of column, and the colon.
    assert_eq!(at(&mut app, 0, 0), format!("{fs}{fs}{fs}{fs}1:1"));
    assert_eq!(at(&mut app, 0, 9), format!("{fs}{fs}{fs}1:10"));
    assert_eq!(at(&mut app, 99, 99), "100:100");
    for (line, col) in [(0, 0), (8, 8), (9, 9), (50, 120), (120, 998)] {
        assert_eq!(at(&mut app, line, col).chars().count(), 7, "at {line}:{col}");
    }
    assert_eq!(at(&mut app, 0, 999), format!("{fs}{fs}1:1000"), "past column 999 it grows");
}

#[test]
fn the_command_line_shows_the_file_size_and_the_leader_hint() {
    let dir = scratch();
    let app = app_with_file(dir.path(), "one\ntwo\n");
    let c = command_of(&app);
    assert!(c.shows);
    assert_eq!(c.text, "\"a.rs\" 2L, 8B");
    assert_eq!(c.tone, MessageTone::Quiet);
    assert_eq!(c.hint, KeyHint::Keys);
}

#[test]
fn the_command_line_shows_pending_keys_and_a_button_for_the_keys() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&a);
    app.wait_for("text:/alpha/")?;
    typ(&mut app, &["d"]);
    wait_text(&mut app, "id:pending", "d")?;
    typ(&mut app, &["escape"]);
    app.wait_for("label:Show the keys")?;
    app.press("label:Show the keys")?;
    wait_text(&mut app, "id:pending", "<Space>")?;
    Ok(())
}

/// The close button is a small square by the mark, not a strip as tall as the tab: a press on
/// the tab above or below it shows the tab and leaves it open.
#[test]
fn the_close_button_takes_only_its_square() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta/")?;
    let bounds = |app: &Headless, sel: &str| app.screen().layout(sel).unwrap().unwrap().bounds;
    let (tab, close) = (bounds(&app, "id:tab-0"), bounds(&app, "id:tab-close-0"));
    assert!(close.size.height.0 < tab.size.height.0 - 8.0, "close {close:?} in tab {tab:?}");
    assert!(close.size.width.0 <= close.size.height.0 + 0.5, "the close button is square: {close:?}");
    // Just above the close button, inside the tab.
    let at = Point::new(Points(close.origin.x.0 - tab.origin.x.0 + close.size.width.0 / 2.0), Points(1.0));
    app.press_at("id:tab-0", at)?;
    assert_eq!(app.screen().count("role:tab")?, 2, "a press above the close button does not close the tab");
    Ok(())
}

/// The bounds of the view with test ID `id`, in the root's points, scroll offsets applied.
fn page_bounds(app: &Headless, id: &str) -> Option<nsh_engine::Bounds<Points>> {
    app.screen().layout_nodes().into_iter().find(|n| n.test_id.as_deref() == Some(id)).map(|n| n.bounds)
}

/// Whether the view `id` lies across the pane, whose tab bar scrolls its tabs.
fn tab_in_sight(app: &Headless, id: &str) -> bool {
    let (Some(line), Some(tab)) = (page_bounds(app, "text-area"), page_bounds(app, id)) else { return false };
    let (l0, l1) = (line.origin.x.0, line.origin.x.0 + line.size.width.0);
    let (t0, t1) = (tab.origin.x.0, tab.origin.x.0 + tab.size.width.0);
    t0 >= l0 - 0.5 && t1 <= l1 + 0.5
}

/// With more tabs than fit, the pane's tab bar scrolls sideways and keeps the tab that shows in
/// sight, and the tabs stay inside the window.
#[test]
fn many_tabs_scroll_and_keep_the_one_that_shows_in_sight() -> Result<(), String> {
    let dir = scratch();
    let names: Vec<String> = (0..30).map(|i| format!("a_long_file_name_{i:02}.txt")).collect();
    let paths: Vec<_> = names.iter().map(|n| write(dir.path(), n, &format!("text {n}\n"))).collect();
    let refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();
    let mut app = start_files(&refs, None);
    app.wait_for("text:/text a_long_file_name_29/")?;
    let line = app.screen().layout("role:tablist").unwrap().ok_or("no tab bar")?.bounds;
    let tabs = page_bounds(&app, "text-area").ok_or("no pane")?;
    assert!(tabs.origin.x.0 + tabs.size.width.0 <= line.origin.x.0 + line.size.width.0 + 0.5, "tabs {tabs:?} in {line:?}");
    let _ = app.wait_until("the last tab is in sight", WAIT, |_| false);
    assert!(tab_in_sight(&app, "tab-29"), "tab {:?} in {tabs:?}", page_bounds(&app, "tab-29"));
    assert!(!tab_in_sight(&app, "tab-0"), "the first tab scrolled away");
    // Back to the first tab (as a key or the tree would): the line scrolls back to it.
    app.press("id:tab-0")?;
    app.wait_for("text:/text a_long_file_name_00/")?;
    let _ = app.wait_until("settle", Duration::from_millis(300), |_| false);
    assert!(tab_in_sight(&app, "tab-0"), "tab {:?} in {tabs:?}", page_bounds(&app, "tab-0"));
    Ok(())
}

/// A tab dragged along the tab bar moves to where it drops, and it shows: the panes are NSH's
/// `DockArea`. A short drag that does not pass the middle of the
/// next tab leaves the order as it was.
#[test]
fn a_dragged_tab_moves_to_where_it_drops() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let c = write(dir.path(), "c.txt", "gamma\n");
    let mut app = start_files(&[&a, &b, &c], None);
    app.wait_for("text:/gamma/")?;
    let names = |app: &Headless| (0..3).map(|i| name(app.screen(), &format!("id:tab-{i}")).unwrap_or_default()).collect::<Vec<_>>();
    assert_eq!(names(&app), ["a.txt", "b.txt", "c.txt"]);
    // Past the middle of the last tab: a.txt goes to the end.
    let by = width(&app, "id:tab-0") / 2.0 + width(&app, "id:tab-1") + width(&app, "id:tab-2") * 0.75;
    app.drag("id:tab-0", Point::new(Points(by as f32), Points(0.0)), DragOptions::default())?;
    app.wait_until("a.txt at the end", WAIT, |s| name(s, "id:tab-2").as_deref() == Some("a.txt"))?;
    assert_eq!(names(&app), ["b.txt", "c.txt", "a.txt"]);
    // As in NSH's `DockArea`, the dropped tab shows, and its file has the keys.
    app.wait_until("a.txt shows", WAIT, |s| s.selected("role:tab[a.txt]").unwrap_or(false))?;
    wait_text(&mut app, "id:file", "a.txt")?;
    assert!(!has(&app, "id:tab-drop"), "the drop indicator goes with the drop");
    // Back to the start.
    let by = width(&app, "id:tab-2") / 2.0 + width(&app, "id:tab-1") + width(&app, "id:tab-0") * 0.75;
    app.drag("id:tab-2", Point::new(Points(-by as f32), Points(0.0)), DragOptions::default())?;
    app.wait_until("a.txt at the start", WAIT, |s| name(s, "id:tab-0").as_deref() == Some("a.txt"))?;
    assert_eq!(names(&app), ["a.txt", "b.txt", "c.txt"]);
    // A short drag keeps the order. It ends on the tab, so it shows the tab, as a click does.
    app.drag("id:tab-0", Point::new(Points(10.0), Points(0.0)), DragOptions::default())?;
    app.wait_until("a.txt shows", WAIT, |s| s.selected("role:tab[a.txt]").unwrap_or(false))?;
    assert_eq!(names(&app), ["a.txt", "b.txt", "c.txt"]);
    Ok(())
}
