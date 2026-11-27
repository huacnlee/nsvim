//! Updates. The window of a copy the installers put in place uses NSH's updater
//! (`nsh.json` `"updater"`): at startup it checks quietly, and NSH's update window asks the
//! user, downloads the release, replaces the app and restarts it, or installs it on quit once
//! the user chose automatic updates. `:Update` and the palette's "Check for updates…" open
//! that window. The terminal editor, and copies that NSH cannot replace, keep the daily check
//! of `services::update`, its message, and `:Update` with the install script.

use super::App;
use super::fx::{Effect, Tx};
use crate::services::update::{self, Checked};

/// Whether the program at `exe` updates with NSH's updater: a window (not the terminal
/// editor) of a copy that the installers put in place. NSH replaces that folder or app as a
/// whole, as the installers do. Other copies (a build, mise's) update the way they came.
pub(crate) fn native_updates(exe: Option<&std::path::Path>, terminal: bool, os: &str) -> bool {
    !terminal && !cfg!(target_arch = "wasm32") && exe.is_some_and(|e| update::installed(e, os).is_some())
}

fn running_exe() -> Option<std::path::PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.canonicalize().ok())
}

impl App {
    /// The check at startup, when `[update] check` is on and this is not a development build.
    pub(crate) fn check_update_at_start(&mut self, tx: &mut Tx) {
        let exe = running_exe();
        if !self.cfg.update.check || crate::app::setup::dev_build(exe.as_deref()) {
            return;
        }
        if native_updates(exe.as_deref(), crate::terminal(), std::env::consts::OS) {
            tx.effects.push(Effect::PromptUpdate { quiet: true });
        } else if let Some(svc) = &self.svc {
            svc.check_update(false);
        }
    }

    pub(crate) fn update_checked(&mut self, checked: Checked) {
        match checked.result {
            Ok(Some(release)) => {
                let cur = env!("CARGO_PKG_VERSION");
                self.ed.message(format!("NsVim {} is available (you have {cur}). Run :Update to install it.", release.version));
                self.update = Some(release);
            }
            Ok(None) if checked.manual => {
                self.ed.message(format!("NsVim {} is the latest version", env!("CARGO_PKG_VERSION")));
            }
            Err(e) if checked.manual => self.ed.error(e),
            _ => {}
        }
    }

    /// `:Update` and "Check for updates…": NSH's update window, or else installs the release a
    /// check found, or checks first.
    pub(crate) fn run_update(&mut self, tx: &mut Tx) {
        if native_updates(running_exe().as_deref(), crate::terminal(), std::env::consts::OS) {
            tx.effects.push(Effect::PromptUpdate { quiet: false });
            return;
        }
        let Some(release) = self.update.clone() else {
            self.ed.message("Checking for updates…");
            if let Some(svc) = &self.svc {
                svc.check_update(true);
            }
            return;
        };
        let dirty = self.ed.docs.iter().filter(|d| d.buf.is_dirty()).count();
        if dirty > 0 {
            self.ed.error(format!("Save your changes first ({dirty} unsaved), then run :Update again"));
            return;
        }
        self.install_update(&release, tx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn install_update(&mut self, release: &update::Release, tx: &mut Tx) {
        let exe = std::env::current_exe().ok().and_then(|e| e.canonicalize().ok());
        let Some((exe, at)) = exe.and_then(|e| update::installed(&e, std::env::consts::OS).map(|at| (e, at))) else {
            self.ed.error(format!(
                "NsVim {} is out: this copy was not installed by the install script, so update it the way you installed it ({})",
                release.version, release.url
            ));
            return;
        };
        let exe = update::relaunch(&at, &exe, std::env::consts::OS);
        match update::start_helper(&at, &exe) {
            Ok(()) => {
                self.ed.message(format!("Installing NsVim {}; NsVim restarts when it is done", release.version));
                self.quit_all(true, tx);
            }
            Err(e) => self.ed.error(e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn install_update(&mut self, release: &update::Release, _tx: &mut Tx) {
        self.ed.message(format!("NsVim {} is out: {}", release.version, release.url));
    }
}
