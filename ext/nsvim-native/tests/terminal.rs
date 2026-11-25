//! The terminal editor (`nsvim --tui`), headless: the same App.tsx and Rust state, with one row
//! of code per terminal cell and icons drawn as text. A test binary of its own, because the
//! terminal mode is set for the whole process.

mod common;

use common::{scratch, start, typ, type_text, write};

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
    let mut app = start(&f);
    app.wait_for("text:/alpha/")?;
    typ(&mut app, &["o"]);
    type_text(&mut app, "beta");
    typ(&mut app, &["escape"]);
    type_text(&mut app, ":w");
    typ(&mut app, &["enter"]);
    app.wait_until("the file is written", std::time::Duration::from_secs(5), |_| {
        std::fs::read_to_string(&f).is_ok_and(|t| t == "alpha\nbeta\n")
    })?;
    Ok(())
}
