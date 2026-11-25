//! One language server process: JSON-RPC over its stdin and stdout, with LSP's
//! `Content-Length` framing. A reader thread forwards every message it sends.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::Sender;

use serde_json::{Value, json};

/// What a server process sends, tagged with the server's index.
pub enum FromServer {
    Message(usize, Value),
    /// The process ended or its output closed.
    Exited(usize),
}

pub struct Server {
    child: Child,
    stdin: ChildStdin,
    next_id: i64,
    /// The language's name in the editor (`rust`), for status messages.
    pub name: &'static str,
    pub ready: bool,
    /// `utf-32` when the server agreed to count columns in chars, else `utf-16`.
    pub utf32: bool,
    /// Messages to send once `initialize` has answered.
    pub queue: Vec<Value>,
}

impl Server {
    /// Starts `cmd` in `root` and sends `initialize`. `None` when the binary isn't installed.
    pub fn start(index: usize, name: &'static str, cmd: &[&str], root: &Path, out: Sender<FromServer>) -> Option<Server> {
        let mut child = Command::new(cmd[0])
            .args(&cmd[1..])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdin = child.stdin.take()?;
        let stdout = child.stdout.take()?;
        std::thread::Builder::new()
            .name(format!("editor-lsp-{name}"))
            .spawn(move || {
                let mut r = BufReader::new(stdout);
                while let Some(msg) = read_message(&mut r) {
                    if out.send(FromServer::Message(index, msg)).is_err() {
                        return;
                    }
                }
                let _ = out.send(FromServer::Exited(index));
            })
            .ok()?;
        let mut s = Server { child, stdin, next_id: 1, name, ready: false, utf32: false, queue: Vec::new() };
        let uri = super::uri(root);
        let params = json!({
            "processId": std::process::id(),
            "rootUri": uri,
            "workspaceFolders": [{ "uri": uri, "name": root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() }],
            "capabilities": {
                "general": { "positionEncodings": ["utf-32", "utf-16"] },
                "textDocument": {
                    "synchronization": { "didSave": true },
                    "publishDiagnostics": { "relatedInformation": false },
                    "hover": { "contentFormat": ["plaintext", "markdown"] },
                    "completion": { "completionItem": { "snippetSupport": false } },
                    "definition": {},
                    "references": {},
                    "rename": {}
                },
                "workspace": { "workspaceFolders": true }
            },
            "clientInfo": { "name": "nsvim" }
        });
        s.send_raw(json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": params }));
        Some(s)
    }

    /// Sends a request and returns its id.
    pub fn request(&mut self, method: &str, params: Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        id
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }

    /// Sends now, or after `initialize` answers.
    fn send(&mut self, msg: Value) {
        if self.ready {
            self.send_raw(msg);
        } else {
            self.queue.push(msg);
        }
    }

    pub fn send_raw(&mut self, msg: Value) {
        let body = msg.to_string();
        let _ = write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len());
        let _ = self.stdin.flush();
    }

    /// The `initialize` answer arrived: say `initialized` and send what waited.
    pub fn initialized(&mut self, result: &Value) {
        self.utf32 = result.pointer("/capabilities/positionEncoding").and_then(Value::as_str) == Some("utf-32");
        self.ready = true;
        self.send_raw(json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} }));
        for m in std::mem::take(&mut self.queue) {
            self.send_raw(m);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Reads one framed message. `None` at the end of the stream.
fn read_message(r: &mut impl BufRead) -> Option<Value> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; len?];
    r.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok().or(Some(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_read_back_in_order() {
        let a = json!({"id": 1});
        let b = json!({"method": "x"});
        let mut raw = Vec::new();
        for m in [&a, &b] {
            let s = m.to_string();
            raw.extend(format!("Content-Length: {}\r\nContent-Type: x\r\n\r\n{s}", s.len()).into_bytes());
        }
        let mut r = std::io::Cursor::new(raw);
        assert_eq!(read_message(&mut r), Some(a));
        assert_eq!(read_message(&mut r), Some(b));
        assert_eq!(read_message(&mut r), None);
    }
}
