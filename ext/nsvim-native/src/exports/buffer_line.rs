//! `Tabs`: the state of the buffer line for `src/buffer-line.tsx`, and the changes its clicks
//! ask for.

use nsh_core::prelude::*;

use crate::app::App;

/// One tab.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TabState {
    /// The document's id, which stays the same while the file is open: the key of its panel in
    /// the dock, and the `doc` of its `TextArea`.
    pub id: f64,
    /// The name on the tab: the file name.
    pub name: String,
    /// The file has unsaved changes.
    pub dirty: bool,
    /// The tab shows.
    pub on: bool,
}

/// The project folder over the tree, while the tree shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ProjectState {
    /// The folder, with `~` for the home folder.
    pub path: String,
    /// The width of the tree, in points.
    pub width: f32,
}

/// A pane command for the panes' screen to carry out once (`App::ask_pane`).
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PaneAsk {
    /// The ask's number: a new number is a new ask.
    pub seq: f64,
    /// What to do to the pane of the active file: `split-right`, `split-down`, `pane-next`,
    /// `pane-prev` or `pane-close`.
    pub cmd: String,
}

/// What the buffer line shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct BufferLineState {
    /// The project folder, while the tree shows.
    pub project: Option<ProjectState>,
    /// The tabs, in order: one for each open file.
    pub tabs: Vec<TabState>,
    /// The last pane command.
    pub pane: Option<PaneAsk>,
    /// The font family of the interface.
    pub font_family: String,
    /// The text size of the interface, in points.
    pub font_size: f32,
}

/// The buffer line's state.
pub fn state_of(a: &App) -> BufferLineState {
    BufferLineState {
        project: a.tree.show.then(|| ProjectState { path: crate::app::tilde(&a.ed.root), width: a.tree.width }),
        tabs: a
            .ed
            .docs
            .iter()
            .enumerate()
            .map(|(i, d)| TabState { id: d.id as f64, name: d.buf.name(), dirty: d.buf.is_dirty(), on: i == a.ed.active })
            .collect(),
        pane: a.pane_ask.map(|(n, c)| PaneAsk { seq: n as f64, cmd: c.id().into() }),
        font_family: if a.cfg.ui.font == "system" { "system-ui".into() } else { a.font_family.into() },
        font_size: a.cfg.ui.size,
    }
}

/// The tabs of the open files, above the text.
#[module]
pub struct Tabs;

#[module]
impl Tabs {
    /// Calls `listener` with what the buffer line shows, now and after each change.
    pub fn on_change(#[args(state)] listener: Callback<BufferLineState>) -> Stop {
        crate::bridge::subscribe(state_of, listener)
    }

    /// Shows tab `index`.
    pub fn show(index: u32) {
        crate::bridge::update(move |a, _| a.show_tab(index as usize));
    }

    /// Shows the file with id `id` (`TabState.id`) and gives it the keys: a pane's tab, by its
    /// key in the dock.
    pub fn show_file(id: f64) {
        crate::bridge::update(move |a, _| {
            if let Some(i) = a.ed.docs.iter().position(|d| d.id == id as u64) {
                a.show_tab(i);
            }
        });
    }

    /// Opens the menu of tab `index` at `x`, `y` in the window: close it, the others, the tabs
    /// to its left or right, the clean tabs or all, copy its path, or reveal it in the tree.
    pub fn open_menu(index: u32, x: f64, y: f64) {
        crate::bridge::update(move |a, tx| a.tab_menu(index as usize, (x as f32, y as f32), tx));
    }

    /// Closes tab `index`. For a file with unsaved changes, a dialog asks to save them, drop
    /// them or keep the tab.
    pub fn close(index: u32) {
        crate::bridge::update(move |a, tx| {
            if (index as usize) < a.ed.docs.len() {
                a.close_tab(index as usize, tx);
            }
        });
    }

    /// Moves tab `from` to place `to`, which counts the places without `from`: the drop of a
    /// dragged tab. The tab that shows stays the one that shows.
    pub fn move_tab(from: u32, to: u32) {
        crate::bridge::update(move |a, _| a.ed.move_doc(from as usize, to as usize));
    }

    /// Shows `text` on the command line: why a pane command did nothing.
    pub fn say(text: String) {
        crate::bridge::update(move |a, _| a.ed.message(text));
    }

    /// Puts the open files in the order of `ids` (`TabState.id`): the order of the tabs across
    /// the panes, so that the next and previous buffer follow what shows.
    pub fn order(ids: Vec<f64>) {
        let ids: Vec<u64> = ids.into_iter().map(|i| i as u64).collect();
        crate::bridge::update(move |a, _| a.ed.order_docs(&ids));
    }
}
