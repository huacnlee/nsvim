//! The pane commands: split the active pane, give the keys to another pane, close a pane. The
//! layout of the panes belongs to the screen (`src/panes.tsx`, NSH's `DockArea`), so Rust only
//! asks. Each ask has a new number, and the screen carries out each number once.

use editor::editor::PaneCmd;

use super::App;

impl App {
    /// Asks the panes' screen to carry out `cmd` on the pane of the active file. The keys go
    /// back to the text. With no file open, or over Settings, there is no pane.
    pub fn ask_pane(&mut self, cmd: PaneCmd) {
        if !self.ed.has_doc() || self.on_settings {
            self.ed.message("no file is open");
            return;
        }
        self.tree.focus = false;
        let n = self.pane_ask.map_or(1, |(n, _)| n + 1);
        self.pane_ask = Some((n, cmd));
    }
}
