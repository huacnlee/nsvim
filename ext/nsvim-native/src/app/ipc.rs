//! Requests on the control socket, answered on the UI thread. Lines and columns in the JSON are
//! 1-based, as editors and compilers print them. See `docs/agents.md` for every command.

use std::path::PathBuf;
use std::sync::mpsc::Sender;

use super::fx::Tx;
use editor::buffer::Pos;
use serde_json::{Value, json};

use super::review::Proposal;
use super::{App, Overlay};
use crate::ipc::{Request, Response};

/// A text range an agent pointed at, tinted in the text area with its note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: PathBuf,
    pub first: usize,
    pub last: usize,
    pub text: String,
}

/// `open --wait`: a reply held until the buffer closes.
pub struct Waiter {
    pub id: u64,
    pub path: PathBuf,
    pub reply: Sender<Response>,
    /// The buffer has been open, so its absence now means it closed.
    pub seen: bool,
}

type Res = Result<Value, String>;

fn pos(line: usize, col: usize) -> Value {
    json!({ "line": line + 1, "column": col + 1 })
}

/// A 1-based position from `{"line", "column"}`, to zero-based.
fn read_pos(v: &Value) -> Result<Pos, String> {
    let line = v["line"].as_u64().ok_or("a position needs a line")? as usize;
    let col = v["column"].as_u64().unwrap_or(1) as usize;
    Ok(Pos::new(line.saturating_sub(1), col.saturating_sub(1)))
}

impl App {
    /// Runs the requests a scene scripted (`App::scripted`) whose time has come, as if an agent
    /// sent them over the socket. Scenes use it to record agent demos headless, where no
    /// socket runs. True when one ran.
    pub fn play_scripted(&mut self, clock: f64, tx: &mut Tx) -> bool {
        let mut ran = false;
        while self.scripted.first().is_some_and(|(t, _)| *t <= clock) {
            let (_, req) = self.scripted.remove(0);
            let (reply, _answer) = std::sync::mpsc::channel();
            self.ipc(req, reply, tx);
            ran = true;
        }
        if let Some((t, _)) = self.scripted.first() {
            tx.wake_after(std::time::Duration::from_secs_f64((t - clock).max(0.0)));
        }
        ran
    }

    pub fn ipc(&mut self, req: Request, reply: Sender<Response>, tx: &mut Tx) {
        let id = req.id;
        let a = &req.args;
        let res: Res = match req.cmd.as_str() {
            "ping" => Ok(json!({ "pid": std::process::id(), "root": self.ed.root, "version": env!("CARGO_PKG_VERSION") })),
            "state" => Ok(self.ipc_state()),
            "selection" => Ok(json!({ "text": self.selection_chars().map(|(a, b)| self.ed.buf().slice(a, b)).unwrap_or_default() })),
            "buffer" | "read" => self.ipc_read(a),
            "diagnostics" => Ok(self.ipc_diagnostics(a)),
            "changes" => Ok(self.ipc_changes()),
            "open" => match self.ipc_open(a, id, &reply) {
                Ok(None) => return,
                Ok(Some(v)) => Ok(v),
                Err(e) => Err(e),
            },
            "reveal" => self.ipc_reveal(a),
            "message" => {
                let text = a["text"].as_str().unwrap_or("").to_string();
                if a["error"].as_bool() == Some(true) {
                    self.ed.error(text)
                } else {
                    self.ed.message(text)
                }
                Ok(json!({}))
            }
            "save" => self.ipc_save(a),
            "focus" => {
                self.raise(a);
                Ok(json!({}))
            }
            "edit" => self.ipc_edit(a),
            "propose" => self.ipc_propose(a),
            "command" => self.ipc_command(a, tx),
            other => Err(format!(
                "unknown command `{other}`. Commands: ping, state, selection, read, diagnostics, changes, open, reveal, message, save, edit, propose, command"
            )),
        };
        let _ = reply.send(match res {
            Ok(v) => Response::ok(id, v),
            Err(e) => Response::err(id, e),
        });
        // A request can close a buffer (`command :q`) while the window draws nothing.
        self.check_waiters();
    }

    /// A path from a request, absolute or relative to the project root.
    fn ipc_path(&self, a: &Value) -> Result<PathBuf, String> {
        match a["path"].as_str() {
            Some(p) if !p.is_empty() => {
                let p = PathBuf::from(p);
                Ok(if p.is_absolute() { p } else { self.ed.root.join(p) })
            }
            _ if self.ed.has_doc() => self.ed.buf().path.clone().ok_or_else(|| "the active buffer has no file".into()),
            _ => Err("no path given and no file is open".into()),
        }
    }

    fn ipc_state(&self) -> Value {
        let active = if self.ed.has_doc() && !self.on_settings {
            let d = self.ed.doc();
            let sel = self.selection_chars().map(|(a, b)| {
                let (pa, pb) = (d.buf.pos(a), d.buf.pos(b));
                json!({ "start": pos(pa.line, pa.col), "end": pos(pb.line, pb.col), "text": d.buf.slice(a, b) })
            });
            let path = d.buf.path.clone();
            let lang =
                path.as_ref().and_then(|p| self.meta.get(p)).and_then(|m| m.lang).map(crate::highlight::lang_label).unwrap_or("text");
            json!({
                "path": path,
                "relative": path.as_deref().map(|p| self.rel(p)),
                "language": lang,
                "line": d.view.cursor.line + 1,
                "column": d.view.cursor.col + 1,
                "lines": d.buf.line_count(),
                "dirty": d.buf.is_dirty(),
                "selection": sel,
                "visible": { "first": d.view.top + 1, "last": (d.view.top + self.ed.rows).min(d.buf.line_count()) },
            })
        } else {
            Value::Null
        };
        let buffers: Vec<Value> = self
            .ed
            .docs
            .iter()
            .enumerate()
            .map(|(i, d)| json!({ "path": d.buf.path, "relative": d.buf.path.as_deref().map(|p| self.rel(p)), "dirty": d.buf.is_dirty(), "lines": d.buf.line_count(), "active": i == self.ed.active && !self.on_settings }))
            .collect();
        let (errors, warnings) = self.meta.values().flat_map(|m| m.diags.iter()).fold((0, 0), |(e, w), d| {
            if d.severity <= 1 {
                (e + 1, w)
            } else if d.severity == 2 {
                (e, w + 1)
            } else {
                (e, w)
            }
        });
        json!({
            "root": self.ed.root,
            "mode": self.ed.mode.label(),
            "keymap": if self.is_vim() { "neovim" } else { "classic" },
            "active": active,
            "buffers": buffers,
            "diagnostics": { "errors": errors, "warnings": warnings },
            "outside_changes": self.outside.len(),
            "proposals": self.proposals.len(),
            "settings_open": self.on_settings,
        })
    }

    fn ipc_read(&self, a: &Value) -> Res {
        let path = self.ipc_path(a)?;
        let Some(i) = self.doc_index(&path) else {
            return Err(format!("\"{}\" is not open. Read it from disk, or open it first", self.rel(&path)));
        };
        let buf = &self.ed.docs[i].buf;
        let first = a["start_line"].as_u64().map_or(0, |n| (n as usize).saturating_sub(1));
        let last = a["end_line"].as_u64().map_or(buf.last_line(), |n| (n as usize).saturating_sub(1).min(buf.last_line()));
        let text = if first == 0 && last == buf.last_line() {
            buf.text()
        } else {
            let a = buf.idx(Pos::new(first, 0));
            let b = if last < buf.last_line() { buf.idx(Pos::new(last + 1, 0)) } else { buf.len_chars() };
            buf.slice(a, b)
        };
        Ok(
            json!({ "path": path, "text": text, "dirty": buf.is_dirty(), "lines": buf.line_count(), "start_line": first + 1, "end_line": last + 1 }),
        )
    }

    fn ipc_diagnostics(&self, a: &Value) -> Value {
        let only = a["path"].as_str().and_then(|_| self.ipc_path(a).ok());
        let mut out = Vec::new();
        for (path, m) in &self.meta {
            if only.as_ref().is_some_and(|p| p != path) {
                continue;
            }
            for d in &m.diags {
                let severity = match d.severity {
                    1 => "error",
                    2 => "warning",
                    3 => "information",
                    _ => "hint",
                };
                out.push(json!({ "path": path, "relative": self.rel(path), "start": pos(d.line, d.col), "end": pos(d.end_line, d.end_col), "severity": severity, "message": d.message }));
            }
        }
        Value::Array(out)
    }

    fn ipc_changes(&self) -> Value {
        let now = self.clock;
        Value::Array(
            self.outside
                .iter()
                .map(|h| json!({ "path": h.path, "relative": self.rel(&h.path), "line": h.line + 1, "added": h.added, "removed": h.removed, "preview": h.preview, "seconds_ago": (now - h.at).max(0.0).round() }))
                .collect(),
        )
    }

    /// Opens a file, brings the window forward, and with `wait` holds the reply until the
    /// buffer closes (`Ok(None)`).
    fn ipc_open(&mut self, a: &Value, id: u64, reply: &Sender<Response>) -> Result<Option<Value>, String> {
        let path = self.ipc_path(a)?;
        let at = a["line"]
            .as_u64()
            .map(|l| Pos::new((l as usize).saturating_sub(1), a["column"].as_u64().map_or(0, |c| (c as usize).saturating_sub(1))));
        self.overlay = Overlay::None;
        self.open_path(path.clone(), at);
        self.raise(a);
        if a["wait"].as_bool() == Some(true) {
            let seen = self.doc_index(&path).is_some();
            self.waiters.push(Waiter { id, path, reply: reply.clone(), seen });
            return Ok(None);
        }
        Ok(Some(json!({ "path": path })))
    }

    /// Brings the window forward, with the caller's activation token on Wayland.
    fn raise(&mut self, a: &Value) {
        let token = a["activation_token"].as_str().filter(|t| !t.is_empty()).map(str::to_string);
        self.raise = Some(token);
    }

    fn ipc_reveal(&mut self, a: &Value) -> Res {
        if a["clear"].as_bool() == Some(true) {
            self.note = None;
            return Ok(json!({}));
        }
        let path = self.ipc_path(a)?;
        let first = a["line"].as_u64().ok_or("reveal needs a line")? as usize;
        let last = a["end_line"].as_u64().map_or(first, |n| n as usize).max(first);
        let text = a["note"].as_str().unwrap_or("").to_string();
        self.open_path(path.clone(), Some(Pos::new(first - 1, 0)));
        self.note = Some(Note { path, first: first - 1, last: last - 1, text: text.clone() });
        if !text.is_empty() {
            self.ed.message(format!("agent: {text}"));
        }
        self.raise(a);
        Ok(json!({}))
    }

    fn ipc_save(&mut self, a: &Value) -> Res {
        if a["all"].as_bool() == Some(true) {
            let n = self.save_all(false);
            return Ok(json!({ "saving": n }));
        }
        let path = self.ipc_path(a)?;
        let i = self.doc_index(&path).ok_or("that file is not open")?;
        let prev = self.ed.active;
        self.ed.active = i;
        self.save_active(None, crate::services::AfterSave::Nothing);
        self.ed.active = prev;
        Ok(json!({ "saving": 1 }))
    }

    fn ipc_edit(&mut self, a: &Value) -> Res {
        if !self.cfg.ipc.allow_edits {
            return Err("edits through the socket are off (ipc.allow_edits). Use propose".into());
        }
        let path = self.ipc_path(a)?;
        let label = a["label"].as_str().unwrap_or("agent edit").to_string();
        let edits: Vec<crate::services::lsp::TextEdit> = match (a["text"].as_str(), a["edits"].as_array()) {
            (Some(text), _) => {
                // A file that isn't open (or doesn't exist yet) opens in the background with the
                // text in it, unsaved, like a rename's other files.
                let Some(i) = self.doc_index(&path) else {
                    let all =
                        crate::services::lsp::TextEdit { start: Pos::new(0, 0), end: Pos::new(usize::MAX / 2, 0), text: text.to_string() };
                    if let Some(svc) = &self.svc {
                        svc.open_edited(path.clone(), vec![all]);
                    }
                    self.ed.message(format!("agent: {label} in \"{}\" (unsaved)", self.rel(&path)));
                    return Ok(json!({ "opened": true, "saved": false }));
                };
                // An open buffer: apply the text as the smallest change.
                let old = self.ed.docs[i].buf.text();
                let p = Proposal::new(path.clone(), label.clone(), &old, text.to_string(), self.ed.docs[i].buf.version());
                let doc = &mut self.ed.docs[i];
                let at = doc.view.cursor;
                doc.buf.begin(at);
                for (a, b, t) in p.edits(&old) {
                    doc.buf.replace(a, b, &t);
                }
                doc.buf.commit(at);
                doc.view.cursor = doc.buf.clamp(at, false);
                self.edited();
                self.ed.message(format!("agent: {label} (+{} -{})", p.added, p.removed));
                return Ok(json!({ "added": p.added, "removed": p.removed }));
            }
            (None, Some(list)) => list
                .iter()
                .map(|e| {
                    Ok(crate::services::lsp::TextEdit {
                        start: read_pos(&e["start"])?,
                        end: read_pos(&e["end"])?,
                        text: e["text"].as_str().unwrap_or("").to_string(),
                    })
                })
                .collect::<Result<_, String>>()?,
            _ => return Err("edit needs `text` (the whole new text) or `edits`".into()),
        };
        let n = edits.len();
        match self.doc_index(&path) {
            Some(i) => {
                super::apply_edits(&mut self.ed.docs[i].buf, &edits);
                let doc = &mut self.ed.docs[i];
                doc.view.cursor = doc.buf.clamp(doc.view.cursor, false);
                self.edited();
            }
            None => {
                if let Some(svc) = &self.svc {
                    svc.open_edited(path, edits);
                }
            }
        }
        self.ed.message(format!("agent: {label} ({n} edits)"));
        Ok(json!({ "edits": n }))
    }

    fn ipc_propose(&mut self, a: &Value) -> Res {
        let path = self.ipc_path(a)?;
        let text = a["text"].as_str().ok_or("propose needs `text`: the whole new text of the file")?.to_string();
        let label = a["label"].as_str().unwrap_or("a change").to_string();
        let (old, version) = match self.doc_index(&path) {
            Some(i) => (self.ed.docs[i].buf.text(), self.ed.docs[i].buf.version()),
            // Not open yet: the diff is made once the file opens, off this request.
            None => (String::new(), u64::MAX),
        };
        let p = Proposal::new(path, label, &old, text, version);
        let (added, removed) = (p.added, p.removed);
        self.propose(p);
        self.raise(a);
        Ok(json!({ "added": added, "removed": removed, "pending": self.proposals.len() }))
    }

    fn ipc_command(&mut self, a: &Value, tx: &mut Tx) -> Res {
        if !self.cfg.ipc.allow_edits {
            return Err("commands through the socket are off (ipc.allow_edits)".into());
        }
        let line = a["line"].as_str().ok_or("command needs `line`, such as \":w\"")?;
        let before = self.ed.msg.clone();
        let actions = editor::vim::run_ex(&mut self.ed, line.trim_start_matches(':'));
        self.run_actions(actions, tx);
        let msg = self.ed.msg.clone().filter(|m| Some(m) != before.as_ref());
        match msg {
            Some(m) if m.error => Err(m.text),
            Some(m) => Ok(json!({ "message": m.text })),
            None => Ok(json!({})),
        }
    }

    /// Answers `open --wait` requests whose buffer has closed. Runs every poll.
    pub fn check_waiters(&mut self) {
        let open: Vec<PathBuf> = self.ed.docs.iter().filter_map(|d| d.buf.path.clone()).collect();
        self.waiters.retain_mut(|w| {
            let is_open = open.contains(&w.path);
            if is_open {
                w.seen = true;
                return true;
            }
            if w.seen {
                let _ = w.reply.send(Response::ok(w.id, json!({ "closed": true })));
                return false;
            }
            true
        });
    }

    /// Answers every held request; the editor is quitting.
    pub fn release_waiters(&mut self) {
        for w in self.waiters.drain(..) {
            let _ = w.reply.send(Response::ok(w.id, json!({ "closed": true })));
        }
    }
}
