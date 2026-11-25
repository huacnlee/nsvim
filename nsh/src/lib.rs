//! editor on NSH. The core (`editor-core`) holds the text and the keymaps. This crate draws
//! it with each platform's own views and carries out the actions the keymaps return.

mod chrome;
mod keys;
mod model;
mod store;
mod text_area;
mod theme;

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
    /// The file to open at start, from the command line.
    pub path: Prop<Option<SharedString>>,
}

/// The root for `cargo run` and the tests: the app with the editor's theme, opening `path`.
pub fn root(path: Option<String>) -> Root {
    Root::new::<App>(move || AppProps { path: Prop::from(path.clone().map(SharedString::from)) })
        .tokens(theme::tokens())
        .app_id("dev.editor.app")
}

/// The editor window: the text area, the status line and the command line. The window takes
/// every key and gives it to the core, as the editor draws its own text and cursor.
#[component]
pub fn App(props: AppProps) -> impl IntoElement {
    let font = format!("{}px {}", theme::CODE_SIZE, theme::CODE_FAMILY);
    // The grid of cells, measured once from the code font. 100 chars average out rounding.
    let cell_w = (measure_text(&font, &"0".repeat(100)).0 / 100.0).max(1.0);
    let cell_h = (theme::CODE_SIZE * theme::CODE_LINE_HEIGHT).round();
    let root = std::env::current_dir().unwrap_or_default();
    let s = Session::new(root, cell_w, cell_h);
    if let Some(path) = props.path.get_untracked() {
        s.open(PathBuf::from(path.as_str()));
    }
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
        view().flex_1().bg(move || theme::color("background")).text_color(move || theme::color("foreground")).child(
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
                .child(text_area::text_area(s, keys))
                .child(chrome::status_line(s))
                .child(chrome::command_line(s)),
        )
    };
    app_window().id("main").title("editor").content(content)
}
