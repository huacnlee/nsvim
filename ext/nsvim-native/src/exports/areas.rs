//! The parts of the screen that Rust draws, and the modules that give TSX the app's state.
//!
//! Each component reads the session of the `Workspace` around it.

use nsh_core::prelude::*;

use super::current;
use crate::app::Screen as AppScreen;
use crate::ui;

/// The props of [`TextArea`].
#[derive(Debug, Clone, PartialEq, Props)]
pub struct TextAreaProps {
    /// The id of the document the area shows (`TabState.id`): a pane of the dock. Left out,
    /// the area shows the active document.
    pub doc: Prop<Option<f64>>,
    /// The pane shows: it is the displayed tab of its group. A pane that does not show draws
    /// nothing. True when left out.
    #[prop(default = true)]
    pub shown: Prop<bool>,
}

/// The code with its gutter, cursor and selections. Rust draws it, as it changes each key.
/// With `doc`, it is a pane that shows that document. The pane of the active document has
/// the cursor. A press on another pane makes its document the active one.
#[component]
pub fn TextArea(props: TextAreaProps) -> impl IntoElement {
    let (s, keys) = current();
    let doc = props.doc.get_untracked().map(|d| d as u64);
    let shown = props.shown;
    ui::text_area::text_area(s, keys, doc, computed(move || shown.get()))
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ScreenState {
    /// First-launch setup shows, in place of the editor.
    pub setup: bool,
    /// Settings shows, over the whole editor.
    pub settings: bool,
    /// The file tree shows beside the text.
    pub tree: bool,
}

/// Which screen and panels the window shows.
#[module]
pub struct Shell;

#[module]
impl Shell {
    /// Calls `listener` with what the window shows, now and after each change.
    pub fn on_change(#[args(screen)] listener: Callback<ScreenState>) -> Stop {
        crate::bridge::subscribe(
            |a| ScreenState { setup: matches!(a.screen, AppScreen::Setup(_)), settings: a.on_settings, tree: a.tree.show },
            listener,
        )
    }

    /// Gives the focus back to the view that takes every key. Call it after a press on a
    /// screen, so that the next key goes to the editor.
    pub fn focus_keys() {
        crate::bridge::send(|_| current().1.focus());
    }

    /// The user asked to close the window. The app decides: it quits, or warns about unsaved
    /// edits first.
    pub fn close_requested() {
        crate::bridge::send(|s| {
            s.close_requested();
            super::refocus_keys();
        });
    }
}
