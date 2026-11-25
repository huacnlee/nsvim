//! The terminal editor (`nsvim` in a terminal), headless: the same App.tsx and Rust state, with one row
//! of code per terminal cell and icons drawn as text. A test binary of its own, because the
//! terminal mode is set for the whole process.

mod common;

use common::{Flow, flow, scratch, start, write};
use editor::editor::{KeyCode, KeyInput};

/// Sends `keys` to the app's key handler. The headless engine of a terminal does not route
/// its keys to the editor's key view, so the tests send them where the terminal host's keys go.
fn keys(f: &mut Flow, keys: &[KeyInput]) {
    for &k in keys {
        f.on_engine(move |s| s.key(k));
    }
    f.ui.catch_up();
}

fn text(t: &str) -> Vec<KeyInput> {
    t.chars().map(|c| if c == '\n' { KeyInput::plain(KeyCode::Enter) } else { KeyInput::ch(c) }).collect()
}

fn terminal() {
    nsvim_native::set_terminal(true);
}

#[test]
fn a_row_of_code_is_one_terminal_cell() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\nbeta\n");
    let mut app = start(&f);
    app.wait_for("text:/beta/")?;
    let h = app.screen().layout("id:row-0")?.ok_or("no row 0")?.bounds.size.height.0;
    assert_eq!(h, nsvim_native::TERMINAL_CELL_H);
    Ok(())
}

#[test]
fn icons_are_text_in_a_terminal() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&f);
    app.wait_for("text:/alpha/")?;
    // The clean tab's mark is an X, drawn as a char.
    app.wait_for("text:×")?;
    Ok(())
}

#[test]
fn edit_and_write_in_a_terminal() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\n");
    let mut app = flow(&f, None);
    keys(&mut app, &text("obeta"));
    keys(&mut app, &[KeyInput::plain(KeyCode::Esc)]);
    keys(&mut app, &text(":w\n"));
    app.ui.wait_until("the file is written", std::time::Duration::from_secs(5), |_| {
        std::fs::read_to_string(&f).is_ok_and(|t| t == "alpha\nbeta\n")
    })?;
    Ok(())
}

/// Whether `selector`'s box starts on a terminal cell and is whole cells wide: a box that ends
/// in half a cell does not cover the code under it. Its height is rows of text and the
/// border's lines, which the terminal host gives a cell each and headless a point.
fn whole_cells(app: &mut nsh_testing::Headless, selector: &str) -> Result<(), String> {
    let b = app.screen().layout(selector)?.ok_or(format!("no {selector}"))?.bounds;
    let (x, y, w, h) = (b.origin.x.0, b.origin.y.0, b.size.width.0, b.size.height.0);
    for (what, v, cell) in [("x", x, 8.0), ("y", y, 16.0), ("width", w, 8.0)] {
        if v % cell != 0.0 {
            return Err(format!("{selector}: {what} {v} is not whole cells ({x}, {y}, {w} x {h})"));
        }
    }
    Ok(())
}

#[test]
fn the_finder_covers_the_editor_in_whole_cells() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\n");
    let mut app = flow(&f, None);
    keys(&mut app, &text(" f"));
    app.ui.wait_for("id:picker")?;
    whole_cells(&mut app.ui, "id:picker")?;
    whole_cells(&mut app.ui, "id:picker-preview")?;
    // Edge to edge: no column of code shows beside it.
    let w = app.ui.screen().layout("id:picker")?.ok_or("no picker")?.bounds.size.width.0;
    assert_eq!(w, 880.0);
    Ok(())
}

#[test]
fn the_leader_menu_is_whole_cells() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\n");
    let mut app = flow(&f, None);
    keys(&mut app, &text(" "));
    app.ui.wait_for("id:leader")?;
    // It stands on the status line: its bottom edge, its left and its width are whole cells.
    let b = app.ui.screen().layout("id:leader")?.ok_or("no leader")?.bounds;
    let bottom = b.origin.y.0 + b.size.height.0;
    assert_eq!((b.origin.x.0 % 8.0, b.size.width.0 % 8.0, bottom % 16.0), (0.0, 0.0, 0.0), "{b:?}");
    Ok(())
}

#[test]
fn a_dirty_tab_marks_itself_with_a_char() -> Result<(), String> {
    terminal();
    let dir = scratch();
    let f = write(dir.path(), "a.txt", "alpha\n");
    let mut app = flow(&f, None);
    keys(&mut app, &text("x"));
    app.ui.wait_for("text:●")?;
    // The close button keeps its mark, never its label as text.
    assert!(app.ui.screen().layout("text:Close")?.is_none());
    Ok(())
}
