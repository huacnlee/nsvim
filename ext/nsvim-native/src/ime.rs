//! Typing through the platform's text input, so an input method can compose (Chinese, Japanese,
//! Korean). The editor is a view that takes every key itself, and Windows turns the input method
//! off for a window whose focused view is not a text input. Where text is typed (Insert mode, the
//! classic keymap) the keyboard goes to a hidden input instead: it types and composes text, and
//! every key that does not type text is captured and handled as before.

use editor::editor::{KeyCode, KeyInput};
use nsh_core::keymap::Keystroke;

/// The text typed into the hidden input that the editor has not taken yet. The input is never
/// emptied while it has the keyboard (that would end a composition): each change sends what is
/// new after the text already sent, and nothing while an input method composes.
#[derive(Default)]
pub struct SinkText {
    sent: String,
    composing: bool,
    pending: String,
}

impl SinkText {
    /// The input's text changed to `text`: what was typed, empty while composing.
    pub fn changed(&mut self, text: &str) -> String {
        self.pending = text.to_owned();
        if self.composing { String::new() } else { self.take() }
    }

    /// A composition started, changed or ended: its end sends what it committed.
    pub fn composed(&mut self, composing: bool) -> String {
        self.composing = composing;
        if composing { String::new() } else { self.take() }
    }

    /// Whether an input method is composing: then its keys (Backspace, Enter, the arrows) are its.
    pub fn composing(&self) -> bool {
        self.composing
    }

    fn take(&mut self) -> String {
        let text = std::mem::take(&mut self.pending);
        let common = self.sent.char_indices().zip(text.chars()).find(|((_, a), b)| a != b).map(|((i, _), _)| i);
        let common = common.unwrap_or_else(|| self.sent.len().min(text.len()));
        // A change that only deletes types nothing: Backspace is a key the editor took.
        let typed = text[common..].to_owned();
        self.sent = text;
        typed
    }
}

/// Whether `k` types text into the hidden input: a character with no modifier but Shift.
pub fn types_text(k: &KeyInput) -> bool {
    matches!(k.code, KeyCode::Char(_)) && !(k.ctrl || k.alt || k.cmd)
}

/// The keys the hidden input leaves to the editor, each with the key the editor takes: every
/// named key and every shortcut, with any modifier. Plain characters (Shift alone too) are not
/// here: they type into the input, where the input method sees them.
pub fn editor_keys(apple: bool) -> Vec<(String, KeyInput)> {
    const NAMED: &[(&str, KeyCode)] = &[
        ("enter", KeyCode::Enter),
        ("escape", KeyCode::Esc),
        ("backspace", KeyCode::Backspace),
        ("delete", KeyCode::Delete),
        ("tab", KeyCode::Tab),
        ("left", KeyCode::Left),
        ("right", KeyCode::Right),
        ("up", KeyCode::Up),
        ("down", KeyCode::Down),
        ("home", KeyCode::Home),
        ("end", KeyCode::End),
        ("pageup", KeyCode::PageUp),
        ("pagedown", KeyCode::PageDown),
        ("space", KeyCode::Char(' ')),
    ];
    // (text, ctrl, alt, shift, cmd)
    const MODS: &[(&str, bool, bool, bool, bool)] = &[
        ("", false, false, false, false),
        ("shift+", false, false, true, false),
        ("ctrl+", true, false, false, false),
        ("alt+", false, true, false, false),
        ("ctrl+shift+", true, false, true, false),
        ("alt+shift+", false, true, true, false),
        ("ctrl+alt+", true, true, false, false),
        ("cmd+", false, false, false, true),
        ("cmd+shift+", false, false, true, true),
        ("cmd+alt+", false, true, false, true),
    ];
    let mut keys: Vec<(String, KeyInput)> = Vec::new();
    let mut seen = Vec::new();
    let mut add = |text: String, code: KeyCode, (ctrl, alt, shift, cmd): (bool, bool, bool, bool)| {
        if cmd && !apple {
            return;
        }
        if let Ok(k) = Keystroke::parse_with(&text, apple)
            && !seen.contains(&k)
        {
            seen.push(k);
            // Vim reads `ctrl`, classic bindings `cmd`: the platform's shortcut key is both.
            let cmd = if apple { cmd } else { ctrl };
            keys.push((text, KeyInput { code, ctrl, alt, shift, cmd }));
        }
    };
    for &(m, ctrl, alt, shift, cmd) in MODS {
        for &(n, code) in NAMED {
            // Space types text unless a modifier other than Shift is held.
            if n == "space" && !(ctrl || alt || cmd) {
                continue;
            }
            add(format!("{m}{n}"), code, (ctrl, alt, shift, cmd));
        }
        for i in 1..=12u8 {
            add(format!("{m}f{i}"), KeyCode::F(i), (ctrl, alt, shift, cmd));
        }
        if ctrl || alt || cmd {
            for c in "abcdefghijklmnopqrstuvwxyz0123456789`-=[]\\;',./".chars() {
                let code = KeyCode::Char(if shift { c.to_ascii_uppercase() } else { c });
                add(format!("{m}{c}"), code, (ctrl, alt, shift, cmd));
            }
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sent(events: &[Result<&str, bool>]) -> Vec<String> {
        let mut s = SinkText::default();
        events
            .iter()
            .map(|e| match e {
                Ok(t) => s.changed(t),
                Err(c) => s.composed(*c),
            })
            .filter(|t| !t.is_empty())
            .collect()
    }

    #[test]
    fn typing_sends_each_new_character() {
        assert_eq!(sent(&[Ok("a"), Ok("ab"), Ok("ab ")]), ["a", "b", " "]);
        assert_eq!(sent(&[Ok("ab"), Ok("a"), Ok("ac")]), ["ab", "c"]);
    }

    #[test]
    fn a_composition_sends_only_its_commit() {
        let ev = [Ok("x"), Err(true), Ok("xn"), Ok("xni"), Ok("x你"), Err(false), Ok("x你a")];
        assert_eq!(sent(&ev), ["x", "你", "a"]);
        // The commit is reported only with the composition's end.
        assert_eq!(sent(&[Err(true), Ok("ni"), Err(false)]), ["ni"]);
        assert_eq!(sent(&[Err(true), Ok("zhong"), Ok("中文"), Err(false)]), ["中文"]);
    }

    #[test]
    fn named_keys_and_shortcuts_stay_the_editors() {
        let keys = editor_keys(false);
        let get = |t: &str| keys.iter().find(|(k, _)| k == t).map(|(_, k)| *k);
        for t in ["escape", "enter", "backspace", "tab", "shift+tab", "ctrl+w", "ctrl+[", "ctrl+space", "f5"] {
            assert!(get(t).is_some(), "{t}");
        }
        assert_eq!(get("escape").unwrap().code, KeyCode::Esc);
        let w = get("ctrl+w").unwrap();
        assert!(w.code == KeyCode::Char('w') && w.ctrl && w.cmd, "Control is the shortcut key off Apple platforms");
        assert!(get("space").is_none() && get("a").is_none() && get("shift+a").is_none(), "they type text");
        assert!(editor_keys(true).iter().any(|(k, i)| k == "cmd+s" && i.cmd && !i.ctrl));
    }
}
