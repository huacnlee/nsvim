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

#[test]
fn an_input_method_types_chinese_in_insert_mode() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "notes.txt", "one\n");
    let mut app = start(&file);
    app.wait_for("text:/one/")?;
    typ(&mut app, &["shift+a"]);
    app.wait_for("INSERT")?;
    // Pinyin composes in the hidden input; only the commit reaches the buffer.
    app.compose("id:text-input", "ni")?;
    app.compose("id:text-input", "你好")?;
    // Nothing is typed while it composes.
    assert!(!app.shows("one你好") && !app.shows("oneni"), "{:?}", app.screen().texts());
    app.type_text("id:text-input", "你好")?;
    app.wait_for("text:/one你好/")?;
    typ(&mut app, &["escape"]);
    app.wait_for("NORMAL")?;
    Ok(())
}

#[test]
fn the_hidden_input_follows_the_cursor_so_candidates_show_by_it() -> Result<(), String> {
    let dir = scratch();
    let file = write(dir.path(), "notes.txt", "one\n");
    let mut app = start(&file);
    app.wait_for("text:/one/")?;
    typ(&mut app, &["shift+a"]);
    app.wait_for("INSERT")?;
    let at = |s: &nsh_testing::Screen| {
        let area = s.layout("id:text-area").ok().flatten()?.bounds;
        let input = s.layout("id:text-input").ok().flatten()?.bounds;
        Some((input.origin.x.0 - area.origin.x.0, input.origin.y.0 - area.origin.y.0, input.size.height.0))
    };
    // After "one", on the first row: past the gutter and three cells, one row tall.
    app.wait_until("the input at the end of row 1", std::time::Duration::from_secs(5), |s| {
        at(s).is_some_and(|(x, y, h)| x > 0.0 && y.abs() < 0.5 && h > 1.0)
    })?;
    let (end_of_one, _, row_h) = at(app.screen()).unwrap();
    typ(&mut app, &["enter"]);
    // The next row, at its start.
    app.wait_until("the input at the start of row 2", std::time::Duration::from_secs(5), |s| {
        at(s).is_some_and(|(x, y, _)| x < end_of_one && (y - row_h).abs() < 0.5)
    })?;
    Ok(())
}
