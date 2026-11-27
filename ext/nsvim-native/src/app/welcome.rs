//! The welcome page. A window that starts with no project (a launcher start with no last
//! project, as Vim and Zed do) shows it in place of the text while no file is open. It lists
//! the first things to do and the recent projects. Each row runs a menu item
//! (`App::app_menu_pick`), so a press there does what the same item of the menu does.

use std::path::PathBuf;

use super::App;
use super::palette::COMMANDS;

/// The palette commands the welcome page lists, in order.
const ACTIONS: &[&str] = &["new", "folder", "project", "settings", "keys"];

/// How many recent projects the welcome page lists at most.
const RECENT: usize = 8;

/// One row of the welcome page: what a press runs (`app_menu_pick`), its label and its keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub label: String,
    pub keys: String,
}

impl App {
    /// Makes this window a window with no project: no tree, no session, and the welcome page
    /// with the recent projects. Reads the sessions folder once.
    pub fn start_welcome(&mut self) {
        let recent = super::session::recent(&self.ed.root);
        self.start_welcome_with(recent);
    }

    /// `start_welcome` with the recent projects given.
    pub fn start_welcome_with(&mut self, recent: Vec<PathBuf>) {
        self.tree.show = false;
        self.welcome = Some(recent.into_iter().take(RECENT).collect());
    }

    /// The welcome page shows: the window has no project, no file is open, and Settings does
    /// not cover the editor.
    pub fn shows_welcome(&self) -> bool {
        self.welcome.is_some() && !self.ed.has_doc() && !self.on_settings
    }

    /// The first things to do, with their keys in the active keymap.
    pub fn welcome_actions(&self) -> Vec<Row> {
        let mac = cfg!(target_os = "macos");
        ACTIONS
            .iter()
            .filter_map(|id| COMMANDS.iter().find(|c| c.id == *id))
            .map(|c| Row { id: format!("cmd:{}", c.id), label: c.label.to_string(), keys: c.keys(self.ed.keymap, self.ed.vim.leader, mac) })
            .collect()
    }

    /// The recent projects, newest first, as `~/…`.
    pub fn welcome_recent(&self) -> Vec<Row> {
        let recent = self.welcome.as_deref().unwrap_or_default();
        recent.iter().map(|r| Row { id: format!("recent:{}", r.display()), label: super::project::shown(r), keys: String::new() }).collect()
    }
}
