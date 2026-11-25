//! `:Update` and the message an update check leaves (`services::update`).

use super::App;
use super::fx::Tx;
use crate::services::update::{self, Checked};

impl App {
    pub(crate) fn update_checked(&mut self, checked: Checked) {
        match checked.result {
            Ok(Some(release)) => {
                let cur = env!("CARGO_PKG_VERSION");
                self.ed.message(format!("nsvim {} is available (you have {cur}). Run :Update to install it.", release.version));
                self.update = Some(release);
            }
            Ok(None) if checked.manual => {
                self.ed.message(format!("nsvim {} is the latest version", env!("CARGO_PKG_VERSION")));
            }
            Err(e) if checked.manual => self.ed.error(e),
            _ => {}
        }
    }

    /// `:Update`: installs the release a check found, or checks first.
    pub(crate) fn run_update(&mut self, tx: &mut Tx) {
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
                "nsvim {} is out: this copy was not installed by the install script, so update it the way you installed it ({})",
                release.version, release.url
            ));
            return;
        };
        match update::start_helper(&at, &exe) {
            Ok(()) => {
                self.ed.message(format!("Installing nsvim {}; nsvim restarts when it is done", release.version));
                self.quit_all(true, tx);
            }
            Err(e) => self.ed.error(e),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn install_update(&mut self, release: &update::Release, _tx: &mut Tx) {
        self.ed.message(format!("nsvim {} is out: {}", release.version, release.url));
    }
}
