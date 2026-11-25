//! Built-in LSP: diagnostics, go to definition, references, hover and completion, from
//! whatever language servers are installed. One thread owns every server; it never blocks the
//! UI, and a missing server only means that language has no LSP.

mod edits;
#[cfg(not(target_arch = "wasm32"))]
mod server;

#[cfg(not(target_arch = "wasm32"))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, channel};

use editor::buffer::Pos;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::json;

#[cfg(not(target_arch = "wasm32"))]
use super::{Event, Out};
pub use edits::{FileEdit, TextEdit, summary as edits_summary};
#[cfg(not(target_arch = "wasm32"))]
use server::{FromServer, Server};

#[cfg(not(target_arch = "wasm32"))]
/// Files bigger than this don't go to a language server: sending them on each change would
/// cost more than it gives.
const MAX_BYTES: usize = 4 * 1024 * 1024;

pub enum Job {
    Open {
        path: PathBuf,
        lang: &'static str,
        text: String,
    },
    Change {
        path: PathBuf,
        text: String,
    },
    Saved {
        path: PathBuf,
    },
    Close {
        path: PathBuf,
    },
    Definition {
        path: PathBuf,
        pos: Pos,
    },
    References {
        path: PathBuf,
        pos: Pos,
    },
    Hover {
        path: PathBuf,
        pos: Pos,
    },
    Completion {
        path: PathBuf,
        pos: Pos,
    },
    Rename {
        path: PathBuf,
        pos: Pos,
        name: String,
    },
    /// The code actions for `start..end` (the cursor, or a selection).
    CodeActions {
        path: PathBuf,
        start: Pos,
        end: Pos,
    },
    /// Runs one action from the last `CodeActions` answer, by index.
    RunAction {
        index: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub line: usize,
    pub col: usize,
    pub end_line: usize,
    pub end_col: usize,
    /// 1 error, 2 warning, 3 information, 4 hint.
    pub severity: u8,
    pub message: String,
}

/// A place in a file, with its line's text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loc {
    pub path: PathBuf,
    pub pos: Pos,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub label: String,
    pub detail: String,
    pub kind: &'static str,
    /// The text to insert, replacing the word before the cursor.
    pub insert: String,
}

#[cfg(not(target_arch = "wasm32"))]
/// The language server for an editor language: its key, its LSP language id and its command.
fn server_for(lang: &str) -> Option<(&'static str, &'static str, &'static [&'static str])> {
    Some(match lang {
        "rust" => ("rust", "rust", &["rust-analyzer"]),
        "go" => ("go", "go", &["gopls"]),
        "python" => ("python", "python", &["pyright-langserver", "--stdio"]),
        "typescript" => ("ts", "typescript", &["typescript-language-server", "--stdio"]),
        "tsx" => ("ts", "typescriptreact", &["typescript-language-server", "--stdio"]),
        "javascript" => ("ts", "javascript", &["typescript-language-server", "--stdio"]),
        "jsx" => ("ts", "javascriptreact", &["typescript-language-server", "--stdio"]),
        "c" => ("c", "c", &["clangd"]),
        "cpp" => ("c", "cpp", &["clangd"]),
        "lua" => ("lua", "lua", &["lua-language-server"]),
        "zig" => ("zig", "zig", &["zls"]),
        _ => return None,
    })
}

pub fn uri(p: &Path) -> String {
    let s = p.to_string_lossy();
    let mut out = String::from("file://");
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn path_of(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(v) = u8::from_str_radix(&rest[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out).into_owned()))
}

#[cfg(not(target_arch = "wasm32"))]
struct Doc {
    server: usize,
    version: i64,
    text: String,
}

#[cfg(not(target_arch = "wasm32"))]
/// What an answer to a request is for.
enum Pending {
    Definition,
    References,
    Hover,
    Completion(Pos),
    Rename(String),
    CodeActions,
    Resolve,
}

#[cfg(not(target_arch = "wasm32"))]
enum Msg {
    Job(Job),
    Server(FromServer),
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start(root: PathBuf, out: Out) -> Sender<Job> {
    let (job_tx, job_rx) = channel::<Job>();
    let (tx, rx) = channel::<Msg>();
    let relay = tx.clone();
    std::thread::Builder::new()
        .name("editor-lsp-jobs".into())
        .spawn(move || {
            for j in job_rx {
                if relay.send(Msg::Job(j)).is_err() {
                    break;
                }
            }
        })
        .expect("start the LSP job relay");
    let (srv_tx, srv_rx) = channel::<FromServer>();
    std::thread::Builder::new()
        .name("editor-lsp-relay".into())
        .spawn(move || {
            for m in srv_rx {
                if tx.send(Msg::Server(m)).is_err() {
                    break;
                }
            }
        })
        .expect("start the LSP relay");
    std::thread::Builder::new()
        .name("editor-lsp".into())
        .spawn(move || {
            let mut m = Manager {
                root,
                out,
                servers: Vec::new(),
                keys: HashMap::new(),
                docs: HashMap::new(),
                pending: HashMap::new(),
                srv_tx,
                raw_diags: HashMap::new(),
                actions: Vec::new(),
            };
            for msg in rx {
                match msg {
                    Msg::Job(j) => m.job(j),
                    Msg::Server(FromServer::Message(i, v)) => m.message(i, v),
                    Msg::Server(FromServer::Exited(i)) => {
                        if let Some(Some(s)) = m.servers.get(i) {
                            let why = if s.ready { "stopped" } else { "exited before it started: check that it runs from a terminal" };
                            m.out.send(Event::LspInfo(format!("{} language server {why}", s.name)));
                        }
                        if let Some(slot) = m.servers.get_mut(i) {
                            *slot = None;
                        }
                    }
                }
            }
        })
        .expect("start the LSP manager");
    job_tx
}

#[cfg(not(target_arch = "wasm32"))]
struct Manager {
    root: PathBuf,
    out: Out,
    servers: Vec<Option<Server>>,
    /// Server index by key; `None` once a start failed, so it isn't tried again.
    keys: HashMap<&'static str, Option<usize>>,
    docs: HashMap<PathBuf, Doc>,
    pending: HashMap<(usize, i64), (Pending, PathBuf)>,
    srv_tx: Sender<FromServer>,
    /// Each file's diagnostics as the server sent them, for code action requests.
    raw_diags: HashMap<PathBuf, Value>,
    /// The last code actions offered, with the server that offered each.
    actions: Vec<(usize, Value)>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Manager {
    fn server(&mut self, key: &'static str, name: &'static str, cmd: &[&str]) -> Option<usize> {
        if let Some(slot) = self.keys.get(key) {
            return *slot;
        }
        let index = self.servers.len();
        let started = Server::start(index, name, cmd, &self.root, self.srv_tx.clone());
        let slot = started.map(|s| {
            self.servers.push(Some(s));
            index
        });
        if slot.is_some() {
            self.out.send(Event::LspInfo(format!("{} started for {name}", cmd[0])));
        }
        self.keys.insert(key, slot);
        slot
    }

    fn job(&mut self, j: Job) {
        match j {
            Job::Open { path, lang, text } => {
                if text.len() > MAX_BYTES || self.docs.contains_key(&path) {
                    return;
                }
                let Some((key, id, cmd)) = server_for(lang) else { return };
                let Some(si) = self.server(key, lang, cmd) else { return };
                let Some(Some(s)) = self.servers.get_mut(si) else { return };
                s.notify(
                    "textDocument/didOpen",
                    json!({ "textDocument": { "uri": uri(&path), "languageId": id, "version": 1, "text": text } }),
                );
                self.docs.insert(path, Doc { server: si, version: 1, text });
            }
            Job::Change { path, text } => {
                let Some(d) = self.docs.get_mut(&path) else { return };
                if text.len() > MAX_BYTES || d.text == text {
                    return;
                }
                d.version += 1;
                d.text = text;
                let (si, v) = (d.server, d.version);
                if let Some(Some(s)) = self.servers.get_mut(si) {
                    let t = self.docs[&path].text.clone();
                    s.notify(
                        "textDocument/didChange",
                        json!({ "textDocument": { "uri": uri(&path), "version": v }, "contentChanges": [{ "text": t }] }),
                    );
                }
            }
            Job::Saved { path } => {
                if let Some(Some(s)) = self.docs.get(&path).and_then(|d| self.servers.get_mut(d.server)) {
                    s.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&path) } }));
                }
            }
            Job::Close { path } => {
                if let Some(d) = self.docs.remove(&path)
                    && let Some(Some(s)) = self.servers.get_mut(d.server)
                {
                    s.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&path) } }));
                }
            }
            Job::Definition { path, pos } => self.ask(path, pos, "textDocument/definition", Pending::Definition, json!({})),
            Job::References { path, pos } => {
                self.ask(path, pos, "textDocument/references", Pending::References, json!({ "context": { "includeDeclaration": true } }))
            }
            Job::Hover { path, pos } => self.ask(path, pos, "textDocument/hover", Pending::Hover, json!({})),
            Job::Completion { path, pos } => self.ask(path, pos, "textDocument/completion", Pending::Completion(pos), json!({})),
            Job::Rename { path, pos, name } => {
                self.ask(path, pos, "textDocument/rename", Pending::Rename(name.clone()), json!({ "newName": name }))
            }
            Job::CodeActions { path, start, end } => self.code_actions(path, start, end),
            Job::RunAction { index } => self.run_action(index),
        }
    }

    fn code_actions(&mut self, path: PathBuf, start: Pos, end: Pos) {
        let Some(d) = self.docs.get(&path) else {
            self.out.send(Event::LspInfo("no language server is running for this file".into()));
            return;
        };
        let si = d.server;
        let Some(Some(s)) = self.servers.get_mut(si) else { return };
        let (sc, ec) = (to_wire(&d.text, start, s.utf32), to_wire(&d.text, end, s.utf32));
        let diags: Vec<Value> = self
            .raw_diags
            .get(&path)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|dg| {
                let a = dg.pointer("/range/start/line").and_then(Value::as_u64).unwrap_or(0) as usize;
                let b = dg.pointer("/range/end/line").and_then(Value::as_u64).unwrap_or(0) as usize;
                a <= end.line && b >= start.line
            })
            .cloned()
            .collect();
        let params = json!({
            "textDocument": { "uri": uri(&path) },
            "range": { "start": { "line": start.line, "character": sc }, "end": { "line": end.line, "character": ec } },
            "context": { "diagnostics": diags, "triggerKind": 1 }
        });
        let id = s.request("textDocument/codeAction", params);
        self.pending.insert((si, id), (Pending::CodeActions, path));
    }

    fn run_action(&mut self, index: usize) {
        let Some((si, action)) = self.actions.get(index).cloned() else { return };
        // A bare Command.
        if action["command"].is_string() {
            self.execute(si, &action);
            return;
        }
        if action.get("edit").is_some() || action.get("command").is_some() {
            self.finish_action(si, &action);
            return;
        }
        // A code action without its edit yet: ask the server to fill it in.
        if let Some(Some(s)) = self.servers.get_mut(si) {
            let id = s.request("codeAction/resolve", action);
            self.pending.insert((si, id), (Pending::Resolve, PathBuf::new()));
        }
    }

    fn finish_action(&mut self, si: usize, action: &Value) {
        let title = action["title"].as_str().unwrap_or("code action").to_string();
        if let Some(edit) = action.get("edit") {
            self.apply_edit(si, edit, title);
        }
        if let Some(cmd) = action.get("command").filter(|c| c.is_object()) {
            self.execute(si, cmd);
        }
    }

    fn execute(&mut self, si: usize, cmd: &Value) {
        if let Some(Some(s)) = self.servers.get_mut(si) {
            s.request(
                "workspace/executeCommand",
                json!({ "command": cmd["command"], "arguments": cmd.get("arguments").cloned().unwrap_or(json!([])) }),
            );
        }
    }

    /// A file's text now: the open document's, else the file on disk.
    fn text_of(&self, path: &Path) -> Option<String> {
        match self.docs.get(path) {
            Some(d) => Some(d.text.clone()),
            None => std::fs::read_to_string(path).ok(),
        }
    }

    /// Carries out a workspace edit: file operations here, text edits in the app.
    fn apply_edit(&mut self, si: usize, edit: &Value, label: String) {
        let utf32 = self.servers.get(si).and_then(|s| s.as_ref()).is_some_and(|s| s.utf32);
        let (files, ops) = edits::convert(edit, utf32, &|p| self.text_of(p));
        for op in ops {
            let r = match &op {
                edits::FileOp::Create(p) => {
                    p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| if p.exists() { Ok(()) } else { std::fs::write(p, "") })
                }
                edits::FileOp::Rename(a, b) => std::fs::rename(a, b),
                edits::FileOp::Delete(p) => std::fs::remove_file(p),
            };
            if let Err(e) = r {
                self.out.send(Event::LspInfo(format!("{label}: {op:?} failed: {e}")));
            }
        }
        if !files.is_empty() {
            self.out.send(Event::ApplyEdit { label, files });
        }
    }

    fn ask(&mut self, path: PathBuf, pos: Pos, method: &str, what: Pending, extra: Value) {
        let Some(d) = self.docs.get(&path) else {
            // Completion asks on its own while you type; only a request you made says why
            // nothing came back.
            if !matches!(what, Pending::Completion(_)) {
                self.out.send(Event::LspInfo("no language server is running for this file".into()));
            }
            return;
        };
        let si = d.server;
        let Some(Some(s)) = self.servers.get_mut(si) else { return };
        let character = to_wire(&d.text, pos, s.utf32);
        let mut params = json!({ "textDocument": { "uri": uri(&path) }, "position": { "line": pos.line, "character": character } });
        if let (Some(p), Some(e)) = (params.as_object_mut(), extra.as_object()) {
            p.extend(e.clone());
        }
        let id = s.request(method, params);
        self.pending.insert((si, id), (what, path));
    }

    fn message(&mut self, si: usize, v: Value) {
        // A request from the server: answer so it doesn't wait (configuration, progress).
        if let (Some(id), Some(method)) = (v.get("id"), v.get("method").and_then(Value::as_str)) {
            let result = if method == "workspace/configuration" {
                let n = v.pointer("/params/items").and_then(Value::as_array).map_or(1, |a| a.len());
                Value::Array(vec![Value::Null; n])
            } else if method == "workspace/applyEdit" {
                let label = v.pointer("/params/label").and_then(Value::as_str).unwrap_or("edit").to_string();
                let edit = v["params"]["edit"].clone();
                self.apply_edit(si, &edit, label);
                json!({ "applied": true })
            } else {
                Value::Null
            };
            if let Some(Some(s)) = self.servers.get_mut(si) {
                s.send_raw(json!({ "jsonrpc": "2.0", "id": id, "result": result }));
            }
            return;
        }
        if v.get("method").and_then(Value::as_str) == Some("textDocument/publishDiagnostics") {
            self.diagnostics(si, &v["params"]);
            return;
        }
        let Some(id) = v.get("id").and_then(Value::as_i64) else { return };
        if id == 0 {
            if let Some(Some(s)) = self.servers.get_mut(si) {
                s.initialized(&v["result"]);
            }
            return;
        }
        let Some((what, path)) = self.pending.remove(&(si, id)) else { return };
        let result = &v["result"];
        let utf32 = self.servers.get(si).and_then(|s| s.as_ref()).is_some_and(|s| s.utf32);
        match what {
            Pending::Definition | Pending::References => {
                let kind = if matches!(what, Pending::Definition) { "definition" } else { "references" };
                let locs = self.locations(result, utf32);
                self.out.send(Event::Locations { kind, locs });
            }
            Pending::Hover => {
                let text = hover_text(&result["contents"]);
                self.out.send(Event::Hover { path, text });
            }
            Pending::Completion(pos) => {
                let items = completion_items(result);
                self.out.send(Event::Completion { path, at: pos, items });
            }
            Pending::Rename(name) => {
                if let Some(e) = v.get("error") {
                    self.out.send(Event::LspInfo(format!("rename: {}", e["message"].as_str().unwrap_or("the server refused"))));
                } else if result.is_null() {
                    self.out.send(Event::LspInfo("nothing to rename here".into()));
                } else {
                    let r = result.clone();
                    self.apply_edit(si, &r, format!("rename to {name}"));
                }
            }
            Pending::CodeActions => {
                self.actions =
                    result.as_array().into_iter().flatten().filter(|a| a.get("disabled").is_none()).map(|a| (si, a.clone())).collect();
                let titles = self.actions.iter().map(|(_, a)| a["title"].as_str().unwrap_or("?").to_string()).collect();
                self.out.send(Event::CodeActions { titles });
            }
            Pending::Resolve => {
                let r = result.clone();
                self.finish_action(si, &r);
            }
        }
    }

    fn diagnostics(&mut self, si: usize, params: &Value) {
        let Some(path) = params["uri"].as_str().and_then(path_of) else { return };
        let utf32 = self.servers.get(si).and_then(|s| s.as_ref()).is_some_and(|s| s.utf32);
        let text = self.docs.get(&path).map(|d| d.text.clone()).unwrap_or_default();
        let lines: Vec<&str> = text.split('\n').collect();
        let mut diags = Vec::new();
        for d in params["diagnostics"].as_array().into_iter().flatten() {
            let (sl, sc) = (
                d.pointer("/range/start/line").and_then(Value::as_u64).unwrap_or(0) as usize,
                d.pointer("/range/start/character").and_then(Value::as_u64).unwrap_or(0) as usize,
            );
            let (el, ec) = (
                d.pointer("/range/end/line").and_then(Value::as_u64).unwrap_or(0) as usize,
                d.pointer("/range/end/character").and_then(Value::as_u64).unwrap_or(0) as usize,
            );
            let col = |l: usize, c: usize| if utf32 { c } else { from_utf16(lines.get(l).copied().unwrap_or(""), c) };
            diags.push(Diag {
                line: sl,
                col: col(sl, sc),
                end_line: el,
                end_col: col(el, ec),
                severity: d["severity"].as_u64().unwrap_or(1) as u8,
                message: d["message"].as_str().unwrap_or("").lines().next().unwrap_or("").to_string(),
            });
        }
        diags.sort_by_key(|d| (d.line, d.severity));
        self.raw_diags.insert(path.clone(), params["diagnostics"].clone());
        self.out.send(Event::Diagnostics { path, diags });
    }

    fn locations(&self, result: &Value, utf32: bool) -> Vec<Loc> {
        let list: Vec<&Value> = match result {
            Value::Array(a) => a.iter().collect(),
            Value::Object(_) => vec![result],
            _ => Vec::new(),
        };
        let mut out = Vec::new();
        for l in list {
            let uri = l.get("uri").or_else(|| l.get("targetUri")).and_then(Value::as_str);
            let range = l.get("range").or_else(|| l.get("targetSelectionRange"));
            let (Some(path), Some(range)) = (uri.and_then(path_of), range) else { continue };
            let line = range.pointer("/start/line").and_then(Value::as_u64).unwrap_or(0) as usize;
            let ch = range.pointer("/start/character").and_then(Value::as_u64).unwrap_or(0) as usize;
            let text = match self.docs.get(&path) {
                Some(d) => d.text.split('\n').nth(line).unwrap_or("").to_string(),
                None => std::fs::read_to_string(&path).ok().and_then(|t| t.split('\n').nth(line).map(str::to_string)).unwrap_or_default(),
            };
            let col = if utf32 { ch } else { from_utf16(&text, ch) };
            out.push(Loc { path, pos: Pos::new(line, col), text });
            if out.len() >= 2000 {
                break;
            }
        }
        out
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A char column as the server counts it: chars (utf-32) or UTF-16 units.
fn to_wire(text: &str, pos: Pos, utf32: bool) -> usize {
    if utf32 {
        return pos.col;
    }
    text.split('\n').nth(pos.line).map_or(pos.col, |l| l.chars().take(pos.col).map(char::len_utf16).sum())
}

#[cfg(not(target_arch = "wasm32"))]
fn from_utf16(line: &str, units: usize) -> usize {
    let mut n = 0;
    for (i, c) in line.chars().enumerate() {
        if n >= units {
            return i;
        }
        n += c.len_utf16();
    }
    line.chars().count()
}

#[cfg(not(target_arch = "wasm32"))]
fn hover_text(contents: &Value) -> String {
    match contents {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(hover_text).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n"),
        Value::Object(o) => o.get("value").and_then(Value::as_str).unwrap_or("").to_string(),
        _ => String::new(),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn completion_items(result: &Value) -> Vec<Item> {
    let list = match result {
        Value::Array(a) => a.as_slice(),
        Value::Object(o) => o.get("items").and_then(Value::as_array).map_or(&[][..], |v| v.as_slice()),
        _ => &[],
    };
    let mut items: Vec<(String, Item)> = list
        .iter()
        .filter_map(|it| {
            let label = it["label"].as_str()?.trim().to_string();
            let insert =
                it.pointer("/textEdit/newText").and_then(Value::as_str).or_else(|| it["insertText"].as_str()).unwrap_or(&label).to_string();
            let sort = it["sortText"].as_str().unwrap_or(&label).to_string();
            Some((
                sort,
                Item {
                    detail: it["detail"].as_str().unwrap_or("").chars().take(60).collect(),
                    kind: kind_name(it["kind"].as_u64().unwrap_or(0)),
                    insert,
                    label,
                },
            ))
        })
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    items.into_iter().map(|(_, i)| i).take(60).collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn kind_name(k: u64) -> &'static str {
    match k {
        2 => "Method",
        3 => "Function",
        4 => "Constructor",
        5 => "Field",
        6 => "Variable",
        7 => "Class",
        8 => "Interface",
        9 => "Module",
        10 => "Property",
        13 => "Enum",
        14 => "Keyword",
        15 => "Snippet",
        20 => "EnumMember",
        21 => "Constant",
        22 => "Struct",
        25 => "TypeParam",
        _ => "Text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_round_trip_with_escapes() {
        let p = PathBuf::from("/home/me/my project/é.rs");
        assert_eq!(path_of(&uri(&p)), Some(p));
    }

    #[test]
    fn utf16_columns_convert_both_ways() {
        let text = "a😀b";
        assert_eq!(to_wire(text, Pos::new(0, 2), false), 3);
        assert_eq!(from_utf16(text, 3), 2);
    }

    #[test]
    fn hover_flattens_markup() {
        assert_eq!(hover_text(&json!({"kind": "markdown", "value": "fn x()"})), "fn x()");
        assert_eq!(hover_text(&json!(["a", {"language": "rust", "value": "b"}])), "a\n\nb");
    }
}

/// Runs against a real `rust-analyzer`: `cargo test lsp_live -- --ignored --nocapture`.
#[cfg(test)]
mod live {
    use super::*;
    use std::sync::Arc;
    use std::sync::mpsc::Receiver;
    use std::time::{Duration, Instant};

    fn next(rx: &Receiver<Event>, secs: u64, want: impl Fn(&Event) -> bool) -> Option<Event> {
        let end = Instant::now() + Duration::from_secs(secs);
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(e) if want(&e) => return Some(e),
                Ok(e) => eprintln!(
                    "event: {}",
                    match &e {
                        Event::Diagnostics { diags, .. } => format!("diagnostics {}", diags.len()),
                        Event::LspInfo(m) => format!("info {m}"),
                        _ => "other".into(),
                    }
                ),
                Err(_) => return None,
            }
        }
        None
    }

    #[test]
    #[ignore]
    fn lsp_live_clangd_answers() {
        let dir = tempfile::tempdir().unwrap();
        let src = "struct S { int a; };\nint helper(void) { return 41; }\n\nint main(void) {\n    struct S s;\n    s.missing = 1;\n    return helper();\n}\n";
        let path = dir.path().join("main.c");
        std::fs::write(&path, src).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let jobs = start(dir.path().to_path_buf(), Out { tx, waker: Arc::new(|| {}) });
        jobs.send(Job::Open { path: path.clone(), lang: "c", text: src.into() }).unwrap();
        match next(&rx, 60, |e| matches!(e, Event::Diagnostics { diags, .. } if !diags.is_empty())) {
            Some(Event::Diagnostics { diags, .. }) => {
                eprintln!("diagnostics: {diags:?}");
                assert!(diags.iter().any(|d| d.line == 5 && d.severity == 1 && d.message.contains("missing")));
            }
            _ => panic!("no diagnostics within 60 s"),
        }
        jobs.send(Job::Definition { path: path.clone(), pos: Pos::new(6, 12) }).unwrap();
        match next(&rx, 30, |e| matches!(e, Event::Locations { .. })) {
            Some(Event::Locations { locs, .. }) => {
                eprintln!("definition: {locs:?}");
                assert_eq!(locs[0].pos, Pos::new(1, 4));
            }
            _ => panic!("no definition"),
        }
        jobs.send(Job::Hover { path: path.clone(), pos: Pos::new(6, 12) }).unwrap();
        match next(&rx, 30, |e| matches!(e, Event::Hover { .. })) {
            Some(Event::Hover { text, .. }) => {
                eprintln!("hover: {text}");
                assert!(text.contains("helper"));
            }
            _ => panic!("no hover"),
        }
        jobs.send(Job::Completion { path: path.clone(), pos: Pos::new(6, 13) }).unwrap();
        match next(&rx, 30, |e| matches!(e, Event::Completion { .. })) {
            Some(Event::Completion { items, .. }) => {
                eprintln!("completion: {} items, first {:?}", items.len(), items.first());
                assert!(items.iter().any(|i| i.label.starts_with("helper")));
            }
            _ => panic!("no completion"),
        }
        jobs.send(Job::Rename { path: path.clone(), pos: Pos::new(6, 12), name: "assist".into() }).unwrap();
        match next(&rx, 30, |e| matches!(e, Event::ApplyEdit { .. })) {
            Some(Event::ApplyEdit { label, files }) => {
                eprintln!("rename: {label} {files:?}");
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].edits.len(), 2, "the definition and the call");
                assert!(files[0].edits.iter().all(|e| e.text == "assist"));
            }
            _ => panic!("no rename edit"),
        }
        jobs.send(Job::CodeActions { path: path.clone(), start: Pos::new(5, 6), end: Pos::new(5, 6) }).unwrap();
        match next(&rx, 30, |e| matches!(e, Event::CodeActions { .. })) {
            Some(Event::CodeActions { titles }) => eprintln!("code actions: {titles:?}"),
            _ => panic!("no code action answer"),
        }
    }
}
