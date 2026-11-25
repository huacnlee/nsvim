//! The editor in a terminal: the same App.tsx bundle, the same Rust state and keymaps, drawn
//! as text cells by NSH's terminal host. `nsvim` in a terminal starts it. A Linux session with no
//! display starts it too, so the editor works over SSH.

use std::path::PathBuf;
use std::process::ExitCode;

/// The JavaScript of App.tsx in the program's `nsh` folder: next to the program on Linux and
/// Windows, in `Contents/Resources` of the app on macOS. The terminal host loads JavaScript, not
/// the window's bytecode, so a release ships `bundle.js` too.
fn bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe = exe.canonicalize().unwrap_or(exe);
    let dir = exe.parent()?;
    [dir.join("nsh"), dir.join("../Resources/nsh")].into_iter().map(|d| d.join("bundle.js")).find(|p| p.is_file())
}

pub fn main() -> ExitCode {
    let Some(bundle) = bundle() else {
        eprintln!("nsvim: nsh/bundle.js is not next to this program. Run `nsh build`, or install a release.");
        return ExitCode::FAILURE;
    };
    nsvim_native::set_terminal(true);
    let mut options = nsh_platform_tui::AppOptions::default();
    options.ui_sets = vec![nsh_base::registry(), nsvim_native::registry()];
    let mut args = nsh_tui_host::Args::default();
    args.bundle = Some(bundle);
    match nsh_tui_host::main_with_args(options, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("nsvim: {error}");
            ExitCode::FAILURE
        }
    }
}
