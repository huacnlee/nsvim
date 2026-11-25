//! The file worker: opens buffers, writes saves, config and the session, and reads files back
//! for reloads. One thread, so writes to one file land in the order they were asked for.

use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, channel};

#[cfg(not(target_arch = "wasm32"))]
use editor::buffer::Buffer;
use editor::buffer::{History, Pos};
use serde::{Deserialize, Serialize};

use super::{AfterSave, WriteKind};
#[cfg(not(target_arch = "wasm32"))]
use super::{Event, Out, editorconfig};

/// Unsaved text kept while editing, so a crash loses at most a couple of seconds of work.
#[derive(Serialize, Deserialize)]
struct Swap {
    path: PathBuf,
    text: String,
}

/// Where the undo history of `path` is kept between runs.
pub fn undo_file(path: &Path) -> PathBuf {
    crate::config::state_dir().join("undo").join(format!("{:016x}.json", key(path)))
}

/// Where the swap file of `path` goes.
pub fn swap_file(path: &Path) -> PathBuf {
    crate::config::state_dir().join("swap").join(format!("{:016x}.json", key(path)))
}

/// Writes the swap file of `path` now. The worker calls it, and the app calls it once on the
/// way out, for edits the user chose to leave unsaved.
pub fn write_swap(path: PathBuf, text: String) {
    if let Ok(json) = serde_json::to_vec(&Swap { path: path.clone(), text }) {
        let _ = write_atomic(&swap_file(&path), &json);
    }
}

fn key(path: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish()
}

#[cfg(not(target_arch = "wasm32"))]
/// Histories bigger than this aren't kept; they would cost more to load than they give.
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;

pub enum Job {
    /// Copy the old folders to the new names (`crate::migrate`).
    Migrate(crate::migrate::Migration),
    Open {
        path: PathBuf,
        at: Option<Pos>,
    },
    Save {
        path: PathBuf,
        bytes: Vec<u8>,
        version: u64,
        then: AfterSave,
    },
    ReadDisk {
        path: PathBuf,
    },
    Write {
        what: WriteKind,
        path: PathBuf,
        text: String,
    },
    /// The start of a file, for the finder's preview.
    Preview {
        path: PathBuf,
    },
    /// Keep a saved file's undo history for the next run.
    History {
        path: PathBuf,
        history: Box<History>,
    },
    /// Keep unsaved text in case the editor crashes.
    Swap {
        path: PathBuf,
        text: String,
    },
    /// Forget the swap file: the text is saved, or dropped on purpose.
    Unswap {
        path: PathBuf,
    },
    /// Open a file in the background with a rename's or code action's edits applied.
    OpenEdited {
        path: PathBuf,
        edits: Vec<crate::services::lsp::TextEdit>,
    },
    /// Make, move or delete a file or folder for the tree.
    Fs(FsOp),
}

/// A change to the project's files that the tree asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FsOp {
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Move { from: PathBuf, to: PathBuf },
    Delete(PathBuf),
}

impl FsOp {
    /// Carries out the change. It never writes over a file or folder that is there.
    pub fn apply(&self) -> Result<(), String> {
        let shown = |p: &Path| format!("\"{}\"", p.display());
        let exists = |p: &Path| p.symlink_metadata().is_ok();
        let parent = |p: &Path| match p.parent() {
            Some(d) if !d.as_os_str().is_empty() => std::fs::create_dir_all(d).map_err(|e| format!("{} {e}", shown(d))),
            _ => Ok(()),
        };
        match self {
            FsOp::NewFile(p) => {
                if exists(p) {
                    return Err(format!("{} already exists", shown(p)));
                }
                parent(p)?;
                std::fs::OpenOptions::new().write(true).create_new(true).open(p).map(|_| ()).map_err(|e| format!("{} {e}", shown(p)))
            }
            FsOp::NewFolder(p) => {
                if exists(p) {
                    return Err(format!("{} already exists", shown(p)));
                }
                std::fs::create_dir_all(p).map_err(|e| format!("{} {e}", shown(p)))
            }
            FsOp::Move { from, to } => {
                if !exists(from) {
                    return Err(format!("{} is not there", shown(from)));
                }
                if to.starts_with(from) {
                    return Err(format!("can't move {} into itself", shown(from)));
                }
                // A rename that only changes the case finds the same file on macOS and Windows.
                if exists(to) && !same_file(from, to) {
                    return Err(format!("{} already exists", shown(to)));
                }
                parent(to)?;
                std::fs::rename(from, to).map_err(|e| format!("{} {e}", shown(from)))
            }
            FsOp::Delete(p) => {
                let meta = p.symlink_metadata().map_err(|e| format!("{} {e}", shown(p)))?;
                let done = if meta.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) };
                done.map_err(|e| format!("{} {e}", shown(p)))
            }
        }
    }
}

/// Whether two paths name one file, as a rename that only changes the case does on a disk
/// that ignores case.
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (a.symlink_metadata(), b.symlink_metadata()) {
            (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// A preview reads at most this much of a file.
const PREVIEW_BYTES: usize = 64 * 1024;

#[cfg(not(target_arch = "wasm32"))]
pub fn start(out: Out) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("editor-io".into())
        .spawn(move || {
            for job in rx {
                match job {
                    Job::Migrate(m) => {
                        if let Some(line) = m.run() {
                            out.send(Event::Migrated(line));
                        }
                    }
                    Job::Open { path, at } => {
                        let mut recovered = None;
                        let buf = Buffer::open(&path).map(|mut b| {
                            if let Some(h) =
                                std::fs::read(undo_file(&path)).ok().and_then(|bytes| serde_json::from_slice::<History>(&bytes).ok())
                            {
                                b.restore_history(h);
                            }
                            if let Some(swap) =
                                std::fs::read(swap_file(&path)).ok().and_then(|bytes| serde_json::from_slice::<Swap>(&bytes).ok())
                                && swap.path == path
                                && swap.text != b.text()
                            {
                                recovered = Some(swap.text);
                            }
                            Box::new(b)
                        });
                        let buf = buf.map_err(|e| format!("\"{}\" {e}", path.display()));
                        let ec = editorconfig::for_file(&path);
                        out.send(Event::Opened { path, buf, at, ec, recovered, background: false });
                    }
                    Job::OpenEdited { path, edits } => {
                        let buf = Buffer::open(&path).map(|mut b| {
                            crate::app::apply_edits(&mut b, &edits);
                            Box::new(b)
                        });
                        let buf = buf.map_err(|e| format!("\"{}\" {e}", path.display()));
                        let ec = editorconfig::for_file(&path);
                        out.send(Event::Opened { path, buf, at: None, ec, recovered: None, background: true });
                    }
                    Job::History { path, history } => {
                        if let Ok(json) = serde_json::to_vec(&*history)
                            && json.len() <= MAX_HISTORY_BYTES
                        {
                            let _ = write_atomic(&undo_file(&path), &json);
                        }
                    }
                    Job::Swap { path, text } => write_swap(path, text),
                    Job::Unswap { path } => {
                        let _ = std::fs::remove_file(swap_file(&path));
                    }
                    Job::Save { path, bytes, version, then } => {
                        let result = write_atomic(&path, &bytes)
                            .map(|_| (bytecount_lines(&bytes), bytes.len()))
                            .map_err(|e| format!("\"{}\" E212: Can't open file for writing: {e}", path.display()));
                        let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                        out.send(Event::Saved { path, version, mtime, result, then });
                    }
                    Job::ReadDisk { path } => {
                        if let Ok(bytes) = std::fs::read(&path) {
                            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                            out.send(Event::DiskText { path, text: String::from_utf8_lossy(&bytes).into_owned(), mtime });
                        }
                    }
                    Job::Write { what, path, text } => {
                        let result = write_atomic(&path, text.as_bytes()).map_err(|e| format!("\"{}\" {e}", path.display()));
                        out.send(Event::Wrote { what, result });
                    }
                    Job::Fs(op) => {
                        let result = op.apply();
                        out.send(Event::FsDone { op, result });
                    }
                    Job::Preview { path } => {
                        use std::io::Read;
                        let mut bytes = Vec::new();
                        if let Ok(f) = std::fs::File::open(&path) {
                            let _ = f.take(PREVIEW_BYTES as u64).read_to_end(&mut bytes);
                        }
                        let binary = bytes.iter().take(8000).any(|b| *b == 0);
                        let text = if binary { "(binary file)".to_string() } else { String::from_utf8_lossy(&bytes).replace("\r\n", "\n") };
                        out.send(Event::Preview { path, text });
                    }
                }
            }
        })
        .expect("start the file worker");
    tx
}

#[cfg(not(target_arch = "wasm32"))]
fn bytecount_lines(bytes: &[u8]) -> usize {
    let n = bytes.iter().filter(|b| **b == b'\n').count();
    if bytes.last().is_some_and(|b| *b != b'\n') { n + 1 } else { n }
}

/// Writes `bytes` to a temporary file beside `path`, then renames it over `path`, so a crash
/// mid-write never leaves half a file. Keeps the old file's permissions.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dir.join(format!(".{name}.editor-{}.tmp", std::process::id()));
    std::fs::write(&tmp, bytes)?;
    if let Ok(meta) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(&tmp, meta.permissions());
    }
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}
