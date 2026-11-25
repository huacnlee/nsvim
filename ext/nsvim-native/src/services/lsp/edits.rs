//! LSP workspace edits (from rename, code actions and `workspace/applyEdit`) turned into char
//! positions per file, and the file operations they carry.

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use editor::buffer::Pos;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
use super::{from_utf16, path_of};

/// One text change: replace `start..end` with `text`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: Pos,
    pub end: Pos,
    pub text: String,
}

/// The changes to one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEdit {
    pub path: PathBuf,
    pub edits: Vec<TextEdit>,
}

#[cfg(not(target_arch = "wasm32"))]
/// A file created, renamed or deleted by an edit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileOp {
    Create(PathBuf),
    Rename(PathBuf, PathBuf),
    Delete(PathBuf),
}

#[cfg(not(target_arch = "wasm32"))]
/// The text edits and file operations of a `WorkspaceEdit`, in order. `text_of` gives a file's
/// text for turning UTF-16 columns into chars; it is not called when the server counts chars.
pub fn convert(edit: &Value, utf32: bool, text_of: &dyn Fn(&Path) -> Option<String>) -> (Vec<FileEdit>, Vec<FileOp>) {
    let mut files: Vec<FileEdit> = Vec::new();
    let mut ops = Vec::new();
    let mut add = |path: PathBuf, list: &Value| {
        let text = if utf32 { None } else { text_of(&path) };
        let lines: Vec<&str> = text.as_deref().map(|t| t.split('\n').collect()).unwrap_or_default();
        let pos = |p: &Value| {
            let line = p["line"].as_u64().unwrap_or(0) as usize;
            let ch = p["character"].as_u64().unwrap_or(0) as usize;
            let col = if utf32 { ch } else { lines.get(line).map_or(ch, |l| from_utf16(l, ch)) };
            Pos::new(line, col)
        };
        let edits: Vec<TextEdit> = list
            .as_array()
            .into_iter()
            .flatten()
            .map(|e| TextEdit {
                start: pos(&e["range"]["start"]),
                end: pos(&e["range"]["end"]),
                text: e["newText"].as_str().unwrap_or("").to_string(),
            })
            .collect();
        if edits.is_empty() {
            return;
        }
        match files.iter_mut().find(|f| f.path == path) {
            Some(f) => f.edits.extend(edits),
            None => files.push(FileEdit { path, edits }),
        }
    };
    if let Some(changes) = edit["documentChanges"].as_array() {
        for c in changes {
            match c["kind"].as_str() {
                Some("create") => ops.extend(c["uri"].as_str().and_then(path_of).map(FileOp::Create)),
                Some("rename") => {
                    if let (Some(a), Some(b)) = (c["oldUri"].as_str().and_then(path_of), c["newUri"].as_str().and_then(path_of)) {
                        ops.push(FileOp::Rename(a, b));
                    }
                }
                Some("delete") => ops.extend(c["uri"].as_str().and_then(path_of).map(FileOp::Delete)),
                _ => {
                    if let Some(p) = c.pointer("/textDocument/uri").and_then(Value::as_str).and_then(path_of) {
                        add(p, &c["edits"]);
                    }
                }
            }
        }
    } else if let Some(changes) = edit["changes"].as_object() {
        for (uri, list) in changes {
            if let Some(p) = path_of(uri) {
                add(p, list);
            }
        }
    }
    (files, ops)
}

/// How many changes in how many files, for the message after an edit.
pub fn summary(files: &[FileEdit]) -> (usize, usize) {
    (files.iter().map(|f| f.edits.len()).sum(), files.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn changes_and_document_changes_both_convert() {
        let a = json!({ "changes": { "file:///p/a.rs": [{ "range": { "start": { "line": 0, "character": 3 }, "end": { "line": 0, "character": 6 } }, "newText": "bar" }] } });
        let (files, ops) = convert(&a, true, &|_| None);
        assert_eq!(
            files,
            vec![FileEdit {
                path: "/p/a.rs".into(),
                edits: vec![TextEdit { start: Pos::new(0, 3), end: Pos::new(0, 6), text: "bar".into() }]
            }]
        );
        assert!(ops.is_empty());
        let b = json!({ "documentChanges": [
            { "textDocument": { "uri": "file:///p/b.rs", "version": 1 }, "edits": [{ "range": { "start": { "line": 0, "character": 2 }, "end": { "line": 0, "character": 4 } }, "newText": "x" }] },
            { "kind": "rename", "oldUri": "file:///p/old.rs", "newUri": "file:///p/new.rs" }
        ] });
        // UTF-16: the emoji is two units, so character 2 is char 1.
        let (files, ops) = convert(&b, false, &|_| Some("😀ab".into()));
        assert_eq!(files[0].edits[0].start, Pos::new(0, 1));
        assert_eq!(files[0].edits[0].end, Pos::new(0, 3));
        assert_eq!(ops, vec![FileOp::Rename("/p/old.rs".into(), "/p/new.rs".into())]);
    }
}
