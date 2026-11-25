//! Runs editor on this machine's native backend: `cargo run -- [file]`.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    let path = std::env::args().nth(1);
    platform::run_app(
        platform::AppSource::Rust(nsh_app_editor::root(path)),
        platform::AppOptions {
            title: "editor".to_owned(),
            #[cfg(any(target_os = "linux", target_os = "windows"))]
            app_id: "dev.editor.app".to_owned(),
            size: (1440, 900),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("editor runs on macOS, Windows and Linux.");
