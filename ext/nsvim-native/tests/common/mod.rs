//! Helpers every Headless test shares: a scratch project, the app started on it on a Linux
//! desktop, and keys typed one after another.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use nsh_core::platform::Os;
use nsh_engine::{EngineConfig, Points, Size};
use nsh_testing::Headless;

/// Config for tests: no control socket.
pub const CONFIG: &str = "[ipc]\nenabled = false\n";

/// A folder for test files, outside /tmp and outside the git checkout.
pub fn scratch() -> tempfile::TempDir {
    let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&base).unwrap();
    tempfile::tempdir_in(base).unwrap()
}

/// Keeps the editor's state and config files in a scratch folder, not in the user's home.
pub fn isolate_home() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("editor-nsh-home");
        // Setup's terminal step writes under $HOME (lines in the shell's rc file, git's global
        // config). Tests must never touch the user's own.
        let home = base.join("home");
        std::fs::create_dir_all(&home).unwrap();
        // SAFETY: set once, before any test starts the app.
        unsafe {
            std::env::set_var("XDG_STATE_HOME", base.join("state"));
            std::env::set_var("XDG_CONFIG_HOME", base.join("config"));
            std::env::set_var("HOME", &home);
            std::env::set_var("SHELL", "/bin/bash");
        }
    });
}

/// The UI sets the app's runner links: NSH's base components, then the app's (`nsvim-native`).
pub fn ui_sets() -> Vec<nsh_core::element::Registry> {
    vec![nsh_base::registry(), nsvim_native::registry()]
}

/// `App.tsx` as bytecode, built once for every test in this binary.
fn bundle() -> nsh_engine::Bundle {
    static APP: std::sync::OnceLock<nsh_engine::Bundle> = std::sync::OnceLock::new();
    APP.get_or_init(|| {
        let entry = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../App.tsx");
        nsh_testing::compile_tsx(entry, &ui_sets()).expect("App.tsx builds")
    })
    .clone()
}

/// The app (`App.tsx`) on a Linux desktop, opening `pending`. The editor is a desktop app, and
/// Headless shows a window only on a desktop OS.
pub fn engine(pending: nsvim_native::Pending) -> Headless {
    isolate_home();
    let app = bundle();
    // One engine starts at a time, so that each takes its own launch.
    static STARTING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = STARTING.lock().unwrap_or_else(|e| e.into_inner());
    nsvim_native::launch_next(pending);
    let mut config = EngineConfig { viewport: Size::new(Points(1200.0), Points(800.0)), ui_sets: ui_sets(), ..EngineConfig::new(app) };
    config.environment.os = Os::Linux;
    let mut ui = Headless::start(config).unwrap();
    let start = std::time::Instant::now();
    while !nsvim_native::launch_taken() {
        assert!(start.elapsed() < std::time::Duration::from_secs(10), "App.tsx never made its Workspace");
        ui.wait(std::time::Duration::from_millis(5)).unwrap();
    }
    ui
}

/// The app with `launch`.
pub fn start_with(launch: nsvim_native::Launch) -> Headless {
    engine(nsvim_native::Pending { launch, config_file: None, on_session: None })
}

/// The app with `files` open, in their folder as the project, with `config` (or the tests'
/// default config).
pub fn start_files(files: &[&Path], config: Option<&str>) -> Headless {
    start_with(nsvim_native::Launch {
        project: files.first().and_then(|f| f.parent()).map(Path::to_path_buf),
        config: Some(config.unwrap_or(CONFIG).into()),
        files: files.iter().map(|f| f.to_path_buf()).collect(),
        first_launch: false,
    })
}

/// The app with one file open.
pub fn start(file: &Path) -> Headless {
    start_files(&[file], None)
}

/// Writes `text` to `name` in `dir` and returns its path.
pub fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, text).unwrap();
    p
}

/// Presses each key in turn (`"j"`, `"shift+a"`, `"escape"`, `"ctrl+r"`).
pub fn typ(app: &mut Headless, keys: &[&str]) {
    for k in keys {
        app.press_key(k).unwrap();
    }
}

/// Types each char of `text` as a key: letters, digits, symbols and spaces.
pub fn type_text(app: &mut Headless, text: &str) {
    for c in text.chars() {
        let key = match c {
            ' ' => "space".to_string(),
            '\n' => "enter".to_string(),
            c if c.is_ascii_uppercase() => format!("shift+{}", c.to_ascii_lowercase()),
            ':' => "shift+;".into(),
            '!' => "shift+1".into(),
            '?' => "shift+/".into(),
            '_' => "shift+-".into(),
            '(' => "shift+9".into(),
            ')' => "shift+0".into(),
            '{' => "shift+[".into(),
            '}' => "shift+]".into(),
            '"' => "shift+'".into(),
            c => c.to_string(),
        };
        app.press_key(&key).unwrap();
    }
}

// Probes: the flows tests read and change the app's state on the engine thread.

use std::time::{Duration, Instant};

use nsh_core::prelude::*;
use nsvim_native::Session;
use nsvim_native::app::App;
use nsvim_native::app::fx::Tx;

/// Work for the engine thread: a closure that gets the app's session.
pub type Job = Box<dyn FnOnce(Session) + Send>;

/// The app under test and a probe into its state.
pub struct Flow {
    pub ui: Headless,
    jobs: tokio::sync::mpsc::UnboundedSender<Job>,
}

/// The app with `launch`, probed: a task on the engine thread runs the probe's jobs with the
/// session.
pub fn flow_with(launch: nsvim_native::Launch) -> Flow {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Job>();
    let on_session = Box::new(move |s: Session| {
        spawn(async move {
            while let Some(job) = rx.recv().await {
                job(s);
            }
        });
    });
    let ui = engine(nsvim_native::Pending { launch, config_file: None, on_session: Some(on_session) });
    let mut f = Flow { ui, jobs: tx };
    // Keys go nowhere until the view that takes them has the focus.
    let start = Instant::now();
    while !f.read(|a| a.key_focus) {
        assert!(start.elapsed() < Duration::from_secs(10), "the key view never took the focus");
        f.ui.wait(Duration::from_millis(20)).unwrap();
    }
    f
}

/// The app with `files` open in their folder, with `config` (or the tests' default), probed.
pub fn flow_files(files: &[&Path], config: Option<&str>) -> Flow {
    flow_with(nsvim_native::Launch {
        project: files.first().and_then(|f| f.parent()).map(Path::to_path_buf),
        config: Some(config.unwrap_or(CONFIG).into()),
        files: files.iter().map(|f| f.to_path_buf()).collect(),
        first_launch: false,
    })
}

/// The app with `file` open, probed. It waits until the file shows in the buffer.
pub fn flow(file: &Path, config: Option<&str>) -> Flow {
    let mut f = flow_files(&[file], config);
    f.wait_open(file);
    f
}

/// Config for the flows: no socket, and the keymap `map`.
pub fn keymap_config(map: &str) -> String {
    format!("{CONFIG}[keys]\nmap = \"{map}\"\n")
}

impl Flow {
    /// Runs `job` on the engine thread and waits for its answer.
    pub fn on_engine<R: Send + 'static>(&self, job: impl FnOnce(Session) -> R + Send + 'static) -> R {
        let (tx, rx) = std::sync::mpsc::channel();
        let job: Job = Box::new(move |s| {
            let _ = tx.send(job(s));
        });
        self.jobs.send(job).map_err(|_| "the probe task ended").unwrap();
        rx.recv_timeout(Duration::from_secs(20)).expect("the engine ran the probe")
    }

    /// Reads the app's state.
    pub fn read<R: Send + 'static>(&self, f: impl FnOnce(&App) -> R + Send + 'static) -> R {
        self.on_engine(move |s| s.read(f))
    }

    /// Changes the app's state through `Session::update`, as an input handler does.
    pub fn update<R: Send + 'static>(&mut self, f: impl FnOnce(&mut App, &mut Tx) -> R + Send + 'static) -> R {
        let r = self.on_engine(move |s| s.update(f));
        self.ui.catch_up();
        r
    }

    /// Whether `cond` holds now.
    pub fn check(&self, cond: impl Fn(&App) -> bool + Send + 'static) -> bool {
        self.read(cond)
    }

    /// Waits until `cond` holds, with the real workers running. Panics with `what` after 20 s.
    pub fn settle(&mut self, what: &str, cond: impl Fn(&App) -> bool + Send + Sync + Clone + 'static) {
        let start = Instant::now();
        loop {
            let c = cond.clone();
            if self.read(c) {
                self.ui.catch_up();
                return;
            }
            assert!(start.elapsed() < Duration::from_secs(20), "{what}: nothing happened in 20 s");
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    /// Waits until the active buffer is `path`.
    pub fn wait_open(&mut self, path: &Path) {
        let p = path.to_path_buf();
        self.settle("the file opens", move |a| a.ed.has_doc() && a.ed.buf().path.as_deref() == Some(p.as_path()));
    }

    /// Presses the key for each char of `text`, as `key_name` names it.
    pub fn keys(&mut self, text: &str) {
        for c in text.chars() {
            self.ui.press_key(&key_name(c)).unwrap();
        }
    }

    /// Presses one keystroke, as `nsh/keymap` names it (`"ctrl+d"`, `"shift+tab"`).
    pub fn press(&mut self, stroke: &str) {
        self.ui.press_key(stroke).unwrap();
    }

    /// The active buffer's text.
    pub fn text(&self) -> String {
        self.read(|a| a.ed.buf().text())
    }

    /// The message on the command line, if any.
    pub fn msg(&self) -> Option<(String, bool)> {
        self.read(|a| a.ed.msg.as_ref().map(|m| (m.text.clone(), m.error)))
    }

    /// Whether the message on the command line holds `part`.
    pub fn said(&self, part: &str) -> bool {
        self.msg().is_some_and(|(m, _)| m.contains(part))
    }

    /// Sends the agent request `cmd` with `args`, as the socket does, and returns the answer.
    pub fn ask(&mut self, cmd: &str, args: serde_json::Value) -> nsvim_native::ipc::Response {
        let (tx, rx) = std::sync::mpsc::channel();
        let req = nsvim_native::ipc::Request { id: 7, cmd: cmd.into(), args };
        self.update(move |a, t| a.ipc(req, tx, t));
        rx.recv_timeout(Duration::from_secs(5)).expect("an answer")
    }
}

/// The keystroke that types `c` on a US layout: `"shift+a"` for `A`, `"shift+;"` for `:`,
/// `"enter"` for a line break and `"escape"` for `\x1b`.
pub fn key_name(c: char) -> String {
    const SHIFTED: &[(char, char)] = &[
        ('~', '`'),
        ('!', '1'),
        ('@', '2'),
        ('#', '3'),
        ('$', '4'),
        ('%', '5'),
        ('^', '6'),
        ('&', '7'),
        ('*', '8'),
        ('(', '9'),
        (')', '0'),
        ('_', '-'),
        ('+', '='),
        ('{', '['),
        ('}', ']'),
        ('|', '\\'),
        (':', ';'),
        ('"', '\''),
        ('<', ','),
        ('>', '.'),
        ('?', '/'),
    ];
    match c {
        ' ' => "space".into(),
        '\n' => "enter".into(),
        '\x1b' => "escape".into(),
        '\t' => "tab".into(),
        c if c.is_ascii_uppercase() => format!("shift+{}", c.to_ascii_lowercase()),
        c => match SHIFTED.iter().find(|(s, _)| *s == c) {
            Some((_, base)) => format!("shift+{base}"),
            None => c.to_string(),
        },
    }
}

/// The Rust file most flows edit.
pub const SRC: &str = "fn main() {\n    let a = 1;\n    let b = 2;\n    println!(\"{}\", a + b);\n}\n";

/// The app with `main.rs` holding `src` open, in keymap `map`. Keep the folder alive while the
/// test runs.
pub fn rust_flow(map: &str, src: &str) -> (tempfile::TempDir, Flow) {
    let dir = scratch();
    let file = write(dir.path(), "main.rs", src);
    let f = flow(&file, Some(&keymap_config(map)));
    (dir, f)
}

/// A point in a view, in points from its top left.
pub fn point(x: f32, y: f32) -> nsh_core::geometry::Point<Points> {
    nsh_core::geometry::Point { x: Points(x), y: Points(y) }
}

impl Flow {
    /// A middle-button click on the view `selector` names: its release, at `at` in the view, as
    /// the desktop hosts report it. Headless has no step for the middle button.
    pub fn middle_click(&mut self, selector: &str, at: nsh_core::geometry::Point<Points>) {
        use nsh_core::{EventKind, EventPayload, MouseButton, MouseEvent};
        let id = self.ui.screen().id(selector).unwrap();
        let event = MouseEvent::released(at, at, Default::default(), MouseButton::Middle);
        self.ui.engine().control().dispatch_event(id, EventKind::MouseUp, EventPayload::Mouse(event));
        self.ui.wait(Duration::from_millis(50)).unwrap();
    }
}
