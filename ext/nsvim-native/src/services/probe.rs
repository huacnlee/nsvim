//! What the first-launch setup must know about this machine: this binary and the leader key
//! Neovim uses. Each answer needs the disk, so a worker finds it and sends an `Event::Setup`.

use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Sender, channel};

#[cfg(not(target_arch = "wasm32"))]
use super::{Event, Out};

pub enum Job {
    /// Find the facts that do not change while setup runs.
    Facts,
}

/// A worker's answer for the setup screen.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Facts {
        /// This binary, with links resolved.
        exe: Option<PathBuf>,
        /// This is a development build (`setup::dev_build`).
        dev_build: bool,
        /// The leader key `~/.config/nvim` sets, if any.
        nvim_leader: Option<char>,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub fn start(out: Out) -> Sender<Job> {
    let (tx, rx) = channel::<Job>();
    std::thread::Builder::new()
        .name("editor-probe".into())
        .spawn(move || {
            for job in rx {
                let answer = match job {
                    Job::Facts => {
                        let exe = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok());
                        let dev_build = crate::app::setup::dev_build(exe.as_deref());
                        Answer::Facts { exe, dev_build, nvim_leader: nvim_leader() }
                    }
                };
                out.send(Event::Setup(answer));
            }
        })
        .expect("start the probe worker");
    tx
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
    fn the_worker_answers_the_facts() {
        let (tx, rx) = channel();
        let jobs = start(Out { tx, waker: Arc::new(|| {}) });
        jobs.send(Job::Facts).unwrap();
        let Ok(Event::Setup(Answer::Facts { exe, dev_build, .. })) = rx.recv() else { panic!("the facts") };
        let exe = exe.expect("this binary");
        assert!(exe.is_absolute());
        assert_eq!(dev_build, crate::app::setup::dev_build(Some(&exe)));
    }
}
