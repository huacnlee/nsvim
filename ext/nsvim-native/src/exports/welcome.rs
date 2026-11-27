//! `Welcome`: the welcome page of a window with no project (`src/welcome.tsx`), and the press
//! on one of its rows. See `app::welcome`.

use nsh_core::prelude::*;

use crate::app::welcome::Row;

/// One row of the welcome page.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct WelcomeRow {
    /// What a press runs. `Welcome.pick` takes it.
    pub id: String,
    pub label: String,
    /// The keys of the command in the active keymap. Empty for a recent project.
    pub keys: String,
}

/// What the welcome page shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct WelcomeState {
    /// The page shows in place of the text: the window has no project and no file is open.
    pub show: bool,
    /// NsVim's version, as `0.2.1`.
    pub version: String,
    /// The first things to do: a new file, a folder, a project, Settings, the keys.
    pub actions: Vec<WelcomeRow>,
    /// The recent projects, newest first.
    pub recent: Vec<WelcomeRow>,
}

fn rows(r: Vec<Row>) -> Vec<WelcomeRow> {
    r.into_iter().map(|r| WelcomeRow { id: r.id, label: r.label, keys: r.keys }).collect()
}

/// The welcome page of a window with no project.
#[module]
pub struct Welcome;

#[module]
impl Welcome {
    /// Calls `listener` with what the welcome page shows, now and after each change.
    pub fn on_change(#[args(state)] listener: Callback<WelcomeState>) -> Stop {
        crate::bridge::subscribe(
            |a| {
                if !a.shows_welcome() {
                    return WelcomeState::default();
                }
                WelcomeState {
                    show: true,
                    version: env!("CARGO_PKG_VERSION").into(),
                    actions: rows(a.welcome_actions()),
                    recent: rows(a.welcome_recent()),
                }
            },
            listener,
        )
    }

    /// Runs the row `id`: a menu item (`cmd:new`) or a recent project (`recent:/path`).
    pub fn pick(id: String) {
        crate::bridge::update(move |a, tx| a.app_menu_pick(&id, tx));
    }
}
