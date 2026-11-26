//! `nsvim-tui`: the editor in a terminal, as `nsvim` runs it in a terminal. Windows needs this second
//! program: `nsvim.exe` is a window program and gets no console, so it cannot draw in one.

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[path = "tui.rs"]
mod tui;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> std::process::ExitCode {
    let argv: Vec<String> = std::env::args().collect();
    match nsvim_native::args::command(&argv) {
        nsvim_native::args::Command::Help => {
            print!("{}", nsvim_native::args::usage(&nsvim_native::args::program()));
            std::process::ExitCode::SUCCESS
        }
        nsvim_native::args::Command::Version => {
            println!("nsvim {}", env!("CARGO_PKG_VERSION"));
            std::process::ExitCode::SUCCESS
        }
        _ => tui::main(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn main() {}
