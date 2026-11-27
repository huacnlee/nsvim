//! The find bar over the panes: find and replace in the active file with no Vim syntax. Classic
//! `⌘ F` and `⌘ ⌥ F` (Ctrl on Linux and Windows), Edit › Find… and Replace…, and the palette
//! open it. The terminal editor keeps the search line. Its keys stay in Rust (`App::key`).

use editor::buffer::Pos;
use editor::editor::{KeyCode, KeyInput, Search};
use editor::find::{self, FindOpts};

use super::App;
use super::fx::Tx;

/// The field that takes the typed keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Field {
    #[default]
    Find,
    Replace,
}

/// A toggle of the find bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    Case,
    Word,
    Regex,
}

/// The find bar's state.
#[derive(Clone, Debug, Default)]
pub struct FindBar {
    /// The text to find.
    pub query: String,
    /// The text that replaces a match.
    pub with: String,
    /// The replace field shows.
    pub replace: bool,
    /// The field that takes the keys.
    pub field: Field,
    /// Match case, whole word, regular expression.
    pub opts: FindOpts,
    /// The bar takes the keys. A click on the text gives them to the text, and the bar stays.
    pub keys: bool,
    /// Where the search starts while the user types: the cursor when the bar opened.
    pub origin: Pos,
}

impl FindBar {
    /// The search pattern for the query and the toggles.
    pub fn pattern(&self) -> String {
        find::pattern(&self.query, self.opts)
    }
}

impl App {
    /// Opens the find bar, or gives the keys to its find field (`replace`: its replace field).
    /// A one-line selection, else the word under the cursor, becomes the query.
    pub fn open_find(&mut self, replace: bool) {
        if !self.ed.has_doc() {
            self.ed.message("no file is open");
            return;
        }
        self.overlay = super::Overlay::None;
        if crate::terminal() {
            // The terminal editor keeps the search line and `:%s/`.
            editor::vim::open_cmdline(&mut self.ed, if replace { ':' } else { '/' }, if replace { "%s/" } else { "" });
            return;
        }
        let seed = self.find_seed();
        let origin = {
            let v = &self.ed.doc().view;
            v.anchor.map_or(v.cursor, |a| a.min(v.cursor))
        };
        let bar = self.find.get_or_insert_with(FindBar::default);
        if let Some(q) = seed {
            bar.query = q;
        }
        bar.origin = origin;
        bar.replace |= replace;
        bar.keys = true;
        bar.field = if replace { Field::Replace } else { Field::Find };
        self.find_changed();
    }

    /// The selected text on one line, else the word under the cursor.
    fn find_seed(&self) -> Option<String> {
        let d = self.ed.doc();
        let cur = d.view.cursor;
        if let Some(a) = d.view.anchor {
            let (s, e) = (a.min(cur), a.max(cur));
            if s.line == e.line && s != e {
                let (s, e) = (d.buf.idx(s), d.buf.idx(e));
                return Some(d.buf.slice(s, e));
            }
            return None;
        }
        let (s, e) = editor::vim::word_range_at(&d.buf, cur);
        let word = d.buf.slice(s, e);
        (!word.trim().is_empty() && !word.contains('\n')).then_some(word)
    }

    /// Closes the find bar and its highlight. The keys go back to the text.
    pub fn close_find(&mut self) {
        if self.find.take().is_some() {
            self.ed.vim.hlsearch = false;
        }
    }

    /// The query or a toggle changed: highlight the matches and go to the first one from the
    /// origin.
    fn find_changed(&mut self) {
        let Some(bar) = &self.find else { return };
        let (pattern, origin) = (bar.pattern(), bar.origin);
        self.ed.search = Search { pattern: pattern.clone(), backward: false };
        self.ed.vim.hlsearch = !pattern.is_empty();
        if self.ed.has_doc() && !pattern.is_empty() {
            find::first_from(&mut self.ed, &pattern, origin);
        }
    }

    /// Flips a toggle.
    pub fn find_toggle(&mut self, t: Toggle) {
        let Some(bar) = &mut self.find else { return };
        let o = &mut bar.opts;
        match t {
            Toggle::Case => o.case = !o.case,
            Toggle::Word => o.word = !o.word,
            Toggle::Regex => o.regex = !o.regex,
        }
        self.find_changed();
    }

    /// Goes to the next match, or the one before it.
    pub fn find_step(&mut self, back: bool) {
        let Some(bar) = &self.find else { return };
        let pattern = bar.pattern();
        if self.ed.has_doc() && !pattern.is_empty() && !find::step(&mut self.ed, &pattern, back) {
            self.ed.message("no results");
        }
        if let Some(bar) = &mut self.find {
            bar.origin = self.ed.cursor();
        }
    }

    /// Replaces the match at the cursor and goes to the next one. `all`: every match, as one
    /// undo step.
    pub fn find_replace(&mut self, all: bool) {
        let Some(bar) = &self.find else { return };
        let (pattern, with) = (bar.pattern(), find::replacement(&bar.with, bar.opts.regex));
        if !self.ed.has_doc() || pattern.is_empty() {
            return;
        }
        let before = self.ed.buf().version();
        if all {
            let n = find::replace_all(&mut self.ed, &pattern, &with);
            self.ed.message(match n {
                0 => "no results".to_string(),
                1 => "1 match replaced".to_string(),
                n => format!("{n} matches replaced"),
            });
        } else if !find::replace_one(&mut self.ed, &pattern, &with) {
            self.ed.message("no results");
        }
        if self.ed.buf().version() != before {
            self.edited();
        }
    }

    /// Gives the keys to `field`. The replace field shows.
    pub fn find_focus(&mut self, field: Field) {
        if let Some(bar) = &mut self.find {
            bar.field = field;
            bar.keys = true;
            bar.replace |= field == Field::Replace;
        }
    }

    /// Shows or hides the replace field.
    pub fn find_show_replace(&mut self, on: bool) {
        if let Some(bar) = &mut self.find {
            bar.replace = on;
            bar.keys = true;
            bar.field = if on { Field::Replace } else { Field::Find };
        }
    }

    /// Types `text` into the focused field.
    pub fn find_type(&mut self, text: &str) {
        let Some(bar) = &mut self.find else { return };
        let text = text.replace(['\r', '\n'], " ");
        match bar.field {
            Field::Find => {
                bar.query.push_str(&text);
                self.find_changed();
            }
            Field::Replace => bar.with.push_str(&text),
        }
    }

    /// A key while the find bar shows. False for a shortcut that the bar does not take, so the
    /// keymap gets it.
    pub fn find_key(&mut self, k: KeyInput, tx: &mut Tx) -> bool {
        let Some(bar) = &mut self.find else { return false };
        if !bar.keys {
            // The text has the keys. Esc closes the bar and goes on to the keymap too.
            if k.code == KeyCode::Esc {
                self.close_find();
            }
            return false;
        }
        let letter = match k.code {
            KeyCode::Char(c) => Some(c.to_ascii_lowercase()),
            _ => None,
        };
        match k.code {
            KeyCode::Esc => self.close_find(),
            KeyCode::Enter if bar.field == Field::Replace => self.find_replace(k.cmd),
            KeyCode::Enter => self.find_step(k.shift),
            KeyCode::F(3) => self.find_step(k.shift),
            KeyCode::Tab if bar.replace => {
                bar.field = if bar.field == Field::Find { Field::Replace } else { Field::Find };
            }
            KeyCode::Tab => {}
            KeyCode::Backspace if k.cmd || k.ctrl => {
                self.find_clear();
            }
            KeyCode::Backspace => {
                match bar.field {
                    Field::Find => {
                        bar.query.pop();
                    }
                    Field::Replace => {
                        bar.with.pop();
                    }
                }
                self.find_changed();
            }
            _ if k.cmd && letter == Some('f') => self.open_find(k.alt),
            _ if k.cmd && letter == Some('g') => self.find_step(k.shift),
            _ if k.cmd && letter == Some('v') => {
                tx.read_clipboard(|s: &mut App, clip, _| s.find_type(clip.text()));
            }
            _ if k.cmd && letter == Some('u') => self.find_clear(),
            _ if k.alt && !k.cmd && letter == Some('c') => self.find_toggle(Toggle::Case),
            _ if k.alt && !k.cmd && letter == Some('w') => self.find_toggle(Toggle::Word),
            _ if k.alt && !k.cmd && letter == Some('r') => self.find_toggle(Toggle::Regex),
            KeyCode::Char(_) if k.cmd || k.ctrl => return false,
            KeyCode::Char(c) => self.find_type(&c.to_string()),
            _ if k.cmd || k.ctrl => return false,
            _ => {}
        }
        true
    }

    /// Clears the focused field.
    fn find_clear(&mut self) {
        let Some(bar) = &mut self.find else { return };
        match bar.field {
            Field::Find => bar.query.clear(),
            Field::Replace => bar.with.clear(),
        }
        self.find_changed();
    }
}
