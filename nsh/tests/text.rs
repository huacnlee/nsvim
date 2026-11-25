//! The text area, headless: search highlights, several cursors, soft wrap, visible whitespace,
//! the mouse (drag, double click, wheel, right click), the completion and hover popups, and
//! the review view of a proposed change.

mod common;

use std::time::Duration;

use common::{CONFIG, scratch, start, start_files, typ, type_text, write};
use nsh_engine::{Point, Points};
use nsh_testing::{DragOptions, Headless};
use serde_json::json;

fn point(x: f32, y: f32) -> Point<Points> {
    Point { x: Points(x), y: Points(y) }
}

/// Whether the text of `row` that reads `text` has a background: selected or matched.
fn marked(app: &Headless, row: usize, text: &str) -> Result<bool, String> {
    Ok(app.screen().span_style(&format!("id:row-{row} >> text:{text}"))?.background.is_some())
}

/// The height of a row of text.
fn row_h(app: &Headless) -> f32 {
    app.screen().layout("id:row-0").unwrap().unwrap().bounds.size.height.0
}

/// The point `dx`, `dy` from the top left of the view `sel` names, in the text area.
fn at(app: &Headless, sel: &str, dx: f32, dy: f32) -> Point<Points> {
    let of = |sel: &str| app.screen().layout(sel).unwrap().unwrap_or_else(|| panic!("{sel} is not on screen")).bounds.origin;
    let (v, area) = (of(sel), of("id:text-area"));
    point(v.x.0 - area.x.0 + dx, v.y.0 - area.y.0 + dy)
}

/// A click at `dx`, `dy` in the view `sel` names.
fn click(app: &mut Headless, sel: &str, dx: f32, dy: f32) -> Result<(), String> {
    let p = at(app, sel, dx, dy);
    app.press_at("id:text-area", p)
}

fn texts(app: &Headless) -> String {
    format!("{:?}", app.screen().texts())
}

#[test]
fn a_search_marks_its_matches_until_nohlsearch() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha beta\ngamma beta\n");
    let mut app = start(&file);
    app.wait_for("text:alpha beta")?;
    assert!(!marked(&app, 1, "beta")?);
    type_text(&mut app, "/beta\n");
    app.wait_until("matches marked", Duration::from_secs(5), |s| {
        s.span_style("id:row-1 >> text:beta").is_ok_and(|st| st.background.is_some())
    })?;
    assert!(marked(&app, 0, "beta")?);
    assert!(!marked(&app, 1, "gamma")?);
    type_text(&mut app, ":noh\n");
    app.wait_until("marks gone", Duration::from_secs(5), |s| {
        s.span_style("id:row-1 >> text:beta").is_ok_and(|st| st.background.is_none())
    })?;
    Ok(())
}

#[test]
fn alt_click_adds_a_cursor_and_typing_edits_at_both() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "one\ntwo\n");
    let mut app = start_files(&[&file], Some(&format!("{CONFIG}[keys]\nmap = \"classic\"\n")));
    app.wait_for("text:two")?;
    app.hold_keys("alt", true)?;
    click(&mut app, "id:row-1 >> text:two", 1.0, 4.0)?;
    app.hold_keys("alt", false)?;
    app.wait_for("id:extra-caret")?;
    type_text(&mut app, "X");
    app.wait_for("text:Xone")?;
    app.wait_for("text:Xtwo")?;
    Ok(())
}

#[test]
fn set_wrap_breaks_a_long_line_into_rows() -> Result<(), String> {
    let dir = scratch();
    let long: String = (0..60).map(|i| format!("w{i:02} ")).collect();
    let file = write(dir.path(), "a.txt", &format!("{long}\nend\n"));
    let mut app = start(&file);
    app.wait_for("text:/^w00 /")?;
    app.wait_for("text:/^~$/")?;
    assert!(!app.screen().texts().iter().any(|t| t.contains("w59")), "the line runs off the right edge: {}", texts(&app));
    type_text(&mut app, ":set wrap\n");
    app.wait_until("the line wraps", Duration::from_secs(5), |s| s.texts().iter().any(|t| t.trim_end().ends_with("w59")))?;
    assert!(!app.shows("w59"), "the end is on a row of its own: {}", texts(&app));
    assert!(app.screen().matches("id:row-1 >> text:/^ ?w\\d\\d /")?, "row 2 goes on with the line: {}", texts(&app));
    Ok(())
}

#[test]
fn whitespace_shows_leading_spaces_and_tabs() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "    four\n\ttab\n");
    let mut app = start_files(&[&file], Some(&format!("{CONFIG}[editor]\nwhitespace = true\n")));
    app.wait_for("text:····four")?;
    app.wait_for("text:/^→ +tab$/")?;
    type_text(&mut app, ":set nolist\n");
    app.wait_for("text:/^ {4}four$/")?;
    Ok(())
}

#[test]
fn the_ruler_shows_at_its_column() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "x\n");
    let mut app = start_files(&[&file], Some(&format!("{CONFIG}[editor]\nruler = 80\n")));
    app.wait_for("id:ruler")?;
    let (ruler, row) = (app.screen().layout("id:ruler")?.unwrap().bounds, app.screen().layout("id:row-0")?.unwrap().bounds);
    assert!(ruler.origin.x.0 > row.origin.x.0 + 40.0, "the ruler is right of the text: {ruler:?}");
    type_text(&mut app, ":set cc=0\n");
    app.wait_for_gone("id:ruler", Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn a_mouse_drag_selects_across_lines() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "first line\nmiddle\nlast line\n");
    let mut app = start(&file);
    app.wait_for("text:middle")?;
    let h = row_h(&app);
    app.drag("id:row-0 >> text:first line", point(0.0, 2.0 * h), DragOptions::default())?;
    app.wait_for("Visual")?;
    assert!(marked(&app, 1, "middle")?, "the middle line is selected: {}", texts(&app));
    assert!(marked(&app, 0, "line")?, "the end of the first line is selected");
    assert!(!marked(&app, 0, "first")?, "the start of the first line is not");
    Ok(())
}

#[test]
fn a_double_click_selects_a_word_and_a_triple_click_the_line() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "hello world\nnext\n");
    let mut app = start(&file);
    app.wait_for("text:hello world")?;
    click(&mut app, "id:row-0 >> text:hello world", 2.0, 4.0)?;
    click(&mut app, "id:row-0 >> text:hello world", 2.0, 4.0)?;
    app.wait_for("Visual")?;
    assert!(marked(&app, 0, "hello")?);
    assert!(!marked(&app, 0, "world")?);
    click(&mut app, "id:row-0 >> text:hello world", 2.0, 4.0)?;
    app.wait_for("Visual line")?;
    assert!(marked(&app, 0, "world")?);
    Ok(())
}

#[test]
fn a_click_moves_the_cursor_and_the_wheel_scrolls() -> Result<(), String> {
    let dir = scratch();
    let text: String = (1..=200).map(|i| format!("line {i}\n")).collect();
    let file = write(dir.path(), "a.txt", &text);
    let mut app = start(&file);
    app.wait_for("text:line 1")?;
    click(&mut app, "id:row-2 >> text:line 3", 1.0, 4.0)?;
    app.wait_for("3:1")?;
    assert!(app.screen().matches("id:row-0 >> text:line 1")?);
    app.wheel("id:text-area", point(0.0, 10.0 * row_h(&app)))?;
    app.wait_until("scrolled down", Duration::from_secs(5), |s| !s.matches("id:row-0 >> text:line 1").unwrap_or(true))?;
    app.wheel("id:text-area", point(0.0, -50.0 * row_h(&app)))?;
    app.wait_until("back at the top", Duration::from_secs(5), |s| s.matches("id:row-0 >> text:line 1").unwrap_or(false))?;
    Ok(())
}

#[test]
fn a_right_click_puts_the_cursor_there() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "one\ntwo\nthree\n");
    let mut app = start_files(&[&file], Some(&format!("{CONFIG}[mouse]\nright_click = true\n")));
    app.wait_for("text:three")?;
    app.right_click("id:row-2 >> text:three")?;
    app.wait_for("3:3")?;
    Ok(())
}

#[test]
fn the_completion_list_shows_and_tab_takes_an_item() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "\n");
    let mut app = start(&file);
    app.wait_for("1:1")?;
    typ(&mut app, &["i"]);
    type_text(&mut app, "pr");
    let items = json!([
        { "label": "println", "kind": "Function", "insert": "println!" },
        { "label": "other", "kind": "Keyword" },
    ]);
    app.send_host_event("editor.lsp", json!({ "kind": "completion", "items": items }))?;
    app.wait_for("id:completion")?;
    assert!(app.shows("println"), "{}", texts(&app));
    assert!(!app.shows("other"), "an item that does not match hides");
    typ(&mut app, &["tab"]);
    app.wait_for_gone("id:completion", Duration::from_secs(5))?;
    app.wait_for("text:println!")?;
    Ok(())
}

#[test]
fn a_click_on_a_completion_item_takes_it() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "\n");
    let mut app = start(&file);
    app.wait_for("1:1")?;
    typ(&mut app, &["i"]);
    type_text(&mut app, "le");
    let items = json!([{ "label": "length", "kind": "Method" }, { "label": "left", "kind": "Field" }]);
    app.send_host_event("editor.lsp", json!({ "kind": "completion", "items": items }))?;
    app.wait_for("id:completion")?;
    app.press("role:option[/^left/]")?;
    app.wait_for("text:left")?;
    app.wait_for("1:5")?;
    app.wait_for_gone("id:completion", Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn hover_text_shows_until_the_next_key() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "value\n");
    let mut app = start(&file);
    app.wait_for("text:value")?;
    app.send_host_event("editor.lsp", json!({ "kind": "hover", "text": "```rust\nfn value() -> u32\n```\nThe value." }))?;
    app.wait_for("id:hover")?;
    assert!(app.shows("fn value() -> u32"), "{}", texts(&app));
    assert!(app.shows("The value."));
    assert!(!app.shows("```rust"), "fences hide");
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:hover", Duration::from_secs(5))?;
    Ok(())
}

#[test]
fn diagnostics_sign_the_gutter_and_show_their_message() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "let x = 1;\nok\n");
    let mut app = start(&file);
    app.wait_for("text:ok")?;
    let diags = json!([{ "line": 0, "col": 4, "end_col": 5, "severity": 1, "message": "unused variable" }]);
    app.send_host_event("editor.lsp", json!({ "kind": "diagnostics", "diags": diags }))?;
    app.wait_for("text:/■ unused variable/")?;
    assert!(app.screen().matches("id:row-0 >> text:E")?, "{}", texts(&app));
    let underline = app.screen().span_style("id:row-0 >> text:x")?.decoration;
    assert_ne!(underline, Default::default(), "the range is underlined");
    Ok(())
}

#[test]
fn a_proposal_shows_as_a_diff_until_y_accepts_it() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "keep\nold line\n");
    let mut app = start(&file);
    app.wait_for("text:old line")?;
    let args = json!({ "path": file, "text": "keep\nnew line\n", "label": "rename" });
    app.send_host_event("editor.request", json!({ "cmd": "propose", "args": args }))?;
    app.wait_for("id:review")?;
    assert!(app.shows("PROPOSAL"), "{}", texts(&app));
    assert!(app.shows("rename"));
    assert!(app.shows("+1") && app.shows("-1"));
    assert!(app.shows("old line") && app.shows("new line"));
    assert!(app.shows("-") && app.shows("+"), "signs mark the lines");
    app.wait_for("text:1 proposal")?;
    assert!(app.screen().matches("id:proposals-icon")?, "an icon goes before the count");
    typ(&mut app, &["y"]);
    app.wait_for_gone("id:review", Duration::from_secs(5))?;
    app.wait_for("text:new line")?;
    assert!(!app.shows("old line"));
    Ok(())
}
