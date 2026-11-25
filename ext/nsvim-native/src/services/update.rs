//! Updates from GitHub Releases. A worker asks the GitHub API for the latest release, at most
//! once a day, and sends an `Event::Update`. `:Update` installs it with the same install script
//! the README gives (`install.sh` or `install.ps1`): a small helper process waits for this one to
//! exit, runs the script into the folder nsvim is installed in, and starts nsvim again.
//!
//! The decisions (versions, the release JSON, where nsvim is installed, the helper's command)
//! are plain functions so the tests can cover them without the network.

use std::path::{Path, PathBuf};

/// The latest release on GitHub.
pub const LATEST: &str = "https://api.github.com/repos/huacnlee/nsvim/releases/latest";
/// The install scripts the README gives.
pub const INSTALL_SH: &str = "https://huacnlee.github.io/nsvim/install.sh";
pub const INSTALL_PS1: &str = "https://huacnlee.github.io/nsvim/install.ps1";
/// Seconds between two automatic checks.
pub const EVERY: u64 = 24 * 60 * 60;

/// A published release newer than this one.
#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Without the `v`: `0.2.0`.
    pub version: String,
    /// The release page.
    pub url: String,
}

/// What a check found, for `Event::Update`. `manual` is a check `:Update` asked for: it reports
/// "up to date" and errors, which an automatic check keeps quiet about.
#[derive(Clone, Debug, PartialEq)]
pub struct Checked {
    pub result: Result<Option<Release>, String>,
    pub manual: bool,
}

/// `x.y.z` with an optional `-pre` part, after an optional `v`.
fn parse(v: &str) -> Option<([u64; 3], Option<&str>)> {
    let v = v.trim().trim_start_matches('v');
    let v = v.split('+').next()?;
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (v, None),
    };
    let mut n = [0u64; 3];
    let mut parts = core.split('.');
    for slot in &mut n {
        *slot = parts.next()?.parse().ok()?;
    }
    parts.next().is_none().then_some((n, pre))
}

/// `latest` is a newer version than `current`. A prerelease is older than its release;
/// anything that does not parse is never newer.
pub fn newer(current: &str, latest: &str) -> bool {
    let (Some((c, cp)), Some((l, lp))) = (parse(current), parse(latest)) else {
        return false;
    };
    match l.cmp(&c) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Less => false,
        std::cmp::Ordering::Equal => match (cp, lp) {
            (Some(_), None) => true,
            (Some(a), Some(b)) => b > a,
            _ => false,
        },
    }
}

/// The release in the GitHub API's `releases/latest` answer, if it is newer than `current`.
/// Drafts and prereleases are never offered.
pub fn release(json: &str, current: &str) -> Result<Option<Release>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("update check: {e}"))?;
    let tag = v["tag_name"].as_str().ok_or("update check: the answer has no tag_name")?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) || !newer(current, tag) {
        return Ok(None);
    }
    Ok(Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        url: v["html_url"].as_str().unwrap_or("https://github.com/huacnlee/nsvim/releases/latest").to_string(),
    }))
}

/// An automatic check is due: none yet, or the last one was `EVERY` ago (or in the future,
/// after the clock moved back).
pub fn due(last: Option<u64>, now: u64) -> bool {
    last.is_none_or(|t| t > now || now - t >= EVERY)
}

/// Where the install script put nsvim, read from the running executable.
#[derive(Clone, Debug, PartialEq)]
pub enum Installed {
    /// `install.sh`: `NSVIM_INSTALL_DIR` is `dir` (the folder holding `nsvim.app` on macOS,
    /// the prefix holding `share/nsvim` on Linux).
    Unix { dir: PathBuf },
    /// `install.ps1`: the folder holding `nsvim.exe`.
    Windows { dir: PathBuf },
}

/// How `exe` was installed, when the install script installed it: `nsvim.app/Contents/MacOS/nsvim`
/// on macOS, `<prefix>/share/nsvim/nsvim` on Linux, a folder with `nsvim.exe` and `nsh\` on
/// Windows. `None` for anything else (a build, a package manager's copy), which must update the
/// way it was installed.
pub fn installed(exe: &Path, os: &str) -> Option<Installed> {
    let name = exe.file_name()?.to_str()?;
    let dir = exe.parent()?;
    match os {
        "macos" => {
            let app = dir.parent()?.parent()?;
            let ok = name == "nsvim"
                && dir.file_name()? == "MacOS"
                && dir.parent()?.file_name()? == "Contents"
                && app.file_name()? == "nsvim.app";
            ok.then_some(Installed::Unix { dir: app.parent()?.to_path_buf() })
        }
        "windows" => (name.eq_ignore_ascii_case("nsvim.exe") && dir.join("nsh").join("bundle.nshc").is_file())
            .then(|| Installed::Windows { dir: dir.to_path_buf() }),
        _ => {
            let share = dir.parent()?;
            let ok = name == "nsvim" && dir.file_name()? == "nsvim" && share.file_name()? == "share";
            ok.then_some(Installed::Unix { dir: share.parent()?.to_path_buf() })
        }
    }
}

/// A PowerShell single-quoted string.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// The helper that installs the update: the program and its arguments. It waits for process
/// `pid` to exit, runs the install script into `at`, then starts `exe` again.
pub fn helper(at: &Installed, pid: u32, exe: &Path) -> (String, Vec<String>) {
    match at {
        Installed::Unix { dir } => {
            let script = format!(
                "while kill -0 \"$1\" 2>/dev/null; do sleep 0.2; done; \
                 curl -fsSL {INSTALL_SH} | NSVIM_INSTALL_DIR=\"$2\" sh && exec \"$3\""
            );
            let args = vec![
                "-c".into(),
                script,
                "nsvim-update".into(),
                pid.to_string(),
                dir.to_string_lossy().into_owned(),
                exe.to_string_lossy().into_owned(),
            ];
            ("sh".into(), args)
        }
        Installed::Windows { dir } => {
            let script = format!(
                "Wait-Process -Id {pid} -ErrorAction SilentlyContinue; \
                 $env:NSVIM_INSTALL_DIR = {}; \
                 irm {INSTALL_PS1} | iex; \
                 Start-Process -FilePath {}",
                ps_quote(&dir.to_string_lossy()),
                ps_quote(&exe.to_string_lossy()),
            );
            let args = vec![
                "-NoProfile".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-WindowStyle".into(),
                "Hidden".into(),
                "-Command".into(),
                script,
            ];
            ("powershell".into(), args)
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod run {
    use super::*;
    use crate::services::{Event, Out};
    use std::process::{Command, Stdio};

    fn stamp_file() -> PathBuf {
        crate::config::state_dir().join("update-check")
    }

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    /// Asks GitHub with `curl`, which macOS, Linux and Windows 10 and later all have.
    fn fetch() -> Result<String, String> {
        let mut cmd = Command::new("curl");
        cmd.args(["-fsSL", "--max-time", "20", "-H", "Accept: application/vnd.github+json", LATEST])
            .stdin(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let out = cmd.output().map_err(|e| format!("update check: curl: {e}"))?;
        if !out.status.success() {
            return Err(format!("update check failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Checks on a thread. An automatic check (`manual` false) runs only when one is `due`.
    pub fn check(out: Out, manual: bool) {
        let _ = std::thread::Builder::new().name("editor-update".into()).spawn(move || {
            let file = stamp_file();
            let now = now();
            if !manual {
                let last = std::fs::read_to_string(&file).ok().and_then(|s| s.trim().parse().ok());
                if !due(last, now) {
                    return;
                }
            }
            let result = fetch().and_then(|json| release(&json, env!("CARGO_PKG_VERSION")));
            if result.is_ok() {
                let _ = std::fs::create_dir_all(crate::config::state_dir());
                let _ = std::fs::write(&file, now.to_string());
            }
            out.send(Event::Update(Checked { result, manual }));
        });
    }

    /// Starts the helper, detached, so it outlives this process.
    pub fn start_helper(at: &Installed, exe: &Path) -> Result<(), String> {
        let (program, args) = helper(at, std::process::id(), exe);
        let mut cmd = Command::new(program);
        cmd.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000 | 0x0000_0008); // CREATE_NO_WINDOW | DETACHED_PROCESS
        }
        cmd.spawn().map(|_| ()).map_err(|e| format!("update: {e}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use run::{check, start_helper};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(newer("0.1.0", "v0.2.0"));
        assert!(newer("0.1.0", "0.1.1"));
        assert!(newer("0.9.0", "0.10.0"));
        assert!(newer("0.2.0-beta.1", "0.2.0"));
        assert!(newer("0.2.0-beta.1", "0.2.0-beta.2"));
        assert!(!newer("0.2.0", "0.2.0"));
        assert!(!newer("0.2.0", "0.1.9"));
        assert!(!newer("0.2.0", "0.2.0-rc.1"));
        assert!(!newer("0.1.0", "nightly"));
        assert!(!newer("0.1.0", "1.2"));
    }

    #[test]
    fn reads_the_latest_release() {
        let json = r#"{"tag_name":"v0.2.0","html_url":"https://github.com/huacnlee/nsvim/releases/tag/v0.2.0","draft":false,"prerelease":false}"#;
        assert_eq!(
            release(json, "0.1.0").unwrap(),
            Some(Release { version: "0.2.0".into(), url: "https://github.com/huacnlee/nsvim/releases/tag/v0.2.0".into() })
        );
        assert_eq!(release(json, "0.2.0").unwrap(), None);
        let pre = json.replace("\"prerelease\":false", "\"prerelease\":true");
        assert_eq!(release(&pre, "0.1.0").unwrap(), None);
        assert!(release("{\"message\":\"Not Found\"}", "0.1.0").is_err());
        assert!(release("<html>", "0.1.0").is_err());
    }

    #[test]
    fn checks_once_a_day() {
        assert!(due(None, 100));
        assert!(!due(Some(100), 100 + EVERY - 1));
        assert!(due(Some(100), 100 + EVERY));
        assert!(due(Some(200), 100));
    }

    #[test]
    fn finds_where_the_install_script_put_nsvim() {
        assert_eq!(
            installed(Path::new("/Users/a/Applications/nsvim.app/Contents/MacOS/nsvim"), "macos"),
            Some(Installed::Unix { dir: "/Users/a/Applications".into() })
        );
        assert_eq!(
            installed(Path::new("/home/a/.local/share/nsvim/nsvim"), "linux"),
            Some(Installed::Unix { dir: "/home/a/.local".into() })
        );
        assert_eq!(installed(Path::new("/usr/bin/nsvim"), "linux"), None);
        assert_eq!(installed(Path::new("/home/a/nsvim/target/release/nsvim"), "linux"), None);
        assert_eq!(installed(Path::new("/opt/homebrew/bin/nsvim"), "macos"), None);

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("nsvim.exe");
        assert_eq!(installed(&exe, "windows"), None);
        std::fs::create_dir_all(dir.path().join("nsh")).unwrap();
        std::fs::write(dir.path().join("nsh").join("bundle.nshc"), b"").unwrap();
        assert_eq!(installed(&exe, "windows"), Some(Installed::Windows { dir: dir.path().to_path_buf() }));
    }

    #[test]
    fn the_helper_waits_installs_and_restarts() {
        let at = Installed::Unix { dir: "/home/a b/.local".into() };
        let (program, args) = helper(&at, 42, Path::new("/home/a b/.local/share/nsvim/nsvim"));
        assert_eq!(program, "sh");
        assert!(args[1].contains(INSTALL_SH));
        // The paths go in as arguments, never into the script, so spaces and quotes are safe.
        assert_eq!(&args[3..], ["42", "/home/a b/.local", "/home/a b/.local/share/nsvim/nsvim"]);

        let at = Installed::Windows { dir: r"C:\Users\o'neil\AppData\Local\Programs\nsvim".into() };
        let (program, args) = helper(&at, 7, Path::new(r"C:\Users\o'neil\AppData\Local\Programs\nsvim\nsvim.exe"));
        assert_eq!(program, "powershell");
        let script = args.last().unwrap();
        assert!(script.starts_with("Wait-Process -Id 7 "));
        assert!(script.contains(r"$env:NSVIM_INSTALL_DIR = 'C:\Users\o''neil\AppData\Local\Programs\nsvim';"));
        assert!(script.contains(INSTALL_PS1));
        assert!(script.ends_with(r"Start-Process -FilePath 'C:\Users\o''neil\AppData\Local\Programs\nsvim\nsvim.exe'"));
    }
}
