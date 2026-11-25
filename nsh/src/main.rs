//! editor: a lightweight modal editor for opening code fast, reading it, finding your way
//! around it and making quick edits, drawn with each platform's native views.
//!
//! `nvim [path[:line]] [+line] [--wait] [--new]` opens a folder or files, in the editor
//! already running for the project when there is one. `nvim` is the shell alias the first-run
//! setup makes. `PROGRAM ctl` and `PROGRAM mcp`, with the binary's full path, talk to a running
//! editor (see `docs/agents.md`).

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use {nsh_linux_app as runner, nsh_platform_linux as platform};
#[cfg(target_os = "macos")]
use {nsh_macos_app as runner, nsh_platform_macos as platform};
#[cfg(target_os = "windows")]
use {nsh_platform_windows as platform, nsh_windows_app as runner};

#[cfg(not(target_arch = "wasm32"))]
use nsh_app_editor::args::{self, Command};
#[cfg(not(target_arch = "wasm32"))]
use nsh_app_editor::{ipc, timing};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    timing::start();
    if let Some(code) = without_window() {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        std::process::exit(code);
    }
    timing::mark("arguments and running-editor check");
    // The screens are App.tsx, built to `nsh/bundle.nshc` next to this program (`nsh build`, or
    // `nsh run` in development). The runner reads it and links the app's Rust components.
    platform::set_ui_sets(vec![nsh_base::registry(), nsh_app_editor::registry()]);
    runner::main()
}

#[cfg(not(target_arch = "wasm32"))]
/// Runs what needs no window: help, the version, `ctl`, `mcp`, and opening files in the
/// editor already running for the project. Returns the exit status, or None to open a window.
fn without_window() -> Option<i32> {
    let argv: Vec<String> = std::env::args().collect();
    let (wait, new) = match args::command(&argv) {
        Command::Ctl(rest) => return Some(ipc::client::main(&rest)),
        Command::Mcp => return Some(ipc::mcp::main()),
        Command::McpHelp => {
            print!("{}", args::mcp_usage(&args::program()));
            return Some(0);
        }
        Command::Help => {
            print!("{}", args::usage(&args::program()));
            return Some(0);
        }
        Command::Version => {
            println!("nsvim {}", env!("CARGO_PKG_VERSION"));
            return Some(0);
        }
        Command::Open { wait, new } => (wait, new),
    };
    if new {
        return None;
    }
    let (root, files) = args::targets();
    ipc::client::open_in_running(&root, &files, wait)
}

/// On the web, the page runs App.tsx with the library. This program has nothing to do there.
#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows", target_arch = "wasm32")))]
compile_error!("nsvim runs on macOS, Windows and Linux.");
