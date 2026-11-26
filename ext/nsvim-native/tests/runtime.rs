//! The runtime, headless: the control socket serves a running editor, the session reopens
//! files, config.toml applies when it changes, and quitting guards unsaved edits.

mod common;

use std::path::{Path, PathBuf};
use std::time::Duration;

use common::{CONFIG, isolate_home, scratch, start, start_with, typ, write};
use editor::buffer::Pos;
use nsh_testing::Headless;
use nsvim_native::{Launch, ipc};
use serde_json::json;

/// A config that serves the control socket.
const IPC_CONFIG: &str = "[ipc]\nenabled = true\n";

/// Keeps every file the editor writes in a folder of this test run: state, config, control
/// sockets, and the home folder and shell, so nothing can reach the user's own. Every test
/// calls it before it starts the app. The socket path stays short: a Unix socket path has a
/// limit of about 100 bytes.
fn isolate() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        isolate_home();
        let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let dir = base.join(format!("rt-{}", std::process::id()));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        // SAFETY: set once, before any test starts the app.
        unsafe {
            std::env::set_var("XDG_RUNTIME_DIR", &dir);
            std::env::set_var("HOME", &home);
            std::env::set_var("SHELL", home.join("sh"));
        }
    });
}

/// The app on `files` in `project`, with the control socket on.
fn start_serving(project: &Path, files: &[&Path]) -> Headless {
    isolate();
    start_with(Launch {
        project: Some(project.to_path_buf()),
        config: Some(IPC_CONFIG.into()),
        files: files.iter().map(|f| f.to_path_buf()).collect(),
        first_launch: false,
    })
}

/// Waits until the editor of `project` answers on its socket, and returns the socket.
fn socket(app: &mut Headless, project: &Path) -> Result<PathBuf, String> {
    let path = ipc::socket_for(project);
    let p = path.clone();
    app.wait_until("the control socket", Duration::from_secs(5), move |_| ipc::alive(&p))?;
    Ok(path)
}

/// Runs `f` on a thread while the engine runs, then returns what it returned. A request blocks
/// until the engine thread answers it, so the test thread must keep the app going.
fn while_running<T: Send + 'static>(app: &mut Headless, f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    let job = std::thread::spawn(f);
    // `wait_until` looks again only when the screen changes, and a request can change nothing.
    for _ in 0..200 {
        if job.is_finished() {
            return job.join().map_err(|_| "the request thread panicked".to_string());
        }
        app.wait(Duration::from_millis(50))?;
    }
    Err(format!("the request did not finish: {:?}", app.screen().texts()))
}

/// Waits until the app quit.
fn wait_quit(app: &mut Headless) -> Result<(), String> {
    for _ in 0..50 {
        if app.quit() {
            return Ok(());
        }
        app.wait(Duration::from_millis(100))?;
    }
    Err(format!("the app did not quit: {:?}", app.screen().texts()))
}

fn ex(app: &mut Headless, cmd: &str) {
    typ(app, &["shift+;"]);
    common::type_text(app, cmd);
    typ(app, &["enter"]);
}

#[test]
fn a_path_opens_in_the_running_editor_over_its_socket() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "bravo one\nbravo two\n");
    let mut app = start_serving(dir.path(), &[&a]);
    app.wait_for("text:/alpha/")?;
    socket(&mut app, dir.path())?;
    let project = dir.path().to_path_buf();
    let code = while_running(&mut app, move || ipc::client::open_in_running(&project, &[(b, Some(Pos::new(1, 0)))], false))?;
    assert_eq!(code, Some(0), "the running editor took the file");
    app.wait_for("text:/bravo two/")?;
    app.wait_until("the cursor on line 2", Duration::from_secs(5), |s| s.shows("2:1"))?;
    assert!(app.shows("b.txt"), "{:?}", app.screen().texts());
    Ok(())
}

#[test]
fn wait_returns_when_the_buffer_closes() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "NOTES.md", "notes\n");
    let mut app = start_serving(dir.path(), &[&a]);
    app.wait_for("text:/alpha/")?;
    socket(&mut app, dir.path())?;
    let project = dir.path().to_path_buf();
    let job = std::thread::spawn(move || ipc::client::open_in_running(&project, &[(b, None)], true));
    app.wait_for("text:/notes/")?;
    app.wait(Duration::from_millis(300))?;
    assert!(!job.is_finished(), "--wait holds while the buffer is open");
    ex(&mut app, "q");
    app.wait_for("text:/alpha/")?;
    let code = while_running(&mut app, move || job.join().ok().flatten())?;
    assert_eq!(code, Some(0));
    assert!(!app.quit(), "a.txt is still open");
    Ok(())
}

#[test]
fn no_running_editor_means_the_path_opens_a_window() {
    isolate();
    let dir = scratch();
    assert_eq!(ipc::client::open_in_running(dir.path(), &[], false), None);
}

#[test]
fn ctl_requests_round_trip_through_the_socket() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\nbeta\n");
    let mut app = start_serving(dir.path(), &[&a]);
    app.wait_for("text:/beta/")?;
    let sock = socket(&mut app, dir.path())?;
    let s = sock.clone();
    let state = while_running(&mut app, move || ipc::call(&s, "state", json!({}), Some(Duration::from_secs(5))))?
        .map_err(|e| format!("state: {e}"))?;
    assert!(state.ok, "{state:?}");
    assert_eq!(state.result["active"]["relative"], "a.txt", "{state:?}");
    assert_eq!(state.result["active"]["lines"], 2, "{state:?}");
    let s = sock.to_string_lossy().into_owned();
    let args: Vec<String> = ["--socket", &s, "message", "hello from ctl"].iter().map(|a| a.to_string()).collect();
    let code = while_running(&mut app, move || ipc::client::main(&args))?;
    assert_eq!(code, 0);
    app.wait_for("text:hello from ctl")?;
    Ok(())
}

#[test]
fn the_session_reopens_its_files_where_the_cursor_was() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let a = write(dir.path(), "a.txt", "alpha\n");
    let b = write(dir.path(), "b.txt", "one\ntwo\nthree\n");
    let mut app = common::start_files(&[&a, &b], None);
    app.wait_for("text:/three/")?;
    app.wait_until("b.txt active", Duration::from_secs(5), |s| s.shows("1:1") && s.shows("b.txt"))?;
    typ(&mut app, &["j", "j", "l"]);
    app.wait_for("3:2")?;
    ex(&mut app, "qa");
    wait_quit(&mut app)?;
    drop(app);
    let mut again =
        start_with(Launch { project: Some(dir.path().to_path_buf()), config: Some(CONFIG.into()), files: vec![], first_launch: false });
    again.wait_for("text:/three/")?;
    again.wait_until("the cursor where it was", Duration::from_secs(5), |s| s.shows("3:2"))?;
    // The other file is open again too: the next buffer is a.txt.
    ex(&mut again, "bn");
    again.wait_for("text:/alpha/")?;
    Ok(())
}

/// The app on `file`, reading `config` again when it changes.
fn start_watching(file: &Path, config: &Path) -> Headless {
    isolate();
    let launch = Launch {
        project: file.parent().map(Path::to_path_buf),
        config: Some(std::fs::read_to_string(config).unwrap()),
        files: vec![file.to_path_buf()],
        first_launch: false,
    };
    common::engine(nsvim_native::Pending { launch, config_file: Some(config.to_path_buf()), on_session: None })
}

#[test]
fn a_change_to_config_toml_applies_while_the_editor_runs() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha\n");
    let conf = scratch();
    let config = write(conf.path(), "config.toml", CONFIG);
    let mut app = start_watching(&file, &config);
    app.wait_for("text:/alpha/")?;
    app.wait_for("NORMAL")?;
    // The watcher starts with the workers. Write until it sees a change.
    for n in 0..20 {
        std::fs::write(&config, format!("{CONFIG}[keys]\nmap = \"classic\"\n# {n}\n")).unwrap();
        if app.wait_for_within("text:/config.toml changed on disk/", Duration::from_millis(500)).is_ok() {
            break;
        }
    }
    app.wait_for("text:/config.toml changed on disk/")?;
    app.wait_until("the classic keymap", Duration::from_secs(5), |s| !s.shows("NORMAL"))?;
    Ok(())
}

#[test]
fn a_config_that_does_not_parse_shows_why_and_keeps_the_old_one() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha\n");
    let conf = scratch();
    let config = write(conf.path(), "config.toml", CONFIG);
    let mut app = start_watching(&file, &config);
    app.wait_for("text:/alpha/")?;
    for n in 0..20 {
        std::fs::write(&config, format!("[keys\nbroken {n}\n")).unwrap();
        if app.wait_for_within("text:/config.toml: /", Duration::from_millis(500)).is_ok() {
            break;
        }
    }
    app.wait_for("text:/config.toml: /")?;
    assert!(app.shows("NORMAL"), "the old keymap stays: {:?}", app.screen().texts());
    Ok(())
}

#[test]
fn q_with_unsaved_edits_warns_and_keeps_the_window() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&file);
    app.wait_for("text:/alpha/")?;
    typ(&mut app, &["x"]);
    app.wait_for("text:/^lpha/")?;
    ex(&mut app, "q");
    app.wait_for("text:/E37/")?;
    assert!(!app.quit(), "the editor stays open");
    assert_eq!(app.window_ids()?, vec!["main".to_string()]);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\n");
    Ok(())
}

#[test]
fn wq_writes_the_file_then_quits() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&file);
    app.wait_for("text:/alpha/")?;
    typ(&mut app, &["x"]);
    app.wait_for("text:/^lpha/")?;
    ex(&mut app, "wq");
    let f = file.clone();
    app.wait_until("the file written", Duration::from_secs(5), move |_| std::fs::read_to_string(&f).unwrap() == "lpha\n")?;
    wait_quit(&mut app)
}

#[test]
fn the_close_button_quits_a_clean_editor() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let file = write(dir.path(), "a.txt", "alpha\n");
    let mut app = start(&file);
    app.wait_for("text:/alpha/")?;
    app.press_window_close("main")?;
    wait_quit(&mut app)
}

/// The editor with `a.txt` open and edited, which answers its dialogs only when the test says.
fn edited(dir: &Path) -> (PathBuf, Headless) {
    let file = write(dir, "a.txt", "alpha\n");
    let mut app = start(&file);
    app.answer_dialogs_myself();
    app.wait_for("text:/alpha/").unwrap();
    typ(&mut app, &["x"]);
    app.wait_for("text:/^lpha/").unwrap();
    (file, app)
}

fn press(app: &mut Headless, button: &str) -> Result<(), String> {
    app.answer_dialog(nsh_testing::DialogAnswer::Press(button.into()))
}

#[test]
fn the_close_button_asks_in_a_dialog_over_unsaved_edits() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let (file, mut app) = edited(dir.path());
    app.press_window_close("main")?;
    let d = app.pending_dialog().ok_or("the close button shows a dialog")?;
    assert_eq!(d.title.as_deref(), Some("Save changes to \"a.txt\"?"));
    let buttons: Vec<&str> = d.buttons.iter().map(|b| b.text.as_str()).collect();
    assert_eq!(buttons, ["Save", "Don't save", "Cancel"]);
    press(&mut app, "Cancel")?;
    assert!(!app.quit(), "cancel keeps the window");
    app.press_window_close("main")?;
    press(&mut app, "Don't save")?;
    wait_quit(&mut app)?;
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "alpha\n", "the file is not written");
    assert!(!nsvim_native::services::io::swap_file(&file).exists(), "no swap file asks to recover the dropped edit");
    Ok(())
}

#[test]
fn save_in_the_close_dialog_writes_the_file_then_quits() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let (file, mut app) = edited(dir.path());
    app.press_window_close("main")?;
    press(&mut app, "Save")?;
    let f = file.clone();
    app.wait_until("the file written", Duration::from_secs(5), move |_| std::fs::read_to_string(&f).unwrap() == "lpha\n")?;
    wait_quit(&mut app)
}

/// The editor with a new buffer (`:enew`) that holds `hello`, which answers its dialogs
/// only when the test says, and whose save dialog picks `pick`.
fn untitled(dir: &Path, pick: Option<PathBuf>) -> Headless {
    let file = write(dir, "a.txt", "alpha\n");
    let mut app = start(&file);
    app.wait_for("text:/alpha/").unwrap();
    ex(&mut app, "enew");
    app.wait_for("text:/No Name/").unwrap();
    typ(&mut app, &["i"]);
    common::type_text(&mut app, "hello");
    typ(&mut app, &["escape"]);
    app.wait_for("text:/hello/").unwrap();
    app.answer_dialogs_myself();
    app.on_pick_save_path(pick).unwrap();
    app
}

/// The save dialogs the app asked for.
fn save_dialogs(app: &Headless) -> Vec<nsh_core::file_dialog::SaveDialog> {
    app.screen()
        .file_requests()
        .iter()
        .filter_map(|r| match r {
            nsh_core::file_dialog::FileRequest::SaveDialog(d) => Some(d.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn save_in_the_close_dialog_asks_where_to_save_a_new_file_then_quits() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let new = dir.path().join("new.txt");
    let mut app = untitled(dir.path(), Some(new.clone()));
    app.press_window_close("main")?;
    let d = app.pending_dialog().ok_or("the close button shows a dialog")?;
    assert_eq!(d.title.as_deref(), Some("Save changes to \"[No Name]\"?"));
    press(&mut app, "Save")?;
    let f = new.clone();
    app.wait_until("the new file written", Duration::from_secs(5), move |_| {
        std::fs::read_to_string(&f).is_ok_and(|t| t.trim_end() == "hello")
    })?;
    let dialogs = save_dialogs(&app);
    assert_eq!(dialogs.len(), 1, "one save dialog");
    assert_eq!(
        dialogs[0].directory.as_deref().map(|p| p.canonicalize().unwrap()),
        Some(dir.path().canonicalize().unwrap()),
        "it starts in the project"
    );
    wait_quit(&mut app)
}

#[test]
fn a_cancel_in_the_save_dialog_keeps_the_window_and_the_edits() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let mut app = untitled(dir.path(), None);
    app.press_window_close("main")?;
    press(&mut app, "Save")?;
    app.wait_until("the save dialog", Duration::from_secs(5), |s| !s.file_requests().is_empty())?;
    app.wait(Duration::from_millis(300))?;
    assert!(!app.quit(), "cancel keeps the window");
    let texts = app.screen().texts().join("\n");
    assert!(texts.contains("hello"), "the edits stay: {texts}");
    assert!(!texts.contains("E141") && !texts.contains("E32"), "{texts}");
    Ok(())
}

#[test]
fn colon_w_on_a_new_file_says_e32_and_shows_no_dialog() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let mut app = untitled(dir.path(), Some(dir.path().join("never.txt")));
    ex(&mut app, "w");
    app.wait_for("text:/E32/")?;
    assert!(save_dialogs(&app).is_empty(), ":w shows no dialog");
    Ok(())
}

/// After the close dialog, the keyboard is the editor's again: `:` opens the command line, and
/// `:w` writes the file, with no click on the text first.
#[test]
fn the_command_line_opens_after_the_close_dialog() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let (file, mut app) = edited(dir.path());
    app.press_window_close("main")?;
    press(&mut app, "Cancel")?;
    assert!(!app.quit(), "cancel keeps the window");
    ex(&mut app, "w");
    let f = file.clone();
    app.wait_until("the file written", Duration::from_secs(5), move |_| std::fs::read_to_string(&f).unwrap() == "lpha\n")?;
    Ok(())
}

#[test]
fn the_quit_key_asks_as_the_close_button_does() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let (_file, mut app) = edited(dir.path());
    app.quit_request()?;
    assert!(app.pending_dialog().is_some_and(|d| d.title.as_deref() == Some("Save changes to \"a.txt\"?")));
    press(&mut app, "Cancel")?;
    assert!(!app.quit());
    Ok(())
}

#[test]
fn search_results_show_the_preview_without_a_key() -> Result<(), String> {
    isolate();
    let dir = scratch();
    let main = write(dir.path(), "main.rs", "fn main() {}\n");
    write(dir.path(), "lib.rs", "// one\n// two\nconst NEEDLE: u8 = 1;\n");
    let mut app = start(&main);
    app.wait_for("text:/fn main/")?;
    typ(&mut app, &["space", "/"]);
    app.wait_for("text:Find in project")?;
    common::type_text(&mut app, "NEEDLE");
    app.wait_for("id:picker-list >> label:lib.rs:3")?;
    // No key after the results: their arrival asks for the preview.
    app.wait_for("id:picker-preview >> text:/const/")?;
    Ok(())
}
