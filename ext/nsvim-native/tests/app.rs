//! The real app, headless: keys go through NSH's focus and key capture to the app, and file
//! reads and writes run on the workers.

mod common;

use common::{scratch, start, typ, write};

#[test]
fn a_file_opens_with_its_name_and_language() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "main.rs", "fn main() {\n    println!(\"hi\");\n}\n");
    let mut app = start(&file);
    app.wait_for("text:/println/")?;
    assert!(app.shows("NORMAL"), "{:?}", app.screen().texts());
    assert!(app.shows("Rust"));
    assert!(app.shows("1:1"));
    Ok(())
}

#[test]
fn keys_move_the_cursor_and_edit_then_w_writes_the_file() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "notes.txt", "one\ntwo\n");
    let mut app = start(&file);
    app.wait_for("text:/two/")?;
    typ(&mut app, &["j", "shift+a"]);
    app.wait_for("INSERT")?;
    typ(&mut app, &["shift+1", "escape"]);
    app.wait_for("text:/two!/")?;
    app.wait_for("NORMAL")?;
    assert!(app.shows("2:4"), "{:?}", app.screen().texts());
    typ(&mut app, &["shift+;", "w", "enter"]);
    app.wait_until("the file written", std::time::Duration::from_secs(5), |_| std::fs::read_to_string(&file).unwrap() == "one\ntwo!\n")?;
    Ok(())
}

#[test]
fn visual_mode_shows_in_the_status_line() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "hello world\n");
    let mut app = start(&file);
    app.wait_for("text:/hello/")?;
    typ(&mut app, &["v", "e"]);
    app.wait_for("VISUAL")?;
    typ(&mut app, &["escape"]);
    app.wait_for("NORMAL")?;
    Ok(())
}
