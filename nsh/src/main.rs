//! editor: a lightweight modal editor for opening code fast, reading it, finding your way
//! around it and making quick edits, drawn with each platform's native views.
//!
//! `edit [path[:line]] [+line] [--wait] [--new]` opens a folder or files, in the editor
//! already running for the project when there is one. `edit ctl` and `edit mcp` talk to a
//! running editor (see `docs/agents.md`).

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

use nsh_app_editor::args::{self, Command};
use nsh_app_editor::{DEFAULT_SIZE, ipc, timing};

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    timing::start();
    if let Some(code) = without_window() {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        std::process::exit(code);
    }
    timing::mark("arguments and running-editor check");
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(nsh_app_editor::root()),
        platform::AppOptions {
            title: "nsvim".to_owned(),
            app_id: "dev.nsvim.app".to_owned(),
            size: (DEFAULT_SIZE.0 as i32, DEFAULT_SIZE.1 as i32),
            ..Default::default()
        },
    )
}

/// Runs what needs no window: help, the version, `ctl`, `mcp`, and opening files in the
/// editor already running for the project. Returns the exit status, or None to open a window.
fn without_window() -> Option<i32> {
    let argv: Vec<String> = std::env::args().collect();
    let (wait, new) = match args::command(&argv) {
        Command::Ctl(rest) => return Some(ipc::client::main(&rest)),
        Command::Mcp => return Some(ipc::mcp::main()),
        Command::McpHelp => {
            print!("{}", args::MCP_USAGE);
            return Some(0);
        }
        Command::Help => {
            print!("{}", args::USAGE);
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

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("nsvim runs on macOS, Windows and Linux.");
