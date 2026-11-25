//! The pickers that share the finder overlay: files (`SPC f`, ⌘P), project search (`SPC /`,
//! ⇧⌘F), open buffers (`SPC b`), the command palette (⇧⌘P) and the key list (`F1`).

use std::path::PathBuf;

use super::fx::Tx;
use editor::buffer::Pos;
use editor::editor::{Action, KeyCode, KeyInput};

use super::{App, Overlay};
use crate::fuzzy::{self, Match};

/// Rows the finder ranks and draws at most.
const LIMIT: usize = 200;
/// The height of one row of the list, in points.
pub const ROW_H: f32 = 22.0;
/// Rows that show until the list reports how many fit. The tallest list fits fewer.
const FIT: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Files,
    Grep,
    Buffers,
    Commands,
    /// References, definitions or diagnostics from the language server.
    Locations,
    /// The language server's code actions at the cursor.
    Actions,
    /// Every key of the keymap, from `docs/keymaps.md`.
    Keys,
}

impl PickKind {
    pub fn title(self) -> &'static str {
        match self {
            PickKind::Files => "Find files",
            PickKind::Grep => "Find in project",
            PickKind::Buffers => "Buffers",
            PickKind::Commands => "Commands",
            PickKind::Locations => "Locations",
            PickKind::Actions => "Code actions",
            PickKind::Keys => "Keys",
        }
    }
}

pub struct Picker {
    pub kind: PickKind,
    pub query: String,
    pub sel: usize,
    pub matches: Vec<Match>,
    /// How many candidates match, before the limit.
    pub total: usize,
    /// The candidates for Buffers and Commands (Files ranks the project index).
    pub items: Vec<String>,
    /// What each Buffers item opens: a document index.
    pub targets: Vec<usize>,
    /// The file the preview shows and its text, once the file worker read it.
    pub preview: Option<(PathBuf, String)>,
    /// The first row on screen, as the last key or wheel left it.
    pub top: usize,
    /// Rows that fit in the list. Zero until the list reports its height.
    pub fit: usize,
    /// The list scrolls to show the picked row. A key sets it, the wheel clears it.
    pub follow: bool,
    /// The part of a row the wheel has turned but not scrolled.
    pub wheel_rest: f32,
}

/// The command palette: a label and what it does.
pub const COMMANDS: &[(&str, &str)] = &[
    ("Find file…", "find"),
    ("Find in project…", "grep"),
    ("Save", "save"),
    ("Save all", "save-all"),
    ("Toggle file tree", "tree"),
    ("Open settings…", "settings"),
    ("Close tab", "close"),
    ("Next tab", "next"),
    ("Previous tab", "prev"),
    ("Keymap: Neovim", "keymap=neovim"),
    ("Keymap: classic", "keymap=classic"),
    ("Theme: dark", "theme=dark"),
    ("Theme: light", "theme=light"),
    ("Theme: follow the OS", "theme=system"),
    ("Toggle soft wrap", "wrap"),
    ("Show every key…", "keys"),
    ("Agent: list outside changes…", "outside"),
    ("Agent: clear change marks", "clear-outside"),
    ("Agent: show the next proposal", "proposal"),
    ("Quit", "quit"),
];

impl Picker {
    pub fn new(kind: PickKind) -> Picker {
        Picker {
            kind,
            query: String::new(),
            sel: 0,
            matches: Vec::new(),
            total: 0,
            items: Vec::new(),
            targets: Vec::new(),
            preview: None,
            top: 0,
            fit: 0,
            follow: true,
            wheel_rest: 0.0,
        }
    }

    /// Rows that show: as many as fit, or `FIT` until the list reports its height.
    pub fn shown(&self) -> usize {
        if self.fit == 0 { FIT } else { self.fit }
    }

    /// The first row on screen when the list has `len` rows: the scroll position, moved the
    /// least that shows the picked row while `follow` is set. The view draws from here, so it
    /// stays a pure function of the state.
    pub fn view_top(&self, len: usize) -> usize {
        let n = self.shown();
        let mut top = self.top;
        if self.follow {
            if self.sel < top {
                top = self.sel;
            } else if self.sel >= top + n {
                top = self.sel + 1 - n;
            }
        }
        top.min(len.saturating_sub(n))
    }

    /// Stores the first row on screen as the scroll position, before a key or the wheel moves it.
    pub fn settle(&mut self, len: usize) {
        self.top = self.view_top(len);
    }

    /// Scrolls the list by `dy` points (positive: toward the end), and leaves the picked row
    /// where it is. A trackpad's small steps add up until they make a whole row.
    pub fn wheel(&mut self, dy: f32, len: usize) {
        self.settle(len);
        self.follow = false;
        self.wheel_rest += dy / ROW_H;
        let rows = self.wheel_rest.trunc();
        self.wheel_rest -= rows;
        let max = len.saturating_sub(self.shown());
        self.top = (self.top as isize + rows as isize).clamp(0, max as isize) as usize;
    }

    /// Ranks the candidates against the query again.
    pub fn refilter(&mut self, files: &[String]) {
        let (matches, total) = match self.kind {
            PickKind::Files => fuzzy::rank(&self.query, files, LIMIT),
            // The key list shows every key, so a scroll reaches the last one.
            PickKind::Keys if self.query.is_empty() => {
                let all: Vec<Match> = (0..self.items.len()).map(|index| Match { index, score: 0, hits: Vec::new() }).collect();
                (all, self.items.len())
            }
            PickKind::Keys => {
                let m = super::key_list::filter(&self.query, &self.items);
                let n = m.len();
                (m, n)
            }
            // With nothing typed, buffers, commands and code actions keep their own order.
            PickKind::Buffers | PickKind::Commands | PickKind::Actions if self.query.is_empty() => {
                let all: Vec<Match> = (0..self.items.len().min(LIMIT)).map(|index| Match { index, score: 0, hits: Vec::new() }).collect();
                let n = self.items.len();
                (all, n)
            }
            PickKind::Buffers | PickKind::Commands | PickKind::Actions => fuzzy::rank(&self.query, &self.items, LIMIT),
            PickKind::Grep | PickKind::Locations => (Vec::new(), 0),
        };
        self.matches = matches;
        self.total = total;
        self.sel = self.sel.min(self.matches.len().saturating_sub(1));
    }

    /// Rows shown now: grep hits or locations for those kinds, else the ranked matches.
    pub fn len(&self, list_len: usize) -> usize {
        match self.kind {
            PickKind::Grep | PickKind::Locations => list_len.min(LIMIT * 5),
            _ => self.matches.len(),
        }
    }
}

impl App {
    /// Rows the open picker lists now, or zero.
    pub fn pick_rows(&self) -> usize {
        let Overlay::Pick(p) = &self.overlay else { return 0 };
        p.len(if p.kind == PickKind::Locations { self.locations.len() } else { self.grep.hits.len() })
    }

    pub fn open_picker(&mut self, kind: PickKind) {
        let mut p = Picker::new(kind);
        // A picker that opens from another (the palette's key list) keeps its list's size.
        if let Overlay::Pick(old) = &self.overlay {
            p.fit = old.fit;
        }
        match kind {
            PickKind::Buffers => {
                for (i, d) in self.ed.docs.iter().enumerate() {
                    let name = d.buf.path.as_deref().map(|p| self.rel(p)).unwrap_or_else(|| "[No Name]".into());
                    let mark = if d.buf.is_dirty() { " [+]" } else { "" };
                    p.items.push(format!("{name}{mark}"));
                    p.targets.push(i);
                }
                p.sel = self.ed.active;
            }
            PickKind::Commands => p.items = COMMANDS.iter().map(|(l, _)| (*l).to_string()).collect(),
            PickKind::Grep => {
                self.grep.hits.clear();
                self.grep.error = None;
            }
            PickKind::Keys => p.items = super::key_list::items(self.is_vim()),
            PickKind::Files | PickKind::Locations | PickKind::Actions => {}
        }
        p.refilter(&self.files);
        self.overlay = Overlay::Pick(Box::new(p));
        self.ask_preview();
    }

    /// Keys while a picker is open. Always takes the key.
    pub fn picker_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let rows = self.pick_rows();
        let Overlay::Pick(p) = &mut self.overlay else { return };
        p.settle(rows);
        p.follow = true;
        let mut requery = false;
        match k.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                return;
            }
            KeyCode::Enter => {
                self.pick_accept(tx);
                return;
            }
            KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k') if k.code == KeyCode::Up || k.ctrl => p.sel = p.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j') if k.code == KeyCode::Down || k.ctrl => {
                p.sel = (p.sel + 1).min(rows.saturating_sub(1))
            }
            KeyCode::Tab if k.shift => p.sel = p.sel.saturating_sub(1),
            KeyCode::Tab => p.sel = (p.sel + 1).min(rows.saturating_sub(1)),
            KeyCode::PageUp => p.sel = p.sel.saturating_sub(10),
            KeyCode::PageDown => p.sel = (p.sel + 10).min(rows.saturating_sub(1)),
            KeyCode::Backspace => {
                p.query.pop();
                requery = true;
            }
            KeyCode::Char('u') if k.ctrl => {
                p.query.clear();
                requery = true;
            }
            KeyCode::Char('w') if k.ctrl => {
                let t = p.query.trim_end().len();
                let cut = p.query[..t].rfind([' ', '/', '_', '-', '.']).map_or(0, |i| i + 1);
                p.query.truncate(cut);
                requery = true;
            }
            KeyCode::Char(c) if !k.ctrl && !k.cmd && p.kind != PickKind::Locations => {
                p.query.push(c);
                requery = true;
            }
            KeyCode::Char('j') if p.kind == PickKind::Locations => p.sel = (p.sel + 1).min(rows.saturating_sub(1)),
            KeyCode::Char('k') if p.kind == PickKind::Locations => p.sel = p.sel.saturating_sub(1),
            _ => {}
        }
        if requery {
            p.sel = 0;
            if p.kind == PickKind::Grep {
                self.grep.generation += 1;
                self.grep.running = !p.query.is_empty();
                if let Some(svc) = &self.svc {
                    svc.grep(p.query.clone(), self.grep.generation);
                }
            } else {
                p.refilter(&self.files);
            }
        }
        self.ask_preview();
    }

    /// Opens the key list, with `query` typed in its search line.
    pub fn open_key_list(&mut self, query: String) {
        self.open_picker(PickKind::Keys);
        if let Overlay::Pick(p) = &mut self.overlay {
            p.query = query;
            p.refilter(&self.files);
        }
    }

    /// Lists code actions to pick from.
    pub fn open_actions(&mut self, titles: Vec<String>) {
        let mut p = Picker::new(PickKind::Actions);
        if let Overlay::Pick(old) = &self.overlay {
            p.fit = old.fit;
        }
        p.items = titles;
        p.refilter(&self.files);
        self.overlay = Overlay::Pick(Box::new(p));
    }

    /// The selected row's file and line, if it names one.
    pub fn picked(&self) -> Option<(PathBuf, Option<Pos>)> {
        let Overlay::Pick(p) = &self.overlay else { return None };
        match p.kind {
            PickKind::Files => {
                let m = p.matches.get(p.sel)?;
                Some((self.ed.root.join(&self.files[m.index]), None))
            }
            PickKind::Grep => {
                let h = self.grep.hits.get(p.sel)?;
                Some((self.ed.root.join(&h.path), Some(Pos::new(h.line, h.col))))
            }
            PickKind::Buffers => {
                let m = p.matches.get(p.sel)?;
                let i = *p.targets.get(m.index)?;
                Some((self.ed.docs.get(i)?.buf.path.clone()?, None))
            }
            PickKind::Locations => {
                let l = self.locations.get(p.sel)?;
                Some((l.path.clone(), Some(l.pos)))
            }
            PickKind::Commands | PickKind::Actions | PickKind::Keys => None,
        }
    }

    /// Asks the file worker for the selected file's start, unless it is open or shown already.
    pub fn ask_preview(&mut self) {
        let Some((path, _)) = self.picked() else { return };
        let Overlay::Pick(p) = &mut self.overlay else { return };
        if p.preview.as_ref().is_some_and(|(pp, _)| *pp == path) {
            return;
        }
        if let Some(i) = self.ed.docs.iter().position(|d| d.buf.path.as_deref() == Some(path.as_path())) {
            p.preview = Some((path, self.ed.docs[i].buf.slice(0, 64 * 1024)));
            return;
        }
        if let Some(svc) = &self.svc {
            svc.preview(path);
        }
    }

    pub fn pick_accept(&mut self, tx: &mut Tx) {
        let Overlay::Pick(p) = &self.overlay else { return };
        let kind = p.kind;
        if kind == PickKind::Actions {
            let index = p.matches.get(p.sel).map(|m| m.index);
            self.overlay = Overlay::None;
            if let (Some(index), Some(svc)) = (index, &self.svc) {
                svc.lsp(crate::services::lsp::Job::RunAction { index });
            }
            return;
        }
        if kind == PickKind::Keys {
            self.overlay = Overlay::None;
            return;
        }
        if kind == PickKind::Commands {
            let id = p.matches.get(p.sel).map(|m| COMMANDS[m.index].1);
            self.overlay = Overlay::None;
            if let Some(id) = id {
                self.run_command(id, tx);
            }
            return;
        }
        if kind == PickKind::Buffers
            && let Some(i) = p.matches.get(p.sel).and_then(|m| p.targets.get(m.index)).copied()
        {
            self.overlay = Overlay::None;
            self.ed.active = i.min(self.ed.docs.len().saturating_sub(1));
            self.on_settings = false;
            self.apply_doc_opts();
            return;
        }
        let picked = self.picked();
        self.overlay = Overlay::None;
        if let Some((path, at)) = picked {
            self.open_path(path, at);
        }
    }

    /// Runs a palette command by id.
    pub fn run_command(&mut self, id: &str, tx: &mut Tx) {
        let action = match id {
            "find" => Action::FindFile,
            "grep" => Action::GrepProject,
            "save" => Action::Save(None),
            "save-all" => Action::SaveAll,
            "tree" => Action::ToggleTree,
            "settings" => Action::Settings,
            "close" => Action::CloseBuffer,
            "next" => Action::NextBuffer,
            "prev" => Action::PrevBuffer,
            "wrap" => Action::SetOption("wrap".into(), (!self.cfg.editor.wrap).to_string()),
            "keys" => Action::KeyList(String::new()),
            "outside" => {
                self.show_outside();
                return;
            }
            "clear-outside" => {
                self.clear_outside(None);
                self.ed.message("change marks cleared");
                return;
            }
            "proposal" => {
                match self.proposals.first().map(|p| p.path.clone()) {
                    Some(p) => self.open_path(p, None),
                    None => self.ed.message("no proposals"),
                }
                return;
            }
            "quit" => Action::QuitAll { force: false },
            other => match other.split_once('=') {
                Some((k, v)) => Action::SetOption(k.into(), v.into()),
                None => return,
            },
        };
        self.run_actions(vec![action], tx);
    }
}
