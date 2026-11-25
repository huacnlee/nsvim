//! The overlays, headless: the finder and the other pickers, the leader menu, the key list,
//! the in-app context menu and the platform's context menu.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use common::{CONFIG, scratch, start, start_files, typ, type_text, write};
use nsh_app_editor::Session;
use nsh_app_editor::app::fx::{PopupItem, PopupMenu};
use nsh_app_editor::app::menu::Menu;
use nsh_app_editor::app::{App, Overlay};
use nsh_core::element::desktop::window;
use nsh_core::platform::Os;
use nsh_core::prelude::*;
use nsh_engine::{EngineConfig, Points, Root, Size};
use nsh_testing::Headless;

const WAIT: Duration = Duration::from_secs(5);

/// The classic keymap, with no control socket.
const CLASSIC: &str = "[ipc]\nenabled = false\n[keys]\nmap = \"classic\"\n";

/// Waits until the project index lists `n` files, so the finder has them.
fn wait_files(app: &mut Headless, n: usize) -> Result<(), String> {
    app.wait_until("the project index", WAIT, |sc| sc.texts().iter().any(|t| t.ends_with(&format!("/ {n}"))))
}

/// The file the status line names.
fn status_file(s: &nsh_testing::Screen) -> Option<String> {
    s.accessibility(Some("id:file")).ok().flatten().map(|n| n.name)
}

#[test]
fn the_finder_filters_by_the_query_and_enter_opens_the_pick() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    write(dir.path(), "src/needle.txt", "found me\n");
    write(dir.path(), "other.md", "# other\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["space", "f"]);
    app.wait_for("text:Find Files")?;
    wait_files(&mut app, 3)?;
    app.wait_for("id:picker-list >> label:other.md")?;
    type_text(&mut app, "needl");
    app.wait_for("id:picker-input >> text:needl")?;
    app.wait_for("text:1 / 3")?;
    assert!(app.screen().matches("id:picker-list >> label:src/needle.txt")?, "{:?}", app.screen().texts());
    assert!(!app.screen().matches("id:picker-list >> label:other.md")?, "other.md does not match");
    // The directory shows apart from the name.
    assert!(app.screen().matches("id:picker-list >> text:src/")?, "{:?}", app.screen().texts());
    assert!(app.screen().matches("id:picker-list >> text:needle.txt")?, "{:?}", app.screen().texts());
    // The preview shows the picked file's text.
    app.wait_for("id:picker-preview >> text:/found me/")?;
    typ(&mut app, &["enter"]);
    app.wait_for_gone("id:picker", WAIT)?;
    app.wait_for("text:/found me/")?;
    app.wait_until("the file opens", WAIT, |sc| status_file(sc).as_deref() == Some("src/needle.txt"))?;
    Ok(())
}

#[test]
fn escape_and_a_press_outside_close_the_finder() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["space", "f"]);
    app.wait_for("id:picker")?;
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:picker", WAIT)?;
    typ(&mut app, &["space", "f"]);
    app.wait_for("id:picker")?;
    app.press("id:overlay-scrim")?;
    app.wait_for_gone("id:picker", WAIT)?;
    // The keys go to the text again.
    typ(&mut app, &["shift+a"]);
    app.wait_for("INSERT")?;
    Ok(())
}

#[test]
fn a_press_on_a_row_opens_it() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    write(dir.path(), "zeta.txt", "zeta text\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["space", "f"]);
    wait_files(&mut app, 2)?;
    app.press("id:picker-list >> label:zeta.txt")?;
    app.wait_for_gone("id:picker", WAIT)?;
    app.wait_for("text:/zeta text/")?;
    Ok(())
}

#[test]
fn the_leader_menu_lists_keys_and_runs_the_one_pressed() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta|alpha/")?;
    typ(&mut app, &["space"]);
    app.wait_for("id:leader")?;
    assert!(app.shows("SPC"));
    assert!(app.screen().matches("role:menuitem[find file]")?);
    assert!(app.shows("+git"));
    // A group opens its own keys.
    typ(&mut app, &["g"]);
    app.wait_for("role:menuitem[next change]")?;
    assert!(app.shows("SPC g"));
    typ(&mut app, &["escape"]);
    app.wait_for_gone("id:leader", WAIT)?;
    // A press on an entry runs it: buffers lists the open files.
    typ(&mut app, &["space"]);
    app.press("role:menuitem[buffers]")?;
    app.wait_for("text:Buffers")?;
    app.wait_for("id:picker-list >> label:a.txt")?;
    app.wait_for("id:picker-list >> label:b.txt")?;
    Ok(())
}

#[test]
fn the_buffer_picker_switches_to_the_picked_buffer() -> Result<(), String> {
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "beta\n");
    let mut app = start_files(&[&a, &b], None);
    app.wait_for("text:/beta|alpha/")?;
    typ(&mut app, &["space", "b"]);
    app.wait_for("id:picker-list >> label:a.txt")?;
    type_text(&mut app, "a.t");
    app.wait_for("text:1 / 2")?;
    typ(&mut app, &["enter"]);
    app.wait_for_gone("id:picker", WAIT)?;
    app.wait_for("text:/alpha/")?;
    app.wait_until("a.txt is active", WAIT, |sc| status_file(sc).as_deref() == Some("a.txt"))?;
    Ok(())
}

#[test]
fn project_search_lists_hits_and_previews_the_line() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    write(dir.path(), "lib.rs", "// one\n// two\nconst NEEDLE: u8 = 1;\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["space", "/"]);
    app.wait_for("text:Find in Project")?;
    app.wait_for("text:type to search the project")?;
    type_text(&mut app, "NEEDLE");
    app.wait_for("id:picker-list >> label:lib.rs:3")?;
    // The preview follows the picked row: a move asks for it.
    typ(&mut app, &["down"]);
    app.wait_for("id:picker-preview >> text:/const/")?;
    assert!(app.screen().matches("id:picker-preview >> text:lib.rs")?, "the preview names the file");
    typ(&mut app, &["enter"]);
    app.wait_for_gone("id:picker", WAIT)?;
    app.wait_for("text:3:7")?;
    Ok(())
}

#[test]
fn the_palette_filters_commands_and_runs_the_pick() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    let mut app = start_files(&[&main], Some(CLASSIC));
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["ctrl+shift+p"]);
    app.wait_for("text:Commands")?;
    assert!(app.screen().matches("role:option[Save all]")?);
    type_text(&mut app, "every key");
    app.wait_for("role:option[Show every key]")?;
    assert!(!app.screen().matches("role:option[Save all]")?, "the query filters the list");
    typ(&mut app, &["enter"]);
    // The command opens the key list in the same overlay.
    app.wait_for("text:Keys")?;
    assert!(!app.shows("Commands"));
    Ok(())
}

#[test]
fn the_key_list_searches_by_words_and_f1_closes_it() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["f1"]);
    app.wait_for("text:Keys")?;
    let all = nsh_app_editor::app::key_list::items(true);
    let total = format!("{} / {}", all.len(), all.len());
    app.wait_for(&format!("text:{total}"))?;
    type_text(&mut app, "first row");
    app.wait_until("the search narrows the list", WAIT, |sc| {
        sc.texts().iter().any(|t| t.contains(" / ") && !t.starts_with(&format!("{} ", all.len())) && !t.starts_with("0 "))
    })?;
    let want: Vec<&String> = all.iter().filter(|i| i.to_lowercase().contains("first") && i.to_lowercase().contains("row")).collect();
    let first = want[0].split('\t').next().unwrap();
    app.wait_for(&format!("role:option[{first}]"))?;
    typ(&mut app, &["f1"]);
    app.wait_for_gone("id:picker", WAIT)?;
    Ok(())
}

#[test]
fn the_list_follows_the_picked_row_past_the_screen() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["f1"]);
    app.wait_for("text:Keys")?;
    let all = nsh_app_editor::app::key_list::items(true);
    let label = |i: usize| all[i].split('\t').next().unwrap().to_string();
    let last = format!("role:option[{}]", label(all.len() - 1));
    assert!(!app.screen().matches(&last)?, "the last row starts off screen");
    for _ in 0..80 {
        typ(&mut app, &["down"]);
    }
    let picked = format!("role:option[{}]", label(80));
    app.wait_for(&picked)?;
    assert!(app.screen().selected(&picked)?, "row 80 is the picked one");
    let first = format!("role:option[{}]", label(0));
    assert!(!app.screen().matches(&first)?, "the list scrolled past the first row");
    // Back up to the top, the list scrolls back.
    for _ in 0..80 {
        typ(&mut app, &["up"]);
    }
    app.wait_for(&first)?;
    assert!(app.screen().selected(&first)?);
    Ok(())
}

#[test]
fn the_wheel_scrolls_the_list_and_keeps_the_picked_row() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["f1"]);
    app.wait_for("text:Keys")?;
    let all = nsh_app_editor::app::key_list::items(true);
    let row = |i: usize| format!("role:option[{}]", all[i].split('\t').next().unwrap());
    app.wait_for(&row(0))?;
    // Ten rows of wheel scroll ten rows. The picked row stays the first one.
    app.wheel("id:picker-list", nsh_core::geometry::Point::new(Points(0.0), Points(22.0 * 10.0)))?;
    app.wait_for_gone(&row(0), WAIT)?;
    app.wait_for(&row(10))?;
    assert!(!app.screen().matches(&row(9))?, "row 9 scrolled off");
    // A key brings the picked row back: Down picks row 1, and the list shows it first.
    typ(&mut app, &["down"]);
    app.wait_for(&row(1))?;
    app.wait_until("row 1 is picked", WAIT, |s| s.selected(&row(1)).unwrap_or(false))?;
    assert!(!app.screen().matches(&row(0))?, "the list moved the least that shows the pick");
    Ok(())
}

/// A session on a scratch project, for the views that need one alone.
fn session(dir: &std::path::Path) -> Session {
    let cfg = toml::from_str(CONFIG).unwrap();
    Session::new(App::new(cfg, PathBuf::from(dir), false, None))
}

fn start_root(root: Root) -> Headless {
    let mut config = EngineConfig { viewport: Size::new(Points(1200.0), Points(800.0)), ..EngineConfig::new(root) };
    config.environment.os = Os::Linux;
    Headless::start(config).unwrap()
}

#[derive(Clone, PartialEq, Props)]
struct MenuHostProps {
    project: Prop<SharedString>,
}

/// A button that shows a native menu and the id it answers with.
#[component]
fn MenuHost(props: MenuHostProps) -> impl IntoElement {
    let s = session(std::path::Path::new(props.project.get_untracked().as_str()));
    let answer = signal(String::from("none yet"));
    let content = move || {
        view().flex_1().child(text(move || format!("picked:{}", answer.get()))).child(text("Open").on_press(move || {
            let menu = PopupMenu::new()
                .item(PopupItem::new("cut", "Cut".into()).enabled(false))
                .item(PopupItem::new("copy", "Copy".into()))
                .separator()
                .item(PopupItem::new("grep", "Find in project".into()));
            let then = Box::new(move |_: &mut App, id: Option<String>, _: &mut nsh_app_editor::app::fx::Tx| {
                answer.set(id.unwrap_or_else(|| "nothing".into()));
            });
            nsh_app_editor::ui::native_menu::native_menu(s, menu, 40.0, 50.0, then);
        }))
    };
    window().id("test").content(content)
}

#[test]
fn the_native_menu_shows_the_items_and_answers_the_pick() -> Result<(), String> {
    let dir = scratch();
    let project = SharedString::from(dir.path().to_string_lossy().into_owned());
    let mut app = start_root(Root::new::<MenuHost>(move || MenuHostProps { project: Prop::from(project.clone()) }));
    app.wait_for("text:picked:none yet")?;
    app.press("Open")?;
    let entries = app.menu().ok_or("no menu shows")?;
    let labels: Vec<&str> = entries.iter().map(|e| e.label.as_str()).collect();
    assert_eq!(labels, ["Cut", "Copy", "Find in project"]);
    assert!(entries[0].disabled && !entries[1].disabled);
    app.choose_menu_item("Copy")?;
    app.wait_for("text:picked:copy")?;
    // Closed with nothing picked, the callback gets None.
    app.press("Open")?;
    app.close_menu()?;
    app.wait_for("text:picked:nothing")?;
    Ok(())
}

#[test]
fn the_in_app_menu_shows_its_items_and_runs_the_pressed_one() -> Result<(), String> {
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "hello world\n");
    let mut f = common::flow(&main, None);
    f.update(|a, _| a.overlay = Overlay::Menu(Menu { x: 100.0, y: 120.0, sel: 0 }));
    let app = &mut f.ui;
    app.wait_for("id:context-menu")?;
    assert!(app.screen().matches("role:menuitem[Go to definition]")?);
    assert!(app.shows("gd"), "the Neovim key hints show");
    assert!(!app.screen().enabled("role:menuitem[Copy]")?, "Copy needs a selection");
    assert!(app.screen().enabled("role:menuitem[Paste]")?);
    assert!(app.screen().accessibility_state("role:menuitem[Cut]")?.selected, "the first item is the picked one");
    // A disabled item takes no press.
    assert!(app.press("role:menuitem[Copy]").is_err(), "Copy takes no press");
    assert!(app.screen().matches("id:context-menu")?);
    // Find in project searches for the word at the cursor.
    app.press("role:menuitem[Find in project]")?;
    app.wait_for("text:Find in Project")?;
    app.wait_for("id:picker-input >> text:hello")?;
    assert!(!app.screen().matches("id:context-menu")?);
    Ok(())
}
