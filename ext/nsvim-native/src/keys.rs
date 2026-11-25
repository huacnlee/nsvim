//! Turns a key from NSH into a key of the core keymaps.

use editor::editor::{KeyCode, KeyInput};
use nsh_core::KeyEvent;

/// The core's key for `ev`, or `None` for a key the keymaps do not use (a modifier alone).
/// `apple` says whether Command is the shortcut key here (macOS). Elsewhere Control is, so it
/// also sets `cmd`, which the classic keymap reads for its shortcuts.
pub fn convert(ev: &KeyEvent, apple: bool) -> Option<KeyInput> {
    let m = ev.modifiers;
    let code = match ev.key.as_ref() {
        "Enter" => KeyCode::Enter,
        "Escape" => KeyCode::Esc,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Tab" => KeyCode::Tab,
        "ArrowLeft" => KeyCode::Left,
        "ArrowRight" => KeyCode::Right,
        "ArrowUp" => KeyCode::Up,
        "ArrowDown" => KeyCode::Down,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        " " => KeyCode::Char(' '),
        key => {
            let mut chars = key.chars();
            match (chars.next(), chars.next()) {
                // Some sources report a letter unshifted with Shift held.
                (Some(c), None) if m.shift && c.is_ascii_lowercase() => KeyCode::Char(c.to_ascii_uppercase()),
                (Some(c), None) => KeyCode::Char(c),
                (Some('F'), Some(_)) => KeyCode::F(key[1..].parse().ok()?),
                _ => return None,
            }
        }
    };
    let cmd = if apple { m.cmd } else { m.ctrl };
    Some(KeyInput { code, ctrl: m.ctrl, alt: m.alt, shift: m.shift, cmd })
}

#[cfg(test)]
mod tests {
    use nsh_core::element::SharedString;
    use nsh_core::keymap::Modifiers;

    use super::*;

    fn ev(key: &str, ctrl: bool) -> KeyEvent {
        let mut e = KeyEvent::default();
        e.key = SharedString::from(key.to_string());
        e.modifiers = Modifiers { ctrl, ..Modifiers::default() };
        e
    }

    #[test]
    fn named_keys_and_chars_convert() {
        assert_eq!(convert(&ev("Escape", false), false).map(|k| k.code), Some(KeyCode::Esc));
        assert_eq!(convert(&ev(":", false), false).map(|k| k.code), Some(KeyCode::Char(':')));
        assert_eq!(convert(&ev("F5", false), false).map(|k| k.code), Some(KeyCode::F(5)));
        assert_eq!(convert(&ev("Shift", false), false), None);
        let mut shifted = ev("a", false);
        shifted.modifiers.shift = true;
        assert_eq!(convert(&shifted, false).map(|k| k.code), Some(KeyCode::Char('A')));
    }

    #[test]
    fn control_is_the_shortcut_key_off_apple_platforms() {
        let k = convert(&ev("s", true), false).unwrap();
        assert!(k.ctrl && k.cmd);
        let k = convert(&ev("r", true), true).unwrap();
        assert!(k.ctrl && !k.cmd);
    }
}
