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

use nsh_core::KeyCapture;
use nsh_core::element::Root;
use nsh_core::element::desktop::window as app_window;
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
}

/// What a root opens: the editor as the command line asks, or a set project for tests.
#[derive(Clone, Debug, Default)]
pub struct Launch {
    pub project: Option<PathBuf>,
    pub config: Option<String>,
    pub files: Vec<PathBuf>,
    pub first_launch: bool,
}

/// The root for `cargo run`: the app as the command line and config.toml ask.
pub fn root() -> Root {
    root_with(Launch::default())
}

/// A root for a set project, config and files.
pub fn root_with(l: Launch) -> Root {
    let props = move || AppProps {
        project: Prop::from(l.project.as_ref().map(|p| SharedString::from(p.to_string_lossy().into_owned()))),
        config: Prop::from(l.config.clone().map(SharedString::from)),
        files: Prop::from(
            (!l.files.is_empty()).then(|| SharedString::from(l.files.iter().map(|f| f.to_string_lossy()).collect::<Vec<_>>().join("\n"))),
        ),
        first_launch: Prop::from(l.first_launch),
    };
    Root::new::<App>(props).app_id("dev.editor.app")
}

/// The app state the props ask for.
fn make_app(props: &AppProps) -> app::App {
    let Some(project) = props.project.get_untracked() else { return app::App::init() };
    let cfg = props.config.get_untracked().map(|t| toml::from_str(t.as_str()).expect("config parses")).unwrap_or_default();
    let mut a = app::App::new(cfg, PathBuf::from(project.as_str()), props.first_launch.get_untracked(), None);
    let files = props.files.get_untracked().unwrap_or_default();
    a.to_open = files.lines().filter(|l| !l.is_empty()).map(|l| (PathBuf::from(l), None)).collect();
    a
}

/// The editor window. The window takes every key and gives it to the app, as the editor draws
/// its own text and cursor.
#[component]
pub fn App(props: AppProps) -> impl IntoElement {
    let s = Session::new(make_app(&props));
    // The colours follow config.toml's theme, the setup's pick and the system appearance.
    let light = s.view(|a| ui::palette(a, false));
    let dark_palette = s.view(|a| ui::palette(a, true));
    effect(move || {
        let p = if dark() { dark_palette.get() } else { light.get() };
        set_tokens(ui::style::tokens(&p));
    });
    s.start();
    let keys = element_ref();
    let has_focus = stored(false);
    // A view takes the focus only once its window shows, which comes after the first layout on
    // Linux. Ask until it holds the focus, for at most 5 seconds.
    spawn(async move {
        for _ in 0..50 {
            if has_focus.with(|f| *f) {
                break;
            }
            keys.focus();
            timer(std::time::Duration::from_millis(100)).await;
        }
    });
    // The window's root paints the background. The view inside it takes the keys.
    let content = move || {
        view().flex_1().bg(move || ui::style::color("background")).text_color(move || ui::style::color("foreground")).child(
            view()
                .flex_1()
                .element_ref(&keys)
                .focusable(true)
                .capture_keys(KeyCapture::All)
                .on_focus(move || has_focus.with(|f| *f = true))
                .on_blur(move || has_focus.with(|f| *f = false))
                .on_key_down(move |e: &KeyEvent| {
                    if let Some(k) = keys::convert(e, apple_keys()) {
                        s.key(k);
                    }
                })
                .child(ui::shell(s, keys)),
        )
    };
    app_window().id("main").title("editor").content(content)
}
