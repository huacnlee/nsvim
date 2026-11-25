//! editor on NSH. The core (`editor-core`) holds the text and the keymaps. `app` holds the
//! rest of the editor's state, `services` the workers, and `ui` draws it all with each
//! platform's own views.

pub mod app;
pub mod args;
pub mod color;
pub mod config;
pub mod fuzzy;
pub mod geometry;
pub mod highlight;
pub mod ipc;
pub mod keys;
pub mod model;
pub mod services;
pub mod store;
pub mod theme;
pub mod timing;
pub mod ui;

use std::path::PathBuf;
use std::sync::OnceLock;

use nsh_core::KeyCapture;
use nsh_core::desktop::{QuitMode, WindowAttributes};
use nsh_core::element::Root;
use nsh_core::element::desktop::{current_window, on_app_quit, on_quit_request, set_quit_mode, window as app_window};
use nsh_core::geometry::{Points, Size};
use nsh_core::prelude::*;

pub use store::Session;

/// The app's UI set (`nsh.json` `"rust"`).
#[native_module("editor/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[derive(Clone, PartialEq, Props)]
pub struct AppProps {
    /// The project folder. When set, the app reads no config.toml, session or arguments: it
    /// uses `config` and opens `files`. Tests set it.
    pub project: Prop<Option<SharedString>>,
    /// config.toml's text, with `project`. None is the default config.
    pub config: Prop<Option<SharedString>>,
    /// Files to open, one path a line, with `project`.
    pub files: Prop<Option<SharedString>>,
    /// Show first-launch setup, with `project`.
    #[prop(default)]
    pub first_launch: Prop<bool>,
    /// The config file to read again when it changes, with `project`. None: no reload.
    pub config_file: Prop<Option<SharedString>>,
}

/// What a root opens: the editor as the command line asks, or a set project for tests.
#[derive(Clone, Debug, Default)]
pub struct Launch {
    pub project: Option<PathBuf>,
    pub config: Option<String>,
    pub files: Vec<PathBuf>,
    pub first_launch: bool,
}

/// How the window of a real launch opens: at the size it had when it last closed, where the
/// user left it. Tests open at the default size.
#[derive(Clone, Copy, Debug)]
struct Opening {
    size: (f32, f32),
}

static OPENING: OnceLock<Opening> = OnceLock::new();

/// The window's size in points when no size is kept.
pub const DEFAULT_SIZE: (f32, f32) = (1440.0, 900.0);

/// The least size of the window in points.
pub const MIN_SIZE: (f32, f32) = (640.0, 420.0);

/// The root for `cargo run`: the app as the command line and config.toml ask. Reads the size
/// the window had when it last closed.
pub fn root() -> Root {
    let size = app::session::load_window().map_or(DEFAULT_SIZE, |f| (f.w, f.h));
    let _ = OPENING.set(Opening { size });
    root_with(Launch::default())
}

/// A root for a set project, config and files.
pub fn root_with(l: Launch) -> Root {
    root_inner(l, None)
}

/// `root_with`, and the app reads `config_file` again each time it changes on disk, as a
/// real launch does with config.toml.
pub fn root_with_config_file(l: Launch, config_file: PathBuf) -> Root {
    root_inner(l, Some(config_file))
}

/// The window's attributes for `cfg`: its least size, a transparent background when
/// `[window] opacity` is below 100, and a close button that asks the app first.
pub fn window_attributes(cfg: &config::Config) -> WindowAttributes {
    let mut a = WindowAttributes::default();
    a.min_size = Some(Some(Size::new(Points(MIN_SIZE.0), Points(MIN_SIZE.1))));
    a.transparent = Some(opacity(cfg) < 1.0);
    a.close_on_press = Some(false);
    a
}

/// The background's opacity from `[window] opacity`: 0.7 to 1.
pub fn opacity(cfg: &config::Config) -> f32 {
    (cfg.window.opacity as f32 / 100.0).clamp(0.7, 1.0)
}

fn root_inner(l: Launch, config_file: Option<PathBuf>) -> Root {
    let props = move || AppProps {
        project: Prop::from(l.project.as_ref().map(|p| SharedString::from(p.to_string_lossy().into_owned()))),
        config: Prop::from(l.config.clone().map(SharedString::from)),
        files: Prop::from(
            (!l.files.is_empty()).then(|| SharedString::from(l.files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>().join("\n"))),
        ),
        first_launch: Prop::from(l.first_launch),
        config_file: Prop::from(config_file.as_ref().map(|p| SharedString::from(p.to_string_lossy().into_owned()))),
    };
    Root::new::<App>(props).app_id("dev.nsvim.app")
}

/// The app state the props ask for.
fn make_app(props: &AppProps) -> app::App {
    let l = Launch {
        project: props.project.get_untracked().map(|p| PathBuf::from(p.as_str())),
        config: props.config.get_untracked().map(|t| t.as_str().to_string()),
        files: props.files.get_untracked().unwrap_or_default().lines().filter(|l| !l.is_empty()).map(PathBuf::from).collect(),
        first_launch: props.first_launch.get_untracked(),
    };
    let mut a = app_for(&l);
    if l.project.is_some() {
        a.config_file = props.config_file.get_untracked().map(|f| PathBuf::from(f.as_str()));
    }
    a
}

/// The app state a launch asks for: with a project, that project, config and files, and the
/// last session when no file is given. Without one, the editor as the command line and
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

/// The editor window. The window takes every key and gives it to the app, as the editor draws
/// its own text and cursor.
#[component]
pub fn App(props: AppProps) -> impl IntoElement {
    window(Session::new(make_app(&props)))
}

/// The editor window for session `s`: the theme tokens, the workers, the quit hooks, the view
/// that takes every key, and the screens. `App` calls it. A test root can call it with a
/// session it keeps, to read the app state.
pub fn window(s: Session) -> impl IntoElement {
    // The colours follow config.toml's theme, the setup's pick and the system appearance.
    let light = s.view(|a| ui::palette(a, false));
    let dark_palette = s.view(|a| ui::palette(a, true));
    let alpha = s.view(|a| opacity(&a.cfg));
    effect(move || {
        let mut p = if dark() { dark_palette.get() } else { light.get() };
        // Below full opacity, the background lets the desktop and its blur show through.
        p.bg = p.bg.alpha(p.bg.a * alpha.get());
        set_tokens(ui::style::tokens(&p));
    });
    let attributes = s.view(|a| window_attributes(&a.cfg));
    // The editor quits when its window closes, on macOS too, as it did before.
    set_quit_mode(QuitMode::LastWindowClosed);
    // The platform's quit key and Quit item ask first, as the close button does.
    on_quit_request(move || s.close_requested());
    // However the app ends (the system logs out), the session is written once.
    on_app_quit(move || {
        s.update(|a, _| a.exit());
        async {}
    });
    s.start();
    let keys = element_ref();
    // A view takes the focus only once its window shows, which comes after the first layout on
    // Linux. Ask until it holds the focus, for at most 5 seconds.
    spawn(async move {
        for _ in 0..50 {
            if s.read(|a| a.key_focus) {
                break;
            }
            keys.focus();
            timer(std::time::Duration::from_millis(100)).await;
        }
    });
    // The window's root paints the background once: a translucent one painted twice would be
    // darker. The view inside it takes the keys.
    let content = move || {
        s.attach(current_window());
        let root_bg = move || {
            if alpha.get() < 1.0 { ui::style::swatch(color::Color::TRANSPARENT) } else { ui::style::color("background") }
        };
        view().flex_1().bg(root_bg).text_color(move || ui::style::color("foreground")).child(
            view()
                .flex_1()
                .element_ref(&keys)
                .focusable(true)
                .capture_keys(KeyCapture::All)
                .on_focus(move || s.update(|a, _| a.key_focus = true))
                .on_blur(move || s.update(|a, _| a.key_focus = false))
                .on_key_down(move |e: &KeyEvent| {
                    if let Some(k) = keys::convert(e, apple_keys()) {
                        s.key(k);
                    }
                })
                .child(ui::shell(s, keys)),
        )
    };
    let mut w = app_window().id("main").title("nsvim").attributes(move || attributes.get()).on_close_press(move |_| s.close_requested());
    if let Some(o) = OPENING.get() {
        w = w.default_size(Size::new(Points(o.size.0), Points(o.size.1))).restore_bounds(true);
    }
    w.content(content)
}
