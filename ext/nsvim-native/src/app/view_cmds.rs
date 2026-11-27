//! Small desktop editor commands of the menu and the palette: zoom, Reopen Closed Tab, and the
//! line commands of the Edit menu. They work in both keymaps.

use editor::classic::LineEdit;
use editor::editor::{KeyCode, KeyInput};

use super::App;

/// The smallest and the largest editor font size that zoom gives, in points.
pub const ZOOM_MIN: f32 = 8.0;
pub const ZOOM_MAX: f32 = 40.0;

/// A zoom command: in, out, or back to the default sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zoom {
    In,
    Out,
    Reset,
}

impl Zoom {
    /// The zoom command of `id`, a palette command.
    pub fn from_id(id: &str) -> Option<Zoom> {
        Some(match id {
            "zoom-in" => Zoom::In,
            "zoom-out" => Zoom::Out,
            "zoom-reset" => Zoom::Reset,
            _ => return None,
        })
    }

    /// The zoom key: `Mod =` (or `Mod +`), `Mod -`, `Mod 0`. On macOS `⌃ -` is Back, so there a
    /// key with `Ctrl` is no zoom key.
    pub fn from_key(k: &KeyInput) -> Option<Zoom> {
        if !k.cmd || k.alt || (cfg!(target_os = "macos") && k.ctrl) {
            return None;
        }
        Some(match k.code {
            KeyCode::Char('=' | '+') => Zoom::In,
            KeyCode::Char('-') if !k.shift => Zoom::Out,
            KeyCode::Char('0') if !k.shift => Zoom::Reset,
            _ => return None,
        })
    }
}

/// The line command of `id`, a palette command.
pub fn line_edit_of(id: &str) -> Option<LineEdit> {
    Some(match id {
        "comment" => LineEdit::Comment,
        "line-up" => LineEdit::MoveUp,
        "line-down" => LineEdit::MoveDown,
        "delete-line" => LineEdit::Delete,
        "indent" => LineEdit::Indent,
        "outdent" => LineEdit::Outdent,
        _ => return None,
    })
}

/// Whether `k` is Reopen Closed Tab: `Mod ⇧ T`.
pub fn is_reopen_key(k: &KeyInput) -> bool {
    k.cmd && k.shift && !k.alt && matches!(k.code, KeyCode::Char('t' | 'T'))
}

impl App {
    /// Makes the editor font one point larger or smaller, from 8 to 40 points, or gives back
    /// the default sizes. The interface text changes by the same ratio. config.toml gets the
    /// change at once, as from Settings.
    pub fn zoom(&mut self, z: Zoom) {
        let def = crate::config::Config::default();
        let old = self.cfg.font.size;
        let (font, ui) = match z {
            Zoom::Reset => (def.font.size, def.ui.size),
            Zoom::In | Zoom::Out => {
                let step = if z == Zoom::In { 1.0 } else { -1.0 };
                let font = (old.round() + step).clamp(ZOOM_MIN, ZOOM_MAX);
                // Half points keep the interface text sharp.
                let ui = (self.cfg.ui.size * font / old.max(1.0) * 2.0).round() / 2.0;
                (font, ui.clamp(ZOOM_MIN, ZOOM_MAX))
            }
        };
        if font == old && ui == self.cfg.ui.size {
            let edge = if z == Zoom::Out {
                "smallest"
            } else if z == Zoom::In {
                "largest"
            } else {
                "default"
            };
            self.ed.message(format!("font.size = {font:.1}, the {edge} size"));
            return;
        }
        self.cfg.font.size = font;
        self.cfg.ui.size = ui;
        self.config_changed();
        self.write_config();
        self.ed.message(format!("font.size = {font:.1}"));
    }

    /// Opens the file of the last closed tab again, with the cursor where it was. A file that
    /// is open or gone is passed over, and the one closed before it opens.
    pub fn reopen_closed(&mut self) {
        while let Some((path, at)) = self.ed.closed.pop() {
            if self.doc_index(&path).is_some() || !path.is_file() {
                continue;
            }
            self.open_path(path, Some(at));
            return;
        }
        self.ed.message("no closed tab to reopen");
    }

    /// Runs a line command of the Edit menu on the active file.
    pub fn line_edit(&mut self, op: LineEdit) {
        let before = self.ed.buf().version();
        editor::classic::line_edit(&mut self.ed, op);
        if self.ed.buf().version() != before {
            self.edited();
        }
    }
}
