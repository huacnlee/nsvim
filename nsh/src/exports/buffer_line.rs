//! `Tabs`: the state of the buffer line for `ui/buffer-line.tsx`, and the changes its clicks
//! ask for.

use nsh_core::prelude::*;

use crate::app::App;

/// One tab of an open file.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TabState {
    /// The file name.
    pub name: String,
    /// The file has unsaved changes.
    pub dirty: bool,
    /// The tab shows.
    pub on: bool,
}

/// The Settings tab, while it is open.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct SettingsTabState {
    /// The tab shows.
    pub on: bool,
    /// The settings have changes that are not written to config.toml.
    pub dirty: bool,
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
    /// A tab for each open file, in order.
    pub tabs: Vec<TabState>,
    /// The Settings tab, while it is open.
    pub settings: Option<SettingsTabState>,
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
            .map(|(i, d)| TabState { name: d.buf.name(), dirty: d.buf.is_dirty(), on: i == a.ed.active && !a.on_settings })
            .collect(),
        settings: a.settings.is_some().then(|| SettingsTabState { on: a.on_settings, dirty: a.cfg != a.disk_cfg }),
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

    /// Shows the file of tab `index`.
    pub fn show(index: u32) {
        crate::bridge::update(move |a, _| a.show_tab(index as usize));
    }

    /// Closes tab `index`. A file with unsaved changes stays open and the editor tells why.
    pub fn close(index: u32) {
        crate::bridge::update(move |a, _| a.close_tab(index as usize));
    }

    /// Shows the Settings tab.
    pub fn show_settings() {
        crate::bridge::update(|a, _| a.on_settings = true);
    }
}
