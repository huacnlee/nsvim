//! The editor's folders were named `editor` before it became nsvim. On the first start after
//! the rename, the old config and state folders are copied to the new names once: config.toml,
//! sessions, undo histories, swap files and window.toml. The old folders stay. A new folder
//! that exists already wins, and nothing is copied into it.
//!
//! `plan` only looks at the disk, at startup. `Migration::run` copies, on the file worker,
//! before the worker opens any file.

use std::path::{Path, PathBuf};

/// The folders to copy: each pair is the old folder and the new one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Migration {
    pub moves: Vec<(PathBuf, PathBuf)>,
}

/// What to copy for this user: each old folder whose new folder does not exist.
pub fn plan() -> Migration {
    plan_for(&[
        (crate::config::old_config_dir(), crate::config::config_dir()),
        (crate::config::old_state_dir(), crate::config::state_dir()),
    ])
}

/// What to copy of `pairs` (old, new): the pairs with an old folder and no new one.
pub fn plan_for(pairs: &[(PathBuf, PathBuf)]) -> Migration {
    Migration { moves: pairs.iter().filter(|(old, new)| old.is_dir() && !new.exists()).cloned().collect() }
}

impl Migration {
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// Copies each old folder to its new place, and never removes the old one. Returns the line
    /// to show on the command line, or None when there was nothing to copy.
    pub fn run(&self) -> Option<String> {
        let mut done = Vec::new();
        let mut failed = Vec::new();
        for (old, new) in &self.moves {
            // Another start may have made the new folder in the meantime. Then it wins.
            if new.exists() && std::fs::read_dir(new).is_ok_and(|mut d| d.next().is_some()) {
                continue;
            }
            match copy_dir(old, new) {
                Ok(()) => done.push(format!("{} to {}", crate::app::tilde(old), crate::app::tilde(new))),
                Err(e) => failed.push(format!("{}: {e}", crate::app::tilde(old))),
            }
        }
        if !failed.is_empty() {
            return Some(format!("nsvim could not copy its old folders: {}", failed.join(", ")));
        }
        (!done.is_empty()).then(|| format!("nsvim copied {}. The old folders stay", done.join(" and ")))
    }
}

/// Copies the files and folders under `from` to `to`. Links and other special files stay.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if kind.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if kind.is_file() && !dest.exists() {
            std::fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(root: &Path) -> (PathBuf, PathBuf) {
        let old = root.join("editor");
        std::fs::create_dir_all(old.join("sessions")).unwrap();
        std::fs::write(old.join("config.toml"), "[keys]\nmap = \"classic\"\n").unwrap();
        std::fs::write(old.join("sessions/p-1.toml"), "root = \"/p\"\n").unwrap();
        std::fs::write(old.join("window.toml"), "w = 900.0\n").unwrap();
        (old, root.join("nsvim"))
    }

    #[test]
    fn the_old_folder_copies_once_and_stays() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = tree(dir.path());
        let m = plan_for(&[(old.clone(), new.clone())]);
        assert_eq!(m.moves.len(), 1);
        let msg = m.run().expect("a line for the command line");
        assert!(msg.contains("copied") && msg.contains("The old folders stay"), "{msg}");
        assert_eq!(std::fs::read_to_string(new.join("config.toml")).unwrap(), "[keys]\nmap = \"classic\"\n");
        assert!(new.join("sessions/p-1.toml").is_file() && new.join("window.toml").is_file());
        assert!(old.join("config.toml").is_file(), "the old folder stays");
        assert!(plan_for(&[(old, new)]).is_empty(), "the next start copies nothing");
    }

    #[test]
    fn a_new_folder_that_exists_wins() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = tree(dir.path());
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("config.toml"), "# mine\n").unwrap();
        assert!(plan_for(&[(old.clone(), new.clone())]).is_empty());
        // A plan made before another start made the new folder copies nothing either.
        let late = Migration { moves: vec![(old, new.clone())] };
        assert_eq!(late.run(), None);
        assert_eq!(std::fs::read_to_string(new.join("config.toml")).unwrap(), "# mine\n");
        assert!(!new.join("sessions").exists());
    }

    #[test]
    fn no_old_folder_means_nothing_to_do() {
        let dir = tempfile::tempdir().unwrap();
        assert!(plan_for(&[(dir.path().join("editor"), dir.path().join("nsvim"))]).is_empty());
    }

    #[test]
    fn startup_reads_the_old_folder_until_the_copy_ran() {
        let dir = tempfile::tempdir().unwrap();
        let (old, new) = tree(dir.path());
        assert_eq!(crate::config::readable(new.clone(), old.clone()), old);
        std::fs::create_dir_all(&new).unwrap();
        assert_eq!(crate::config::readable(new.clone(), old), new);
    }
}
