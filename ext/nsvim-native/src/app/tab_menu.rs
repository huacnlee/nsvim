//! The context menu of a tab (a right-click in the buffer line), and the commands that close
//! a group of tabs. The names and the order follow Zed's tab menu.

use super::App;
use super::fx::{PopupItem, PopupMenu, Tx};

/// Which tabs a close command closes, from tab `i`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseTabs {
    /// Every tab but `i`.
    Others,
    /// The tabs before `i`.
    Left,
    /// The tabs after `i`.
    Right,
    /// Every tab with no unsaved changes.
    Clean,
    /// Every tab.
    All,
}

impl CloseTabs {
    /// The command id of the palette and of the menu.
    pub fn from_id(id: &str) -> Option<CloseTabs> {
        Some(match id {
            "close-others" => CloseTabs::Others,
            "close-left" => CloseTabs::Left,
            "close-right" => CloseTabs::Right,
            "close-clean" => CloseTabs::Clean,
            "close-all" => CloseTabs::All,
            _ => return None,
        })
    }

    /// Whether the command closes tab `j`, from tab `i`. A tab with unsaved changes stays open
    /// all the same.
    fn takes(self, i: usize, j: usize) -> bool {
        match self {
            CloseTabs::Others => j != i,
            CloseTabs::Left => j < i,
            CloseTabs::Right => j > i,
            CloseTabs::Clean | CloseTabs::All => true,
        }
    }
}

/// The menu's items: id and title. `-` is a separator. The palette's id for the same command
/// gives the key hint.
pub const TAB_ITEMS: &[(&str, &str)] = &[
    ("close", "Close"),
    ("close-others", "Close others"),
    ("-", ""),
    ("close-left", "Close left"),
    ("close-right", "Close right"),
    ("-", ""),
    ("close-clean", "Close clean"),
    ("close-all", "Close all"),
    ("-", ""),
    ("copy-path", "Copy path"),
    ("copy-relative-path", "Copy relative path"),
    ("reveal", "Reveal in file tree"),
];

impl App {
    /// A right-click on tab `i`: the system's menu at `at`, in the window.
    pub fn tab_menu(&mut self, i: usize, at: (f32, f32), tx: &mut Tx) {
        if i >= self.ed.docs.len() {
            return;
        }
        tx.instant();
        let mut menu = PopupMenu::new();
        for &(id, title) in TAB_ITEMS {
            if id == "-" {
                menu = menu.separator();
                continue;
            }
            let hint = self.tab_item_keys(id);
            let title = if hint.is_empty() { title.to_string() } else { format!("{title}    {hint}") };
            menu = menu.item(PopupItem::new(id, title).enabled(self.tab_item_enabled(i, id)));
        }
        tx.popup_menu(menu, at, move |s: &mut App, picked, tx| {
            if let Some(id) = picked {
                s.tab_menu_pick(i, id, tx);
            }
        });
    }

    /// The keys of the palette command that does what menu item `id` does, in the active keymap.
    fn tab_item_keys(&self, id: &str) -> String {
        super::palette::COMMANDS
            .iter()
            .find(|c| c.id == id)
            .map(|c| c.keys(self.ed.keymap, self.ed.vim.leader, cfg!(target_os = "macos")))
            .unwrap_or_default()
    }

    /// Whether menu item `id` of tab `i` can run: it has tabs to close, or a file on disk.
    pub fn tab_item_enabled(&self, i: usize, id: &str) -> bool {
        let n = self.ed.docs.len();
        let path = self.ed.docs.get(i).and_then(|d| d.buf.path.as_deref());
        match id {
            "close-others" => n > 1,
            "close-left" => i > 0,
            "close-right" => i + 1 < n,
            "close-clean" => self.ed.docs.iter().any(|d| !d.buf.is_dirty()),
            "copy-path" => path.is_some(),
            "copy-relative-path" | "reveal" => path.is_some_and(|p| p.starts_with(&self.ed.root)),
            _ => i < n,
        }
    }

    /// Runs menu item `id` on tab `i`. A disabled item does nothing.
    pub fn tab_menu_pick(&mut self, i: usize, id: &str, tx: &mut Tx) {
        if !self.tab_item_enabled(i, id) {
            return;
        }
        if let Some(which) = CloseTabs::from_id(id) {
            self.close_tabs(i, which);
            return;
        }
        let Some(path) = self.ed.docs.get(i).and_then(|d| d.buf.path.clone()) else {
            if id == "close" {
                self.close_tab(i);
            }
            return;
        };
        match id {
            "close" => self.close_tab(i),
            "copy-path" => self.copy_tab_path(path.display().to_string(), tx),
            "copy-relative-path" => {
                let rel = path.strip_prefix(&self.ed.root).unwrap_or(&path).to_string_lossy().replace('\\', "/");
                self.copy_tab_path(rel, tx);
            }
            "reveal" => {
                self.tree.show = true;
                self.reveal_in_tree(&path);
            }
            _ => {}
        }
    }

    /// Puts `text` on the clipboard, and says so.
    fn copy_tab_path(&mut self, text: String, tx: &mut Tx) {
        self.ed.message(format!("copied {text}"));
        tx.copy(text);
    }

    /// Closes the tabs that `which` takes from tab `i`. A tab with unsaved changes stays open,
    /// and the editor tells which. The tab that showed still shows when it stays open, else tab
    /// `i`, else the tab before it.
    pub fn close_tabs(&mut self, i: usize, which: CloseTabs) {
        let n = self.ed.docs.len();
        if i >= n {
            return;
        }
        let take: Vec<bool> = (0..n).map(|j| which.takes(i, j) && !self.ed.docs[j].buf.is_dirty()).collect();
        let kept: Vec<String> = (0..n).filter(|&j| which.takes(i, j) && !take[j]).map(|j| self.ed.docs[j].buf.name()).collect();
        let want = [self.ed.active, i].into_iter().find(|&j| j < n && !take[j]);
        for j in (0..n).rev().filter(|&j| take[j]) {
            self.drop_doc(j);
        }
        if let Some(w) = want {
            self.ed.active = (0..w).filter(|&j| !take[j]).count();
        }
        self.ed.mode = self.ed.rest_mode();
        if let Some(first) = kept.first() {
            let more = if kept.len() > 1 { format!(" and {} more", kept.len() - 1) } else { String::new() };
            self.ed.error(format!("E37: No write since last change for \"{first}\"{more}"));
        }
        self.apply_doc_opts();
    }

    /// Removes document `j`: the language server and the swap file let it go first.
    pub(crate) fn drop_doc(&mut self, j: usize) {
        if let (Some(path), Some(svc)) = (self.ed.docs.get(j).and_then(|d| d.buf.path.clone()), &self.svc) {
            svc.lsp(crate::services::lsp::Job::Close { path: path.clone() });
            svc.unswap(path);
        }
        self.ed.close(j);
    }
}
