//! `textDocument/formatting` and `textDocument/rangeFormatting`: the request, whether the server
//! can answer it, and its reply turned into char edits.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::edits::TextEdit;
use super::{Event, FormatJob, Manager, Pending, to_wire, uri};

/// What a format request is waiting for.
pub struct FormatWait {
    pub version: u64,
    pub save: bool,
}

/// Whether a server's capability says yes: `true`, or an options object.
fn can(caps: &Value, key: &str) -> bool {
    match &caps[key] {
        Value::Bool(b) => *b,
        Value::Object(_) => true,
        _ => false,
    }
}

impl Manager {
    pub(super) fn format(&mut self, j: FormatJob) {
        let fail = |out: &super::Out, why: String| {
            out.send(Event::Formatted { path: j.path.clone(), version: j.version, edits: Vec::new(), save: j.save, error: Some(why) })
        };
        let none = || "no language server is running for this file".to_string();
        let Some(d) = self.docs.get(&j.path) else { return fail(&self.out, none()) };
        let si = d.server;
        let Some(Some(s)) = self.servers.get(si) else { return fail(&self.out, none()) };
        let key = if j.range.is_some() { "documentRangeFormattingProvider" } else { "documentFormattingProvider" };
        // Before `initialize` answers, the capabilities are not known: ask, and an error says no.
        if s.ready && !can(&s.caps, key) {
            let what = if j.range.is_some() { "a selection" } else { "documents" };
            return fail(&self.out, format!("the {} language server does not format {what}", s.name));
        }
        let options = json!({ "tabSize": j.tab_size, "insertSpaces": j.spaces });
        let doc = json!({ "uri": uri(&j.path) });
        let (method, params) = match j.range {
            Some((a, b)) => {
                let (ac, bc) = (to_wire(&d.text, a, s.utf32), to_wire(&d.text, b, s.utf32));
                let range = json!({ "start": { "line": a.line, "character": ac }, "end": { "line": b.line, "character": bc } });
                ("textDocument/rangeFormatting", json!({ "textDocument": doc, "range": range, "options": options }))
            }
            None => ("textDocument/formatting", json!({ "textDocument": doc, "options": options })),
        };
        let Some(Some(s)) = self.servers.get_mut(si) else { return };
        let id = s.request(method, params);
        self.pending.insert((si, id), (Pending::Format(FormatWait { version: j.version, save: j.save }), j.path));
    }

    /// The reply to a format request.
    pub(super) fn formatted(&mut self, si: usize, path: PathBuf, w: FormatWait, reply: &Value) {
        let utf32 = self.servers.get(si).and_then(|s| s.as_ref()).is_some_and(|s| s.utf32);
        let (edits, error) = match reply.get("error") {
            Some(e) => (Vec::new(), Some(format!("format: {}", e["message"].as_str().unwrap_or("the server refused")))),
            None => (reply_edits(&path, &reply["result"], utf32, &|p| self.text_of(p)), None),
        };
        self.out.send(Event::Formatted { path, version: w.version, edits, save: w.save, error });
    }
}

/// The `TextEdit[]` of a format reply as char edits. The buffer keeps `\n` line ends, so a
/// `\r\n` in the new text becomes `\n`.
pub fn reply_edits(path: &Path, result: &Value, utf32: bool, text_of: &dyn Fn(&Path) -> Option<String>) -> Vec<TextEdit> {
    let edit = json!({ "changes": { uri(path): result } });
    let (files, _) = super::edits::convert(&edit, utf32, text_of);
    let mut edits: Vec<TextEdit> = files.into_iter().flat_map(|f| f.edits).collect();
    for e in &mut edits {
        if e.text.contains('\r') {
            e.text = e.text.replace("\r\n", "\n").replace('\r', "\n");
        }
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor::buffer::Pos;

    fn edit(sl: u64, sc: u64, el: u64, ec: u64, text: &str) -> Value {
        json!({ "range": { "start": { "line": sl, "character": sc }, "end": { "line": el, "character": ec } }, "newText": text })
    }

    #[test]
    fn utf16_columns_become_chars() {
        // `é` is one UTF-16 unit and `😀` is two: character 4 is after the emoji, char 3.
        let text = "é😀 x\n";
        let r = json!([edit(0, 4, 0, 5, "")]);
        let edits = reply_edits(Path::new("/p/a.rs"), &r, false, &|_| Some(text.into()));
        assert_eq!(edits, vec![TextEdit { start: Pos::new(0, 3), end: Pos::new(0, 4), text: String::new() }]);
        let edits = reply_edits(Path::new("/p/a.rs"), &r, true, &|_| None);
        assert_eq!(edits[0].start, Pos::new(0, 4), "a utf-32 server counts chars");
    }

    #[test]
    fn crlf_in_new_text_becomes_lf() {
        let r = json!([edit(0, 0, 1, 0, "fn a() {\r\n}\r\n")]);
        let edits = reply_edits(Path::new("/p/a.rs"), &r, true, &|_| None);
        assert_eq!(edits[0].text, "fn a() {\n}\n");
    }

    #[test]
    fn a_null_reply_is_no_edits() {
        assert!(reply_edits(Path::new("/p/a.rs"), &Value::Null, true, &|_| None).is_empty());
    }

    #[test]
    fn capabilities_read_bools_and_objects() {
        let caps = json!({ "documentFormattingProvider": true, "documentRangeFormattingProvider": { "workDoneProgress": false } });
        assert!(can(&caps, "documentFormattingProvider"));
        assert!(can(&caps, "documentRangeFormattingProvider"));
        assert!(!can(&json!({ "documentFormattingProvider": false }), "documentFormattingProvider"));
        assert!(!can(&json!({}), "documentFormattingProvider"));
    }
}
