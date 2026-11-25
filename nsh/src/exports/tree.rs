//! `FileTree`: the parts of the app that the file tree (`ui/tree.tsx`) shows, and the changes
//! that it asks for.
//!
//! Rust keeps the scroll position and gives only the rows on screen. The tree can hold
//! 200,000 files, so TSX never sees all of them. The tree's keys go through `App::key`.

use nsh_core::prelude::*;

use crate::app::App;
use crate::app::fx::{Mods, WheelEvent};
use crate::app::tree::{DIR_MARK, ROW_H};

/// Lines one notch of a mouse wheel scrolls.
const WHEEL_LINES: f32 = 3.0;

/// The narrowest and widest the tree can be, in points.
const MIN_W: f32 = 160.0;
const MAX_W: f32 = 640.0;

/// The tree's frame: its width and its font.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TreeFrame {
    /// The width of the tree, in points.
    pub width: f32,
    /// The font family of the interface text.
    pub font: String,
    /// The size of the interface text, in points. It follows config.toml, not the system
    /// text size.
    pub size: f32,
}

/// One row of the tree on screen.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TreeItem {
    /// The path from the project folder, with `/` between names.
    pub path: String,
    /// The name of the file or folder.
    pub name: String,
    /// How many folders are above the row. A top row is at 0.
    pub depth: u32,
    /// The row is a folder.
    pub dir: bool,
    /// The folder is open.
    pub open: bool,
    /// The file shows in the text area.
    pub active: bool,
    /// The keyboard is on this row.
    pub picked: bool,
    /// The git letter of a file: `M`, `U`, `A` or `D`.
    pub git: Option<String>,
    /// The folder has changes inside.
    pub changes: bool,
}

/// The rows on screen.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct TreeRows {
    /// From the scroll position, as many rows as fit and one more.
    pub rows: Vec<TreeItem>,
}

/// The rows on screen: from the scroll position, as many as fit and one more.
pub fn rows_of(app: &App) -> TreeRows {
    let rows = app.tree.index.rows(&app.tree.open);
    let active = if app.ed.has_doc() && !app.on_settings {
        app.ed.buf().path.as_ref().and_then(|p| p.strip_prefix(&app.ed.root).ok()).map(|r| r.to_string_lossy().replace('\\', "/"))
    } else {
        None
    };
    let start = app.tree_top(&rows);
    let rows = rows
        .iter()
        .skip(start)
        .take(app.tree_visible() + 1)
        .map(|r| {
            let git = app.git_letter(r);
            TreeItem {
                path: r.path.clone(),
                name: r.name.clone(),
                depth: r.depth as u32,
                dir: r.dir,
                open: r.open,
                active: active.as_deref() == Some(r.path.as_str()),
                picked: app.tree.focus && app.tree.sel.as_deref() == Some(r.path.as_str()),
                git: git.filter(|c| *c != DIR_MARK).map(String::from),
                changes: git == Some(DIR_MARK),
            }
        })
        .collect();
    TreeRows { rows }
}

fn frame_of(app: &App) -> TreeFrame {
    TreeFrame {
        width: app.tree.width,
        font: if app.cfg.ui.font == "system" { "system-ui".into() } else { app.font_family.into() },
        size: app.cfg.ui.size,
    }
}

/// The file tree: its frame and its rows on screen. A press opens a folder or closes it, or
/// opens a file. The wheel scrolls it. A drag on its edge changes its width.
#[module]
pub struct FileTree;

#[module]
impl FileTree {
    /// Calls `listener` with the tree's frame, now and after each change.
    pub fn on_frame(#[args(frame)] listener: Callback<TreeFrame>) -> Stop {
        crate::bridge::subscribe(frame_of, listener)
    }

    /// Calls `listener` with the rows on screen, now and after each change.
    pub fn on_rows(#[args(rows)] listener: Callback<TreeRows>) -> Stop {
        crate::bridge::subscribe(rows_of, listener)
    }

    /// Opens or closes the folder at `path`, or opens the file at `path`. The keyboard leaves
    /// the tree.
    pub fn press(path: String, dir: bool) {
        crate::bridge::update(move |a, _| {
            a.tree.focus = false;
            a.tree_press(&path, dir);
        });
    }

    /// Scrolls the tree by `dy`: positive scrolls down. `lines` tells that `dy` is in wheel
    /// notches, else it is in points.
    pub fn wheel(dy: f32, lines: bool) {
        let dy = if lines { dy * WHEEL_LINES * ROW_H } else { dy };
        crate::bridge::update(move |a, _| {
            a.tree_wheel(WheelEvent { dx: 0.0, dy: -dy, mods: Mods::default() });
        });
    }

    /// Sets the tree's width, between 160 and 640 points. `done` keeps it in the config. It
    /// does nothing when the mouse is off in the config.
    pub fn resize(width: f32, done: bool) {
        crate::bridge::update(move |a, _| {
            if !a.cfg.mouse.enabled {
                return;
            }
            a.tree.width = width.clamp(MIN_W, MAX_W);
            if done {
                a.cfg.ui.tree_width = a.tree.width;
            }
        });
    }
}
