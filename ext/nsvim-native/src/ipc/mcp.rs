//! `nsvim mcp`: a Model Context Protocol server over stdio, so AI agents (Claude Code, Codex,
//! opencode) can see and drive the running editor. Each tool call is a request on the
//! project's control socket. Connect it with, for example, `claude mcp add nsvim -- PROGRAM mcp`,
//! where PROGRAM is the binary's full path (`nvim --help` shows it).

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};

const INSTRUCTIONS: &str = "Tools for the user's running `editor` window. Call editor_state first to see what the \
user is looking at: the active file, the cursor and the selection. editor_read returns a buffer's text with \
unsaved edits, which can differ from the file on disk. editor_diagnostics returns the editor's language \
server errors. Prefer editor_propose for changes the user should review; it shows a diff they accept or \
reject. Use editor_reveal to point the user at lines. Lines and columns are 1-based.";

fn tools() -> Value {
    let path = json!({ "type": "string", "description": "File path, absolute or relative to the project root. Omit for the active file." });
    json!([
        { "name": "editor_state", "description": "What the user sees: project root, mode, the active file with cursor, selection and visible lines, open buffers with unsaved state, diagnostic counts.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "editor_selection", "description": "The text the user selected.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "editor_read", "description": "A buffer's text as the editor holds it, unsaved edits included. Optional 1-based line range.", "inputSchema": { "type": "object", "properties": { "path": path, "start_line": { "type": "integer" }, "end_line": { "type": "integer" } } } },
        { "name": "editor_diagnostics", "description": "Language server errors and warnings the editor shows, for one file or every open file.", "inputSchema": { "type": "object", "properties": { "path": path } } },
        { "name": "editor_changes", "description": "Changes made to files on disk while the editor was open (by you, git, or another tool), newest first.", "inputSchema": { "type": "object", "properties": {} } },
        { "name": "editor_open", "description": "Open a file in the editor, optionally at a 1-based line and column, and bring the window forward.", "inputSchema": { "type": "object", "properties": { "path": path, "line": { "type": "integer" }, "column": { "type": "integer" } }, "required": ["path"] } },
        { "name": "editor_reveal", "description": "Jump the editor to lines and highlight them, with an optional note the user reads in the editor.", "inputSchema": { "type": "object", "properties": { "path": path, "line": { "type": "integer" }, "end_line": { "type": "integer" }, "note": { "type": "string" } }, "required": ["line"] } },
        { "name": "editor_message", "description": "Show a short line on the editor's command line.", "inputSchema": { "type": "object", "properties": { "text": { "type": "string" }, "error": { "type": "boolean" } }, "required": ["text"] } },
        { "name": "editor_propose", "description": "Propose the whole new text of a file. The editor shows it as a diff; the user accepts with y or rejects with n. Nothing changes until they accept.", "inputSchema": { "type": "object", "properties": { "path": path, "text": { "type": "string" }, "label": { "type": "string", "description": "A few words on what the change does." } }, "required": ["path", "text"] } },
        { "name": "editor_edit", "description": "Change a buffer at once, as one undo step: either `text` (the whole new text) or `edits` (1-based start and end positions with replacement text). The user can undo it with u.", "inputSchema": { "type": "object", "properties": { "path": path, "text": { "type": "string" }, "label": { "type": "string" }, "edits": { "type": "array", "items": { "type": "object", "properties": { "start": { "type": "object", "properties": { "line": { "type": "integer" }, "column": { "type": "integer" } } }, "end": { "type": "object", "properties": { "line": { "type": "integer" }, "column": { "type": "integer" } } }, "text": { "type": "string" } } } } }, "required": ["path"] } },
        { "name": "editor_save", "description": "Write a buffer to disk, or every modified buffer with all=true.", "inputSchema": { "type": "object", "properties": { "path": path, "all": { "type": "boolean" } } } },
        { "name": "editor_command", "description": "Run an Ex command in the editor, such as \":w\" or \":%s/old/new/g\".", "inputSchema": { "type": "object", "properties": { "line": { "type": "string" } }, "required": ["line"] } }
    ])
}

/// The socket request for a tool.
fn command_for(tool: &str) -> Option<&'static str> {
    Some(match tool {
        "editor_state" => "state",
        "editor_selection" => "selection",
        "editor_read" => "read",
        "editor_diagnostics" => "diagnostics",
        "editor_changes" => "changes",
        "editor_open" => "open",
        "editor_reveal" => "reveal",
        "editor_message" => "message",
        "editor_propose" => "propose",
        "editor_edit" => "edit",
        "editor_save" => "save",
        "editor_command" => "command",
        _ => return None,
    })
}

/// Runs the server on stdin and stdout until stdin closes. Returns the exit status.
pub fn main() -> i32 {
    let cwd = std::env::current_dir().unwrap_or_default();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        if let Some(reply) = handle(&msg, &cwd) {
            let _ = writeln!(stdout, "{reply}");
            let _ = stdout.flush();
        }
    }
    0
}

/// The reply to one JSON-RPC message, or `None` for a notification.
pub fn handle(msg: &Value, cwd: &std::path::Path) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let method = msg["method"].as_str().unwrap_or("");
    let result = match method {
        "initialize" => {
            let version = msg.pointer("/params/protocolVersion").and_then(Value::as_str).unwrap_or("2025-06-18");
            json!({
                "protocolVersion": version,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "nsvim", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": tools() }),
        "tools/call" => call_tool(&msg["params"], cwd),
        _ => {
            return Some(
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("method not found: {method}") } }),
            );
        }
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn call_tool(params: &Value, cwd: &std::path::Path) -> Value {
    let text = |t: String, error: bool| json!({ "content": [{ "type": "text", "text": t }], "isError": error });
    let name = params["name"].as_str().unwrap_or("");
    let Some(cmd) = command_for(name) else { return text(format!("unknown tool {name}"), true) };
    let socket: Option<PathBuf> = super::find(cwd);
    let Some(socket) = socket else {
        return text(format!("No nsvim is running for {}. Ask the user to start one with `nvim .` in the project.", cwd.display()), true);
    };
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    match super::call(&socket, cmd, args, Some(Duration::from_secs(30))) {
        Ok(r) if r.ok => {
            let out = match (cmd, &r.result) {
                ("read" | "selection", v) => v["text"].as_str().unwrap_or("").to_string(),
                (_, v) => serde_json::to_string_pretty(v).unwrap_or_default(),
            };
            text(out, false)
        }
        Ok(r) => text(r.error.unwrap_or_default(), true),
        Err(e) => text(format!("the editor did not answer: {e}"), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_and_tools_list_answer_and_notifications_do_not() {
        let cwd = std::env::temp_dir();
        let init =
            handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}}), &cwd).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2025-03-26");
        assert_eq!(init["result"]["serverInfo"]["name"], "nsvim");
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}), &cwd).is_none());
        let list = handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}), &cwd).unwrap();
        let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"editor_state") && names.contains(&"editor_propose"));
        assert!(names.iter().all(|n| command_for(n).is_some()), "every tool maps to a socket command");
        let missing = handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "nope"}), &cwd).unwrap();
        assert_eq!(missing["error"]["code"], -32601);
    }
}
