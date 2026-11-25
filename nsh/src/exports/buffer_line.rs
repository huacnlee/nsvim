//! `Tabs`: the state of the buffer line for `ui/buffer-line.tsx`, and the changes its clicks
//! ask for.

use nsh_core::prelude::*;

use crate::app::App;

/// One tab.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TabState {
    /// The name on the tab: a file name, or `Settings`.
    pub name: String,
    /// What the tab shows has unsaved changes.
    pub dirty: bool,
    /// The tab shows.
    pub on: bool,
    /// The tab has a close button.
    pub closable: bool,
}

/// The project folder over the tree, while the tree shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct ProjectState {
    /// The folder, with `~` for the home folder.
    pub path: String,
    /// The width of the tree, in points.
    pub width: f32,
}

/// What the buffer line shows.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct BufferLineState {
    /// The project folder, while the tree shows.
    pub project: Option<ProjectState>,
    /// The tabs, in order: one for each open file, then Settings while it is open.
    pub tabs: Vec<TabState>,
    /// The font family of the interface.
    pub font_family: String,
    /// The text size of the interface, in points.
    pub font_size: f32,
}

/// The buffer line's state.
pub fn state_of(a: &App) -> BufferLineState {
    BufferLineState {
        project: (a.tree.show && !a.on_settings).then(|| ProjectState { path: crate::app::tilde(&a.ed.root), width: a.tree.width }),
        tabs: a
            .ed
            .docs
            .iter()
            .enumerate()
            .map(|(i, d)| TabState { name: d.buf.name(), dirty: d.buf.is_dirty(), on: i == a.ed.active && !a.on_settings, closable: true })
            .chain(a.settings.is_some().then(|| TabState {
                name: "Settings".into(),
                dirty: a.cfg != a.disk_cfg,
                on: a.on_settings,
                closable: false,
            }))
            .collect(),
        font_family: if a.cfg.ui.font == "system" { "system-ui".into() } else { a.font_family.into() },
        font_size: a.cfg.ui.size,
    }
}

/// The tabs of the open files and of Settings, above the text.
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
        crate::bridge::update(move |a, _| {
            let i = index as usize;
            // The Settings tab comes after the files.
            if i == a.ed.docs.len() && a.settings.is_some() {
                a.on_settings = true;
            } else {
                a.show_tab(i);
            }
        });
    }

    /// Closes tab `index` when it has a close button. A file with unsaved changes stays open
    /// and the editor tells why.
    pub fn close(index: u32) {
        crate::bridge::update(move |a, _| {
            if (index as usize) < a.ed.docs.len() {
                a.close_tab(index as usize);
            }
        });
    }
}
