//! Opening another project: `Open project` (`SPC p`, Ctrl/⌘ O, `:Projects`, the palette)
//! lists the recent projects and a folder dialog. The pick opens in a window of its own, as
//! `nvim <folder>` does, so an editor already running for that project takes it. A window
//! with no file open gives its place to the new one.

use std::path::{Path, PathBuf};

use super::App;
use super::fx::Tx;

/// A project's folder as the list shows it: under the home folder as `~/…`.
pub fn shown(root: &Path) -> String {
    shown_from(root, dirs::home_dir().as_deref())
}

fn shown_from(root: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| root.strip_prefix(h).ok()) {
        Some(rest) if !rest.as_os_str().is_empty() => format!("~/{}", rest.display()),
        _ => root.display().to_string(),
    }
}

impl App {
    /// Opens `root` as a project in a window of its own. This window closes when it has no
    /// file open, so opening a project right after start feels like opening it here.
    pub fn open_project(&mut self, root: PathBuf, tx: &mut Tx) {
        if root == self.ed.root {
            return;
        }
        if !root.is_dir() {
            return self.ed.error(format!("not a folder: {}", root.display()));
        }
        tx.open_project(root);
        if !self.ed.has_doc() {
            self.quit(tx);
        }
    }

    /// Shows the system's folder dialog, then opens the folder picked.
    pub fn choose_folder(&mut self, tx: &mut Tx) {
        let start = self.ed.root.parent().map(Path::to_path_buf).unwrap_or_else(|| self.ed.root.clone());
        tx.choose_folder(start, |a, picked, tx| match picked {
            Ok(Some(root)) => a.open_project(root, tx),
            Ok(None) => {}
            Err(e) => a.ed.error(format!("no folder dialog: {e}. Run `nvim <folder>` instead")),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_under_home_show_with_a_tilde() {
        let home = Path::new("/home/dev");
        assert_eq!(shown_from(Path::new("/home/dev/work/nsh"), Some(home)), "~/work/nsh");
        assert_eq!(shown_from(Path::new("/srv/app"), Some(home)), "/srv/app");
        assert_eq!(shown_from(home, Some(home)), "/home/dev");
    }
}

#[cfg(test)]
mod flow_tests {
    use std::path::PathBuf;

    use editor::editor::{KeyCode, KeyInput};

    use crate::app::fx::{Effect, Tx};
    use crate::app::{App, Overlay, PickKind};

    fn app(root: PathBuf) -> App {
        let cfg = toml::from_str("[ipc]\nenabled = false\n").unwrap();
        App::new(cfg, root, false, None)
    }

    #[test]
    fn the_leader_p_lists_the_folder_dialog_first() {
        let mut a = app(PathBuf::from("/project"));
        let mut tx = Tx::new();
        a.key(KeyInput::ch(' '), &mut tx);
        a.key(KeyInput::ch('p'), &mut tx);
        let Overlay::Pick(p) = &a.overlay else { panic!("no picker") };
        assert_eq!(p.kind, PickKind::Projects);
        assert_eq!(p.items[0], "Open folder…");
        a.key(KeyInput::plain(KeyCode::Enter), &mut tx);
        assert!(matches!(a.overlay, Overlay::None));
        assert!(tx.effects.iter().any(|e| matches!(e, Effect::ChooseFolder(..))));
    }

    #[test]
    fn a_window_with_no_file_open_gives_its_place_to_the_project() {
        let other = tempfile::tempdir().unwrap();
        let mut a = app(PathBuf::from("/project"));
        let mut tx = Tx::new();
        a.open_project(other.path().to_path_buf(), &mut tx);
        assert!(matches!(tx.effects.first(), Some(Effect::OpenProject(p)) if p == other.path()));
        assert!(tx.effects.iter().any(|e| matches!(e, Effect::Quit)));
    }

    #[test]
    fn the_same_project_or_a_missing_folder_opens_nothing() {
        let mut a = app(PathBuf::from("/project"));
        let mut tx = Tx::new();
        a.open_project(PathBuf::from("/project"), &mut tx);
        a.open_project(PathBuf::from("/no/such/folder"), &mut tx);
        assert!(tx.effects.is_empty());
    }
}
