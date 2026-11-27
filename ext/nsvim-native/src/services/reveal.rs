//! "Reveal in Finder": show a file in the system's file manager, with the file selected.
//! Each `reveal` waits for its command, so it runs off the engine thread.
//! The name and the behaviour follow Zed (`reveal_in_file_manager_label`, `reveal_path`).

use std::path::Path;

/// The menu title on this system: Finder on macOS, File Explorer on Windows, else the file
/// manager.
pub const TITLE: &str = if cfg!(target_os = "macos") {
    "Reveal in Finder"
} else if cfg!(target_os = "windows") {
    "Reveal in File Explorer"
} else {
    "Reveal in file manager"
};

#[cfg(not(any(target_os = "windows", target_arch = "wasm32")))]
fn status(cmd: &mut std::process::Command) -> Result<(), String> {
    let name = cmd.get_program().to_string_lossy().into_owned();
    match cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!("{name} failed ({s})")),
        Err(e) => Err(format!("{name}: {e}")),
    }
}

/// `open -R` selects the file in a Finder window.
#[cfg(target_os = "macos")]
pub fn reveal(path: &Path) -> Result<(), String> {
    status(std::process::Command::new("open").arg("-R").arg(path))
}

/// `explorer /select,` selects the file in a File Explorer window. Explorer reads its own
/// command line: the path is in quotes after the comma, not the whole argument. Explorer
/// gives exit code 1 also when it opens, so only a failed start is an error.
#[cfg(target_os = "windows")]
pub fn reveal(path: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("explorer")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map(drop)
        .map_err(|e| format!("explorer: {e}"))
}

/// The file manager's D-Bus call `org.freedesktop.FileManager1.ShowItems` selects the file
/// (Nautilus, Dolphin, Nemo, Thunar and more). Without it, `xdg-open` opens the folder.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_arch = "wasm32")))]
pub fn reveal(path: &Path) -> Result<(), String> {
    let shown = status(
        std::process::Command::new("dbus-send")
            .args(["--session", "--print-reply", "--dest=org.freedesktop.FileManager1", "/org/freedesktop/FileManager1"])
            .arg("org.freedesktop.FileManager1.ShowItems")
            .arg(format!("array:string:{}", file_uri(path)))
            .arg("string:"),
    );
    if shown.is_ok() {
        return shown;
    }
    let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
    status(std::process::Command::new("xdg-open").arg(dir))
}

/// `path` as a `file://` URI. Every byte but the unreserved ones and `/` is percent-encoded,
/// so a `,` cannot split the D-Bus array.
pub fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for &b in path.to_string_lossy().as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b'-' | b'_' | b'.' | b'~') {
            uri.push(b as char);
        } else {
            uri.push_str(&format!("%{b:02X}"));
        }
    }
    uri
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uri_encodes_all_but_the_plain_bytes() {
        assert_eq!(file_uri(Path::new("/home/a/src/lib.rs")), "file:///home/a/src/lib.rs");
        assert_eq!(file_uri(Path::new("/home/a/my file,1.rs")), "file:///home/a/my%20file%2C1.rs");
        assert_eq!(file_uri(Path::new("/home/a/文")), "file:///home/a/%E6%96%87");
    }
}
