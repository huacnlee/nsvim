//! Talking to a running editor from outside it: a Unix socket per project, one JSON request
//! and one JSON reply per line. `editor ctl` and `editor mcp` are its clients, and AI agents,
//! scripts and shells use those. See `docs/agents.md`.
//!
//! Windows has no Unix sockets in the standard library. There the editor serves no socket, and
//! the clients report that no editor runs.

pub mod client;
pub mod mcp;

use std::hash::{Hash, Hasher};
#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    #[serde(default)]
    pub id: u64,
    pub cmd: String,
    #[serde(default)]
    pub args: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    #[serde(default)]
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub result: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(id: u64, result: Value) -> Response {
        Response { id, ok: true, result, error: None }
    }

    pub fn err(id: u64, error: impl Into<String>) -> Response {
        Response { id, ok: false, result: Value::Null, error: Some(error.into()) }
    }
}

/// `$XDG_RUNTIME_DIR/editor`, or a folder per user under the temp folder.
pub fn runtime_dir() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(d) if !d.is_empty() => PathBuf::from(d).join("editor"),
        _ => std::env::temp_dir().join(format!("editor-{}", user_id())),
    }
}

#[cfg(unix)]
fn user_id() -> String {
    unsafe extern "C" {
        #[link_name = "getuid"]
        fn libc_getuid() -> u32;
    }
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc_getuid() }.to_string()
}

#[cfg(not(unix))]
fn user_id() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "user".into())
}

/// The socket of the editor whose project is `root`.
pub fn socket_for(root: &Path) -> PathBuf {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    root.hash(&mut h);
    runtime_dir().join(format!("{:016x}.sock", h.finish()))
}

/// Whether an editor answers on `path`.
#[cfg(unix)]
pub fn alive(path: &Path) -> bool {
    UnixStream::connect(path).is_ok()
}

/// Whether an editor answers on `path`: never, on a system without Unix sockets.
#[cfg(not(unix))]
pub fn alive(_path: &Path) -> bool {
    false
}

/// The socket of the editor for `dir`: the nearest folder at or above it with a running editor,
/// else the only running editor.
pub fn find(dir: &Path) -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("EDITOR_SOCKET").map(PathBuf::from).filter(|p| alive(p)) {
        return Some(p);
    }
    for a in dir.ancestors() {
        let s = socket_for(a);
        if s.exists() && alive(&s) {
            return Some(s);
        }
    }
    let live = running();
    if live.len() == 1 { live.into_iter().next() } else { None }
}

/// Every socket with a running editor behind it.
pub fn running() -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(runtime_dir()) else { return Vec::new() };
    rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "sock") && alive(p)).collect()
}

/// Sends one request and waits for its reply. `timeout` bounds the wait; `None` waits for as
/// long as the editor takes (`open --wait` returns when the buffer closes).
#[cfg(unix)]
pub fn call(socket: &Path, cmd: &str, args: Value, timeout: Option<Duration>) -> std::io::Result<Response> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_read_timeout(timeout)?;
    let req = Request { id: 1, cmd: cmd.to_string(), args };
    let mut line = serde_json::to_string(&req).map_err(std::io::Error::other)?;
    line.push('\n');
    stream.write_all(line.as_bytes())?;
    let mut reader = BufReader::new(stream);
    let mut reply = String::new();
    reader.read_line(&mut reply)?;
    if reply.is_empty() {
        return Err(std::io::Error::other("the editor closed the connection"));
    }
    serde_json::from_str(&reply).map_err(std::io::Error::other)
}

/// Sends one request: on a system without Unix sockets, no editor can answer.
#[cfg(not(unix))]
pub fn call(_socket: &Path, _cmd: &str, _args: Value, _timeout: Option<Duration>) -> std::io::Result<Response> {
    Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "the control socket needs Unix sockets"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sockets_are_per_project_and_stable() {
        let a = socket_for(Path::new("/p/one"));
        assert_eq!(a, socket_for(Path::new("/p/one")));
        assert_ne!(a, socket_for(Path::new("/p/two")));
        assert!(a.extension().is_some_and(|x| x == "sock"));
    }

    #[test]
    fn replies_serialize_without_empty_fields() {
        let ok = serde_json::to_string(&Response::ok(3, serde_json::json!({"a": 1}))).unwrap();
        assert_eq!(ok, r#"{"id":3,"ok":true,"result":{"a":1}}"#);
        let err = serde_json::to_string(&Response::err(4, "no")).unwrap();
        assert_eq!(err, r#"{"id":4,"ok":false,"error":"no"}"#);
    }
}
