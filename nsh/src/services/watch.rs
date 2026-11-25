//! Watches the project, open files outside it and Omarchy's theme folder. Git checkouts and AI
//! agents change files underneath the editor; the app reloads them cleanly.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, channel};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

use super::{Event, Out};

pub enum Job {
    Add(PathBuf),
    /// Watch the config file. Its folder is watched, as editors replace the file on save.
    Config(PathBuf),
}

enum Msg {
    Fs(notify::Result<notify::Event>),
    Job(Job),
}

pub fn start(root: PathBuf, out: Out) -> Sender<Job> {
    let (job_tx, job_rx) = channel::<Job>();
    let (tx, rx) = channel::<Msg>();
    let fs_tx = tx.clone();
    std::thread::Builder::new()
        .name("editor-watch-jobs".into())
        .spawn(move || {
            for job in job_rx {
                if tx.send(Msg::Job(job)).is_err() {
                    break;
                }
            }
        })
        .expect("start the watch job relay");
    std::thread::Builder::new()
        .name("editor-watch".into())
        .spawn(move || {
            let Ok(mut watcher) = notify::recommended_watcher(move |e| {
                let _ = fs_tx.send(Msg::Fs(e));
            }) else {
                return;
            };
            let _ = watcher.watch(&root, RecursiveMode::Recursive);
            let omarchy = crate::theme::omarchy_theme_dir().and_then(|d| d.parent().map(Path::to_path_buf));
            if let Some(dir) = &omarchy {
                let _ = watcher.watch(dir, RecursiveMode::Recursive);
            }
            let mut config: Option<PathBuf> = None;
            while let Ok(first) = rx.recv() {
                // Batch what arrives within 100 ms: one save is several events.
                let mut msgs = vec![first];
                std::thread::sleep(Duration::from_millis(100));
                msgs.extend(rx.try_iter());
                let mut changed: Vec<PathBuf> = Vec::new();
                let mut theme = false;
                let mut config_changed = false;
                for m in msgs {
                    match m {
                        Msg::Job(Job::Add(path)) => {
                            if !path.starts_with(&root) {
                                let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
                            }
                        }
                        Msg::Job(Job::Config(file)) => {
                            if let Some(dir) = file.parent() {
                                let _ = std::fs::create_dir_all(dir);
                                if !dir.starts_with(&root) {
                                    let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
                                }
                            }
                            config = Some(file);
                        }
                        Msg::Fs(Ok(e)) => {
                            for p in e.paths {
                                if config.as_ref() == Some(&p) {
                                    config_changed = true;
                                }
                                if omarchy.as_ref().is_some_and(|d| p.starts_with(d)) {
                                    theme = true;
                                } else if !ignored(&root, &p) && !changed.contains(&p) {
                                    changed.push(p);
                                }
                            }
                        }
                        Msg::Fs(Err(_)) => {}
                    }
                }
                if theme {
                    out.send(Event::Omarchy(crate::theme::read_omarchy()));
                }
                if let Some(file) = config.as_ref().filter(|_| config_changed)
                    && let Ok(src) = std::fs::read_to_string(file)
                {
                    out.send(Event::Config(crate::config::parse(file, &src)));
                }
                if !changed.is_empty() {
                    out.send(Event::FsChanged(changed));
                }
            }
        })
        .expect("start the watcher");
    job_tx
}

/// Build output, git's object store and the editor's own temporary files change all the time and
/// never need a reload.
fn ignored(root: &Path, p: &Path) -> bool {
    let name = p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    if name.contains(".editor-") && name.ends_with(".tmp") {
        return true;
    }
    let Ok(rel) = p.strip_prefix(root) else { return false };
    let mut parts = rel.components().map(|c| c.as_os_str().to_string_lossy().into_owned());
    match parts.next().as_deref() {
        Some("target") | Some("node_modules") => true,
        Some(".git") => !matches!(parts.next().as_deref(), Some("HEAD") | Some("index")),
        _ => false,
    }
}
