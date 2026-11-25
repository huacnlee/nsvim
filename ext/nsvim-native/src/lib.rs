//! nsvim on NSH. The core (`editor-core`) holds the text and the keymaps. `app` holds the
//! rest of the editor's state, and `services` the workers. The screens are TSX (`App.tsx` at
//! the repository root). They get the app's state and send changes through `bridge`, and use
//! the components and modules in `exports`.

pub mod app;
pub mod args;
pub mod bridge;
pub mod color;
pub mod config;
pub mod exports;
pub mod fuzzy;
pub mod geometry;
pub mod highlight;
pub mod ipc;
pub mod keys;
pub mod migrate;
pub mod model;
pub mod services;
pub mod store;
pub mod theme;
pub mod timing;
pub mod ui;
#[cfg(target_arch = "wasm32")]
mod web_libc;

use std::path::PathBuf;
use std::sync::Mutex;

pub use store::Session;

/// The app's UI set (an nshm, `nsh.json` `"dependencies"`): what `App.tsx` imports from `nsvim-native`.
#[native_module("nsvim-native")]
fn native(registry: &mut Registry) {
    exports::register(registry);
}

use nsh_core::geometry::{Points, Size};
use nsh_core::prelude::*;

/// What a window opens: the editor as the command line asks, or a set project for tests.
#[derive(Clone, Debug, Default)]
pub struct Launch {
    pub project: Option<PathBuf>,
    pub config: Option<String>,
    pub files: Vec<PathBuf>,
    pub first_launch: bool,
}

/// A launch that the next `Workspace` takes, instead of the command line's. Tests set it
/// (`launch_next`).
pub struct Pending {
    pub launch: Launch,
    /// The config file to read again when it changes. None: no reload.
    pub config_file: Option<PathBuf>,
    /// Runs on the engine thread with the new session, before the screens show.
    pub on_session: Option<Box<dyn FnOnce(Session) + Send>>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Makes the next `Workspace` open `p`. A test holds a lock of its own around this and the
/// engine's start, so that each engine takes its own launch (`launch_taken`).
pub fn launch_next(p: Pending) {
    *PENDING.lock().unwrap() = Some(p);
}

/// True once a `Workspace` took the launch that `launch_next` set.
pub fn launch_taken() -> bool {
    PENDING.lock().unwrap().is_none()
}

pub(crate) fn take_launch() -> Option<Pending> {
    PENDING.lock().unwrap().take()
}

/// The window's size in points when no size is kept.
pub const DEFAULT_SIZE: (f32, f32) = (1440.0, 900.0);

/// The least size of the window in points.
pub const MIN_SIZE: (f32, f32) = (640.0, 420.0);

/// The window's attributes for `cfg`: its title, least size, a transparent background when
/// `[window] opacity` is below 100, and a close button that asks the app first.
pub fn window_attributes(cfg: &config::Config) -> nsh_core::desktop::WindowAttributes {
    let mut a = nsh_core::desktop::WindowAttributes::default();
    a.min_size = Some(Some(Size::new(Points(MIN_SIZE.0), Points(MIN_SIZE.1))));
    a.transparent = Some(opacity(cfg) < 1.0);
    a.close_on_press = Some(false);
    a
}

/// The background's opacity from `[window] opacity`: 0.7 to 1.
pub fn opacity(cfg: &config::Config) -> f32 {
    (cfg.window.opacity as f32 / 100.0).clamp(0.7, 1.0)
}

/// The app state that a launch asks for: with a project, that project, config and files, and
/// the last session when no file is given. Without one, the editor as the command line and
/// config.toml ask (reads files).
pub fn app_for(l: &Launch) -> app::App {
    let Some(project) = &l.project else { return app::App::init() };
    let cfg = l.config.as_deref().map(|t| toml::from_str(t).expect("config parses")).unwrap_or_default();
    let mut a = app::App::new(cfg, project.clone(), l.first_launch, None);
    a.to_open = l.files.iter().map(|f| (f.clone(), None)).collect();
    // The tests' projects are new folders, so only a test that saved a session restores one.
    a.restore_session();
    a
}
