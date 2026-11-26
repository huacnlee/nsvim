//! What the keymaps ask for outside the text: saving, closing, opening files and pickers,
//! the clipboard and `:set`.

use std::path::PathBuf;

use super::fx::Tx;
use editor::editor::{Action, Keymap};

use super::{App, Overlay, PickKind, Settings};
use crate::services::{AfterSave, WriteKind};

impl App {
    pub fn run_actions(&mut self, actions: Vec<Action>, tx: &mut Tx) {
        let mut close_after_save = false;
        for a in &actions {
            if matches!(a, Action::Quit { force: false }) && actions.iter().any(|b| matches!(b, Action::Save(_))) {
                close_after_save = true;
            }
        }
        // `:wqa` and `:xa`: quit once the writes are done, not before.
        let quit_after_all =
            actions.iter().any(|a| matches!(a, Action::SaveAll)) && actions.iter().any(|a| matches!(a, Action::QuitAll { force: false }));
        for a in actions {
            match a {
                Action::Save(path) => self.save_active(path, if close_after_save { AfterSave::Close } else { AfterSave::Nothing }),
                Action::Quit { .. } if close_after_save => {}
                Action::SaveAll if quit_after_all => self.save_all_and_quit(tx),
                Action::QuitAll { force: false } if quit_after_all => {}
                Action::SaveAll => {
                    self.save_all(false);
                }
                Action::Quit { force } => self.close_active(force, true, tx),
                Action::QuitAll { force } => self.quit_all(force, tx),
                Action::Open(path) => self.open_path(path, None),
                Action::OpenAt(path, pos) => self.open_path(path, Some(pos)),
                Action::Rename(name) => self.rename(name),
                Action::CodeActions => self.code_actions(),
                Action::Filter { first, last, cmd } => self.filter(first, last, cmd),
                Action::Shell(cmd) => {
                    self.ed.message(format!(":!{cmd}  running"));
                    if let Some(svc) = &self.svc {
                        svc.shell(cmd);
                    }
                }
                Action::Reload => self.reload_active(),
                Action::FindFile => self.open_picker(PickKind::Files),
                Action::OpenProject => self.open_picker(PickKind::Projects),
                Action::GrepProject => self.open_picker(PickKind::Grep),
                Action::PickBuffer => self.open_picker(PickKind::Buffers),
                Action::Palette => self.open_picker(PickKind::Commands),
                Action::KeyList(query) => self.open_key_list(query),
                Action::ToggleTree => {
                    self.tree.show = !self.tree.show;
                    self.tree.focus = false;
                }
                Action::FocusTree => self.focus_tree(),
                a @ (Action::OpenFolder(_)
                | Action::NewFile(_)
                | Action::NewFolder(_)
                | Action::MovePath(_)
                | Action::DeletePath { .. }) => self.file_action(a, tx),
                Action::CloseBuffer => self.close_active(false, false, tx),
                Action::NextBuffer => self.cycle_buffer(1),
                Action::PrevBuffer => self.cycle_buffer(-1),
                Action::Diagnostics => self.show_diagnostics(),
                Action::GotoDefinition => self.goto_definition(),
                Action::Hover => self.hover_here(),
                Action::Settings => self.open_settings(),
                Action::Setup => self.open_setup(),
                Action::Update => self.run_update(tx),
                Action::LeaderMenu => self.overlay = Overlay::Leader(None),
                Action::Copy(text) => tx.copy(text),
                Action::Paste { before, count } => self.paste(before, count, tx),
                Action::InsertClipboard => tx.read_clipboard(|s: &mut App, content, _tx| {
                    editor::vim::type_text(&mut s.ed, &content.text().replace("\r\n", "\n"));
                    s.edited();
                }),
                Action::SetOption(k, v) => self.set_option(&k, &v),
                Action::Message(m) => self.ed.message(m),
                Action::Error(e) => self.ed.error(e),
            }
        }
    }

    /// Writes the active buffer.
    pub fn save_active(&mut self, path: Option<PathBuf>, then: AfterSave) {
        if !self.ed.has_doc() {
            self.ed.error("E32: No file name");
            return;
        }
        if let Some(p) = path {
            let p = if p.is_absolute() { p } else { self.ed.root.join(p) };
            self.ed.doc_mut().buf.path = Some(p.clone());
            self.meta.entry(p.clone()).or_default().lang = Some(crate::highlight::lang_for(Some(&p)));
        }
        self.save_doc(self.ed.active, then);
    }

    fn save_doc(&mut self, i: usize, then: AfterSave) -> bool {
        let opts = self.ed.opts.save;
        let crlf_new = self.cfg.format.line_endings == "crlf";
        let doc = &mut self.ed.docs[i];
        if doc.buf.disk_mtime.is_none() && crlf_new {
            doc.buf.line_ending = editor::buffer::LineEnding::CrLf;
        }
        if let Some(crlf) = doc.buf.path.as_ref().and_then(|p| self.meta.get(p)).and_then(|m| m.ec.crlf) {
            doc.buf.line_ending = if crlf { editor::buffer::LineEnding::CrLf } else { editor::buffer::LineEnding::Lf };
        }
        let at = doc.view.cursor;
        match doc.buf.prepare_save_at(opts, at) {
            Ok((path, bytes, version)) => {
                let cursor = doc.view.cursor;
                doc.view.cursor = doc.buf.clamp(cursor, false);
                if let Some(svc) = &self.svc {
                    svc.save(path, bytes, version, then);
                }
                true
            }
            Err(e) => {
                self.ed.error(e.to_string());
                false
            }
        }
    }

    /// Writes every modified buffer. With `quit`, quits once the last write lands.
    pub fn save_all(&mut self, quit: bool) -> usize {
        let dirty: Vec<usize> =
            (0..self.ed.docs.len()).filter(|&i| self.ed.docs[i].buf.is_dirty() && self.ed.docs[i].buf.path.is_some()).collect();
        let n = dirty.iter().filter(|&&i| self.save_doc(i, AfterSave::Nothing)).count();
        if quit && n > 0 {
            self.quit_after_saves = Some(n);
        }
        n
    }

    /// `:wqa`: writes every modified buffer, then quits once the writes are
    /// done.
    fn save_all_and_quit(&mut self, tx: &mut Tx) {
        if let Some(i) = self.ed.docs.iter().position(|d| d.buf.is_dirty() && d.buf.path.is_none()) {
            self.ed.error(format!("E141: No file name for buffer {}", i + 1));
            return;
        }
        let n = self.save_all(false);
        if n == 0 {
            self.quit_all(false, tx);
        } else {
            self.quit_after_saves = Some(n);
        }
    }

    /// One of the writes `:wqa` waits for is done: quit after the last one.
    pub fn saved_one(&mut self, tx: &mut Tx) {
        if let Some(n) = self.quit_after_saves.as_mut() {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.quit(tx);
            }
        }
    }

    /// `:q` closes the buffer and quits after the last one (`quit_last`); `SPC q` only closes.
    fn close_active(&mut self, force: bool, quit_last: bool, tx: &mut Tx) {
        if !self.ed.has_doc() {
            if quit_last {
                self.quit(tx);
            }
            return;
        }
        if !force && self.ed.buf().is_dirty() {
            self.ed.error("E37: No write since last change (add ! to override)");
            return;
        }
        let i = self.ed.active;
        if let (Some(path), Some(svc)) = (self.ed.docs[i].buf.path.clone(), &self.svc) {
            svc.lsp(crate::services::lsp::Job::Close { path: path.clone() });
            svc.unswap(path);
        }
        self.ed.close(i);
        self.ed.mode = self.ed.rest_mode();
        if self.ed.docs.is_empty() && quit_last {
            self.quit(tx);
            return;
        }
        self.apply_doc_opts();
    }

    pub(crate) fn quit_all(&mut self, force: bool, tx: &mut Tx) {
        if !force {
            let dirty: Vec<String> = self.ed.docs.iter().filter(|d| d.buf.is_dirty()).map(|d| d.buf.name()).collect();
            if let Some(first) = dirty.first() {
                let more = if dirty.len() > 1 { format!(" and {} more", dirty.len() - 1) } else { String::new() };
                self.ed.error(format!("E37: No write since last change for \"{first}\"{more} (add ! to override)"));
                return;
            }
        }
        // The process ends right after this, before a worker could run: remove the swap files
        // of the edits `:qa!` drops here.
        for d in self.ed.docs.iter().filter(|d| d.buf.is_dirty()) {
            if let Some(p) = &d.buf.path {
                let _ = std::fs::remove_file(crate::services::io::swap_file(p));
            }
        }
        self.quit(tx);
    }

    fn reload_active(&mut self) {
        if !self.ed.has_doc() {
            return;
        }
        if let (Some(path), Some(svc)) = (self.ed.buf().path.clone(), &self.svc) {
            // `:e!` drops local edits: mark the buffer clean so the reload applies.
            let v = self.ed.buf().version();
            self.ed.doc_mut().buf.mark_saved(v, None);
            svc.unswap(path.clone());
            svc.read_disk(path);
        }
    }

    fn cycle_buffer(&mut self, by: isize) {
        let n = self.ed.docs.len();
        if n == 0 {
            return;
        }
        self.ed.active = (self.ed.active as isize + by).rem_euclid(n as isize) as usize;
        self.apply_doc_opts();
        self.ed.mode = self.ed.rest_mode();
    }

    fn paste(&mut self, before: bool, count: usize, tx: &mut Tx) {
        tx.read_clipboard(move |s: &mut App, content, _tx| {
            let text = content.text().replace("\r\n", "\n");
            let (text, linewise) = if text.is_empty() || text == s.ed.regs.unnamed.text {
                (s.ed.regs.unnamed.text.clone(), s.ed.regs.unnamed.linewise)
            } else {
                let lw = text.ends_with('\n') && s.ed.keymap == Keymap::Vim;
                (text, lw)
            };
            if !text.is_empty() && s.ed.has_doc() {
                editor::vim::paste_text(&mut s.ed, &text, linewise, before, count);
                s.edited();
            }
        });
    }

    /// `:set name=value`, the palette's toggles and Settings' keymap switch. The value `?` shows
    /// the option, and `!` flips a switch.
    pub fn set_option(&mut self, k: &str, v: &str) {
        // The core applied these. They last for the session, outside config.toml.
        if matches!(k, "hlsearch" | "ignorecase" | "smartcase") {
            return;
        }
        if v == "?" {
            match self.option_value(k) {
                Some(val) => self.ed.message(format!("  {k}={val}")),
                None => self.ed.error(format!("E518: Unknown option: {k}")),
            }
            return;
        }
        let flipped;
        let v = if v == "!" {
            flipped = match self.option_value(k).as_deref() {
                Some("true") => "false",
                Some("false") => "true",
                Some(_) => {
                    self.ed.error(format!("E488: Trailing characters: {k}!"));
                    return;
                }
                None => {
                    self.ed.error(format!("E518: Unknown option: {k}"));
                    return;
                }
            };
            flipped
        } else {
            v
        };
        let on = !matches!(v, "false" | "0" | "off" | "no");
        match k {
            "keymap" | "keys.map" => self.cfg.keys.map = if v == "classic" { "classic".into() } else { "neovim".into() },
            // A mode (`light`, `dark`, `system`), or a built-in theme by id.
            "theme" => match v {
                "light" | "dark" | "system" => self.cfg.theme.mode = v.into(),
                // The old Omarchy mode is System, which follows the Omarchy theme where it runs.
                "omarchy" => {
                    self.cfg.theme.mode = "system".into();
                    self.config_changed();
                    self.ed.message("theme=omarchy is now theme=system. System follows the Omarchy theme");
                    return;
                }
                id if crate::theme::name(id) != id => {
                    let light = crate::theme::builtin(id).light;
                    self.cfg.theme.mode = if light { "light" } else { "dark" }.into();
                    if light {
                        self.cfg.theme.light = id.into();
                    } else {
                        self.cfg.theme.dark = id.into();
                    }
                }
                _ => {
                    self.ed.error(format!("E474: Invalid argument: theme={v}. Use light, dark, system or a theme id"));
                    return;
                }
            },
            "wrap" => self.cfg.editor.wrap = on,
            "number" | "nu" | "numbers" => {
                self.cfg.editor.numbers = if !on {
                    "off".into()
                } else if v == "true" {
                    "absolute".into()
                } else {
                    v.into()
                }
            }
            "relativenumber" | "rnu" => self.cfg.editor.numbers = if on { "hybrid".into() } else { "absolute".into() },
            "cursorline" | "cul" => self.cfg.editor.cursorline = on,
            "list" => self.cfg.editor.whitespace = on,
            "colorcolumn" | "cc" | "ruler" => self.cfg.editor.ruler = v.parse().unwrap_or(0),
            "mouse" => self.cfg.mouse.enabled = on && !v.is_empty(),
            "tabstop" | "ts" | "shiftwidth" | "sw" | "softtabstop" | "sts" => {
                self.cfg.indent.width = v.parse().unwrap_or(self.cfg.indent.width)
            }
            "ligatures" => self.cfg.font.ligatures = on,
            "expandtab" | "et" => self.cfg.indent.spaces = on,
            "scrolloff" | "so" => self.cfg.editor.scrolloff = v.parse().unwrap_or(self.cfg.editor.scrolloff),
            _ => {
                self.ed.error(format!("E518: Unknown option: {k}"));
                return;
            }
        }
        self.config_changed();
    }

    /// An option's value as `:set name?` shows it: `true` or `false` for a switch.
    fn option_value(&self, k: &str) -> Option<String> {
        let c = &self.cfg;
        Some(match k {
            "keymap" | "keys.map" => c.keys.map.clone(),
            "theme" => c.theme.mode.clone(),
            "wrap" => c.editor.wrap.to_string(),
            "number" | "nu" | "numbers" => (c.editor.numbers != "off").to_string(),
            "relativenumber" | "rnu" => matches!(c.editor.numbers.as_str(), "hybrid" | "relative").to_string(),
            "cursorline" | "cul" => c.editor.cursorline.to_string(),
            "list" => c.editor.whitespace.to_string(),
            "colorcolumn" | "cc" | "ruler" => c.editor.ruler.to_string(),
            "mouse" => c.mouse.enabled.to_string(),
            "tabstop" | "ts" | "shiftwidth" | "sw" | "softtabstop" | "sts" => c.indent.width.to_string(),
            "expandtab" | "et" => c.indent.spaces.to_string(),
            "scrolloff" | "so" => c.editor.scrolloff.to_string(),
            "ligatures" => c.font.ligatures.to_string(),
            _ => return None,
        })
    }

    /// Sends lines `first..=last` through `cmd`; the output replaces them when it arrives.
    fn filter(&mut self, first: usize, last: usize, cmd: String) {
        if !self.ed.has_doc() {
            return;
        }
        let buf = self.ed.buf();
        let Some(path) = buf.path.clone() else {
            self.ed.error("E32: No file name");
            return;
        };
        let last = last.min(buf.last_line());
        let a = buf.idx(editor::buffer::Pos::new(first, 0));
        let b = if last < buf.last_line() { buf.idx(editor::buffer::Pos::new(last + 1, 0)) } else { buf.len_chars() };
        let mut input = buf.slice(a, b);
        if !input.ends_with('\n') {
            input.push('\n');
        }
        let version = buf.version();
        self.ed.message(format!(":{},{}!{cmd}  running", first + 1, last + 1));
        if let Some(svc) = &self.svc {
            svc.filter(path, version, first, last, cmd, input);
        }
    }

    /// Shows the file of tab `i` from the mouse: a click on its tab.
    pub fn show_tab(&mut self, i: usize) {
        if i >= self.ed.docs.len() {
            return;
        }
        self.ed.active = i;
        self.on_settings = false;
        self.ed.mode = self.ed.rest_mode();
        self.apply_doc_opts();
    }

    /// Closes tab `i` from the mouse: the close button or a middle-click. A tab with unsaved
    /// changes stays open and shows why.
    pub fn close_tab(&mut self, i: usize) {
        if self.ed.docs.get(i).is_some_and(|d| d.buf.is_dirty()) {
            self.ed.active = i;
            self.ed.error("E37: No write since last change (save first, or :q! to drop it)");
            return;
        }
        if let (Some(path), Some(svc)) = (self.ed.docs.get(i).and_then(|d| d.buf.path.clone()), &self.svc) {
            svc.lsp(crate::services::lsp::Job::Close { path });
        }
        self.ed.close(i);
        self.apply_doc_opts();
    }

    /// Shows Settings over the whole window.
    pub fn open_settings(&mut self) {
        if self.settings.is_none() {
            self.settings = Some(Settings::default());
        }
        self.on_settings = true;
        self.overlay = Overlay::None;
        self.ed.mode = self.ed.rest_mode();
        self.ed.message(":Settings");
    }

    /// Hides Settings. Each change is on disk already.
    pub fn close_settings(&mut self) {
        self.settings = None;
        self.on_settings = false;
    }

    /// Writes config.toml through the io worker. Settings calls it after each change.
    pub fn write_config(&mut self) {
        let text = self.cfg.to_toml();
        self.disk_cfg = self.cfg.clone();
        if let Some(svc) = &self.svc {
            let path = self.config_file.clone().unwrap_or_else(crate::config::config_path);
            svc.write(WriteKind::Config, path, text);
        }
    }
}
