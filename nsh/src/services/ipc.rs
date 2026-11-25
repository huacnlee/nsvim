//! The control socket: a listener thread, a thread per connection, and each request handed to
//! the UI thread as an [`Event::Ipc`] with a channel for its reply. Also runs `:!` commands.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::channel;

use super::{Event, Out};
use crate::ipc::{Request, Response};

/// Binds the project's socket and serves it. `None` when another editor already serves this
/// project, or the socket cannot be made.
pub fn start(root: &Path, out: Out) -> Option<PathBuf> {
    let path = crate::ipc::socket_for(root);
    let dir = path.parent()?;
    std::fs::create_dir_all(dir).ok()?;
    let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    if path.exists() {
        if crate::ipc::alive(&path) {
            return None;
        }
        let _ = std::fs::remove_file(&path);
    }
    let listener = UnixListener::bind(&path).ok()?;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    std::thread::Builder::new()
        .name("editor-ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let out = out.clone();
                let _ = std::thread::Builder::new().name("editor-ipc-conn".into()).spawn(move || serve(stream, out));
            }
        })
        .ok()?;
    Some(path)
}

/// One connection: a request per line, each answered before the next is read.
fn serve(stream: UnixStream, out: Out) {
    let Ok(mut writer) = stream.try_clone() else { return };
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id;
                let (tx, rx) = channel();
                out.send(Event::Ipc { req, reply: tx });
                rx.recv().unwrap_or_else(|_| Response::err(id, "the editor is closing"))
            }
            Err(e) => Response::err(0, format!("bad request: {e}")),
        };
        let mut text = serde_json::to_string(&reply).unwrap_or_default();
        text.push('\n');
        if writer.write_all(text.as_bytes()).is_err() {
            return;
        }
    }
}

/// Runs `cmd` through `sh -c` in `cwd` with `input` on its stdin, off the UI thread.
pub fn run_shell(
    cmd: String,
    cwd: PathBuf,
    input: Option<String>,
    done: impl FnOnce(Result<(i32, String, String), String>) + Send + 'static,
) {
    std::thread::spawn(move || {
        let child = Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .current_dir(&cwd)
            .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => return done(Err(format!("{cmd}: {e}"))),
        };
        if let (Some(text), Some(mut stdin)) = (input, child.stdin.take()) {
            // Write from a thread, so a command that prints before reading all its input
            // cannot deadlock against a full pipe.
            std::thread::spawn(move || {
                let _ = stdin.write_all(text.as_bytes());
            });
        }
        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(mut s) = child.stdout.take() {
            let _ = s.read_to_string(&mut stdout);
        }
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_string(&mut stderr);
        }
        let code = child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1);
        done(Ok((code, stdout, stderr)));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_filter_sees_its_input() {
        let (tx, rx) = channel();
        run_shell("sort -r".into(), std::env::temp_dir(), Some("a\nc\nb\n".into()), move |r| {
            let _ = tx.send(r);
        });
        let (code, out, _) = rx.recv().unwrap().unwrap();
        assert_eq!(code, 0);
        assert_eq!(out, "c\nb\na\n");
    }
}

#[cfg(test)]
mod socket_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_request_round_trips_through_the_socket() {
        let dir = tempfile::tempdir().unwrap();
        // SAFETY: tests in this module run with this one variable for the runtime folder.
        unsafe { std::env::set_var("XDG_RUNTIME_DIR", dir.path()) };
        let (tx, rx) = channel();
        let out = Out { tx, waker: Arc::new(|| {}) };
        let root = dir.path().join("project");
        let path = start(&root, out).expect("the socket binds");
        // The app's side: answer every request with its command name.
        std::thread::spawn(move || {
            while let Ok(Event::Ipc { req, reply }) = rx.recv() {
                let _ = reply.send(Response::ok(req.id, serde_json::json!({ "cmd": req.cmd })));
            }
        });
        let r = crate::ipc::call(&path, "state", serde_json::json!({}), Some(std::time::Duration::from_secs(5))).unwrap();
        assert!(r.ok);
        assert_eq!(r.result["cmd"], "state");
        assert_eq!(crate::ipc::find(&root.join("src/deep")), Some(path.clone()), "found from a folder inside the project");
        assert!(start(&root, Out { tx: channel().0, waker: Arc::new(|| {}) }).is_none(), "a second editor does not steal it");
    }
}
