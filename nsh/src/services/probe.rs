//! What the first-launch setup must know about this machine: this binary, whether
//! `~/.local/bin` is on `$PATH`, which command names are taken, and the leader key Neovim
//! uses. Each answer needs the disk, so a worker finds it and sends an `Event::Setup`.

use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, channel};

#[cfg(not(target_arch = "wasm32"))]
use super::{Event, Out};

pub enum Job {
    /// Find the facts that do not change while setup runs.
    Facts,
    /// Look these command names up on `$PATH`.
    LookUp(Vec<String>),
}

/// A worker's answer for the setup screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Facts {
        /// This binary, with links resolved.
        exe: Option<PathBuf>,
        /// `~/.local/bin` is a folder on `$PATH`.
        bin_on_path: bool,
        /// The leader key `~/.config/nvim` sets, if any.
        nvim_leader: Option<char>,
    },
    /// Each name, and where it already is on `$PATH`, if anywhere.
    OnPath(Vec<(String, Option<String>)>),
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start(out: Out) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("editor-probe".into())
        .spawn(move || {
            for job in rx {
                let answer = match job {
                    Job::Facts => Answer::Facts {
                        exe: std::env::current_exe().ok().and_then(|e| e.canonicalize().ok()),
                        bin_on_path: bin_on_path(),
                        nvim_leader: nvim_leader(),
                    },
                    Job::LookUp(names) => Answer::OnPath(names.into_iter().map(|n| (n.clone(), command_v(&n))).collect()),
                };
                out.send(Event::Setup(answer));
            }
        })
        .expect("start the probe worker");
    tx
}

#[cfg(not(target_arch = "wasm32"))]
/// Whether `~/.local/bin` is a folder on `$PATH`.
fn bin_on_path() -> bool {
    let (Some(path), Some(home)) = (std::env::var_os("PATH"), dirs::home_dir()) else { return false };
    let bin = home.join(".local/bin");
    std::env::split_paths(&path).any(|d| d == bin)
}

#[cfg(not(target_arch = "wasm32"))]
/// `command -v name`: the first executable `name` on `$PATH`.
pub fn command_v(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    let found = std::env::split_paths(&path).map(|d| d.join(name)).find(|p| p.is_file())?;
    // This editor, installed under that name (`cargo install`, an earlier setup): not taken.
    let me = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok());
    if me.is_some() && found.canonicalize().ok() == me {
        return None;
    }
    let shown = crate::app::tilde(&found);
    Some(match std::fs::read_link(&found) {
        Ok(target) => format!("{shown}, a link to {}", crate::app::tilde(&target)),
        Err(_) => shown,
    })
}

#[cfg(not(target_arch = "wasm32"))]
/// The leader key `~/.config/nvim` sets (`vim.g.mapleader` or `let mapleader`), if any. Reads
/// `init.lua`, `init.vim` and the Lua files under `lua/`.
fn nvim_leader() -> Option<char> {
    let dir = dirs::config_dir()?.join("nvim");
    let mut files = vec![dir.join("init.lua"), dir.join("init.vim")];
    let mut stack = vec![dir.join("lua")];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() && stack.len() < 64 {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "lua") && files.len() < 400 {
                files.push(p);
            }
        }
    }
    files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).find_map(|src| crate::app::setup::leader_in(&src))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn the_worker_answers_facts_and_names() {
        let (tx, rx) = channel();
        let jobs = start(Out { tx, waker: Arc::new(|| {}) });
        jobs.send(Job::LookUp(vec!["no-such-command-for-editor-tests".into()])).unwrap();
        jobs.send(Job::Facts).unwrap();
        let Ok(Event::Setup(Answer::OnPath(names))) = rx.recv() else { panic!("names first") };
        assert_eq!(names, vec![("no-such-command-for-editor-tests".to_string(), None)]);
        let Ok(Event::Setup(Answer::Facts { exe, .. })) = rx.recv() else { panic!("then the facts") };
        assert!(exe.is_some_and(|e| e.is_absolute()));
    }
}
