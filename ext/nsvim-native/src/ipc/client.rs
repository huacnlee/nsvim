//! `PROGRAM ctl` (`nsvim ctl` on PATH): the command-line client of a running editor, for shells, scripts and agents.
//! It also opens files in a running editor when `edit file` finds one for the project.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

const USAGE: &str = "nsvim ctl: talk to the running editor of this folder.

Usage: PROGRAM ctl [--socket PATH] [--json] COMMAND [ARGS]

PROGRAM is the full path of nsvim. nsvim ctl works too when nsvim is on PATH.

Read:
  state                         the active file, cursor, selection, open buffers (JSON)
  selection                     the selected text
  read [PATH] [--lines A-B]     a buffer's text, unsaved edits included
  diagnostics [PATH]            language server errors and warnings (JSON)
  changes                       changes made on disk while the editor was open (JSON)
  list                          every running editor

Act:
  open PATH[:LINE[:COL]] [--wait]   open a file; --wait returns when its buffer closes
  reveal PATH:LINE[-END] [--note TEXT]   jump to lines and highlight them
  reveal --clear                remove the highlight
  message TEXT [--error]        show a line on the command line
  propose PATH [--label TEXT]   show the new text on stdin as a diff to accept or reject
  edit PATH [--label TEXT]      replace a buffer's text with stdin, as one undo step
  save [PATH] [--all]           write a buffer, or every modified one
  command ':EX COMMAND'         run an Ex command, such as ':w' or ':%s/a/b/g'

Paths are relative to the project root. Lines and columns are 1-based.
Exit status: 0 done, 1 the editor refused, 2 no editor or bad usage.";

/// Runs `nsvim ctl ARGS` and returns the exit status.
pub fn main(args: &[String]) -> i32 {
    let mut socket: Option<PathBuf> = None;
    let mut json_out = false;
    let mut rest: Vec<String> = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--socket" => socket = it.next().map(PathBuf::from),
            "--json" => json_out = true,
            "-h" | "--help" | "help" => {
                println!("{}", USAGE.replace("PROGRAM", &crate::args::program()));
                return 0;
            }
            _ => rest.push(a.clone()),
        }
    }
    let Some(cmd) = rest.first().cloned() else {
        eprintln!("{}", USAGE.replace("PROGRAM", &crate::args::program()));
        return 2;
    };
    let args = &rest[1..];
    if cmd == "list" {
        for s in super::running() {
            match super::call(&s, "ping", json!({}), Some(Duration::from_secs(2))) {
                Ok(r) => println!("{}\t{}", r.result["root"].as_str().unwrap_or("?"), s.display()),
                Err(_) => continue,
            }
        }
        return 0;
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let Some(socket) = socket.or_else(|| super::find(&cwd)) else {
        eprintln!("nsvim ctl: no nsvim is running for {}. Start one with `nsvim .`", cwd.display());
        return 2;
    };
    let (req, args_json, shape) = match build(&cmd, args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("nsvim ctl: {e}");
            return 2;
        }
    };
    let wait = args_json["wait"].as_bool() == Some(true);
    let timeout = if wait { None } else { Some(Duration::from_secs(10)) };
    match super::call(&socket, &req, args_json, timeout) {
        Ok(r) if r.ok => {
            print(&r.result, if json_out { Shape::Json } else { shape });
            0
        }
        Ok(r) => {
            eprintln!("nsvim: {}", r.error.unwrap_or_default());
            1
        }
        Err(e) => {
            eprintln!("nsvim ctl: {e}");
            2
        }
    }
}

/// How a reply prints.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape {
    Json,
    /// The `text` field, raw.
    Text,
    /// The `message` field when there is one, else nothing.
    Quiet,
}

fn print(v: &Value, shape: Shape) {
    match shape {
        Shape::Json => println!("{}", serde_json::to_string_pretty(v).unwrap_or_default()),
        Shape::Text => print!("{}", v["text"].as_str().unwrap_or("")),
        Shape::Quiet => {
            if let Some(m) = v["message"].as_str() {
                println!("{m}");
            }
        }
    }
}

/// `path:line:col` → path and 1-based line and column.
fn split_place(s: &str) -> (String, Option<u64>, Option<u64>) {
    let mut parts = s.rsplitn(3, ':').collect::<Vec<_>>();
    parts.reverse();
    match parts.as_slice() {
        [p, l, c] if l.parse::<u64>().is_ok() && c.parse::<u64>().is_ok() => (p.to_string(), l.parse().ok(), c.parse().ok()),
        [p, l, c] if c.parse::<u64>().is_ok() => (format!("{p}:{l}"), c.parse().ok(), None),
        [p, l] if l.parse::<u64>().is_ok() => (p.to_string(), l.parse().ok(), None),
        _ => (s.to_string(), None, None),
    }
}

fn flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn value(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

/// The first argument that isn't a flag or a flag's value.
fn positional(args: &[String], takes_value: &[&str]) -> Option<String> {
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if takes_value.contains(&a.as_str()) {
            skip = true;
            continue;
        }
        if !a.starts_with("--") {
            return Some(a.clone());
        }
    }
    None
}

fn stdin_text() -> Result<String, String> {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s).map_err(|e| format!("reading stdin: {e}"))?;
    Ok(s)
}

/// The request for `cmd args`: its command, its arguments and how its reply prints.
fn build(cmd: &str, args: &[String]) -> Result<(String, Value, Shape), String> {
    let token = std::env::var("XDG_ACTIVATION_TOKEN").unwrap_or_default();
    Ok(match cmd {
        "state" | "diagnostics" | "changes" | "ping" => {
            let mut a = json!({});
            if let Some(p) = positional(args, &[]) {
                a["path"] = json!(p);
            }
            (cmd.into(), a, Shape::Json)
        }
        "selection" => ("selection".into(), json!({}), Shape::Text),
        "read" | "buffer" => {
            let mut a = json!({});
            if let Some(p) = positional(args, &["--lines"]) {
                a["path"] = json!(p);
            }
            if let Some(r) = value(args, "--lines") {
                let (s, e) = r.split_once('-').unwrap_or((&r, &r));
                a["start_line"] = json!(s.parse::<u64>().map_err(|_| "--lines takes A-B")?);
                a["end_line"] = json!(e.parse::<u64>().map_err(|_| "--lines takes A-B")?);
            }
            ("read".into(), a, Shape::Text)
        }
        "open" | "goto" => {
            let p = positional(args, &[]).ok_or("open needs a path")?;
            let (path, line, col) = split_place(&p);
            (
                "open".into(),
                json!({ "path": path, "line": line, "column": col, "wait": flag(args, "--wait"), "activation_token": token }),
                Shape::Quiet,
            )
        }
        "reveal" => {
            if flag(args, "--clear") {
                return Ok(("reveal".into(), json!({ "clear": true }), Shape::Quiet));
            }
            let p = positional(args, &["--note"]).ok_or("reveal needs PATH:LINE or PATH:LINE-END")?;
            let (path, range) = p.rsplit_once(':').ok_or("reveal needs PATH:LINE or PATH:LINE-END")?;
            let (a, b) = range.split_once('-').unwrap_or((range, range));
            let line: u64 = a.parse().map_err(|_| "reveal needs a line number")?;
            let end: u64 = b.parse().map_err(|_| "reveal needs a line number")?;
            (
                "reveal".into(),
                json!({ "path": path, "line": line, "end_line": end, "note": value(args, "--note"), "activation_token": token }),
                Shape::Quiet,
            )
        }
        "message" => {
            let text = args.iter().filter(|a| *a != "--error").cloned().collect::<Vec<_>>().join(" ");
            ("message".into(), json!({ "text": text, "error": flag(args, "--error") }), Shape::Quiet)
        }
        "propose" | "edit" => {
            let path = positional(args, &["--label"]).ok_or(format!("{cmd} needs a path"))?;
            let text = stdin_text()?;
            (cmd.into(), json!({ "path": path, "text": text, "label": value(args, "--label"), "activation_token": token }), Shape::Quiet)
        }
        "save" => {
            let mut a = json!({ "all": flag(args, "--all") });
            if let Some(p) = positional(args, &[]) {
                a["path"] = json!(p);
            }
            ("save".into(), a, Shape::Quiet)
        }
        "command" | "ex" => {
            let line = args.join(" ");
            if line.trim().is_empty() {
                return Err("command needs an Ex command, such as ':w'".into());
            }
            ("command".into(), json!({ "line": line }), Shape::Quiet)
        }
        other => return Err(format!("unknown command `{other}`. Run `nsvim ctl --help`")),
    })
}

/// `nsvim [files] [--wait]` with an editor already running for the project: opens the files
/// there and returns the exit status. `None` when no editor runs for it, so this process
/// opens the window.
pub fn open_in_running(root: &Path, files: &[(PathBuf, Option<editor::buffer::Pos>)], wait: bool) -> Option<i32> {
    let socket = super::socket_for(root);
    if !socket.exists() || !super::alive(&socket) {
        return None;
    }
    let token = std::env::var("XDG_ACTIVATION_TOKEN").unwrap_or_default();
    if files.is_empty() {
        let _ = super::call(&socket, "focus", json!({ "activation_token": token }), Some(Duration::from_secs(5)));
        return Some(0);
    }
    for (i, (path, at)) in files.iter().enumerate() {
        let last = i + 1 == files.len();
        let args = json!({
            "path": path,
            "line": at.map(|p| p.line + 1),
            "column": at.map(|p| p.col + 1),
            "wait": wait && last,
            "activation_token": token,
        });
        let timeout = if wait && last { None } else { Some(Duration::from_secs(10)) };
        match super::call(&socket, "open", args, timeout) {
            Ok(r) if !r.ok => {
                eprintln!("nsvim: {}", r.error.unwrap_or_default());
                return Some(1);
            }
            Ok(_) => {}
            Err(e) => {
                eprintln!("nsvim: {e}");
                return Some(1);
            }
        }
    }
    Some(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_split_into_path_line_and_column() {
        assert_eq!(split_place("src/a.rs:12:4"), ("src/a.rs".into(), Some(12), Some(4)));
        assert_eq!(split_place("src/a.rs:12"), ("src/a.rs".into(), Some(12), None));
        assert_eq!(split_place("src/a.rs"), ("src/a.rs".into(), None, None));
    }

    #[test]
    fn requests_build_from_arguments() {
        let args: Vec<String> = ["src/a.rs:3-9", "--note", "this loop"].iter().map(|s| s.to_string()).collect();
        let (cmd, a, _) = build("reveal", &args).unwrap();
        assert_eq!(cmd, "reveal");
        assert_eq!((a["line"].as_u64(), a["end_line"].as_u64(), a["note"].as_str()), (Some(3), Some(9), Some("this loop")));
        assert!(build("nope", &[]).is_err());
    }
}
