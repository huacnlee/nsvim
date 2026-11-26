//! Turns a key from NSH into a key of the core keymaps.

use editor::editor::{KeyCode, KeyInput};
use nsh_core::KeyEvent;
use nsh_core::platform::Os;

/// macOS's quit key (`⌘ Q`): whether `ev` is it, and if so, whether the editor must answer
/// it. `Workspace` captures every key (`KeyCapture::All`), so NSH gives it the keys that macOS
/// keeps for its menu too. The Quit item then never sees `⌘ Q`, and the editor asks to quit
/// itself (`Some(true)`). A `⌘ Q` that the view did not capture goes on to the Quit item, which
/// asks once (`Some(false)`). A second ask here would count as the second close press and quit
/// over unsaved edits. No keymap gets `⌘ Q` on macOS.
pub fn mac_quit_key(ev: &KeyEvent, os: Os) -> Option<bool> {
    let m = ev.modifiers;
    let quit = os == Os::MacOs && m.cmd && !m.ctrl && !m.alt && !m.shift && ev.key.eq_ignore_ascii_case("q");
    quit.then_some(ev.default_prevented)
}

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

    fn cmd(key: &str, captured: bool) -> KeyEvent {
        let mut e = ev(key, false);
        e.modifiers.cmd = true;
        e.default_prevented = captured;
        e
    }

    #[test]
    fn cmd_q_on_macos_is_the_quit_key() {
        assert_eq!(mac_quit_key(&cmd("q", true), Os::MacOs), Some(true));
        assert_eq!(mac_quit_key(&cmd("Q", true), Os::MacOs), Some(true));
        // Left to the menu: the Quit item asks, and no keymap gets the key.
        assert_eq!(mac_quit_key(&cmd("q", false), Os::MacOs), Some(false));
        // Off a Mac, Command is not the quit key.
        assert_eq!(mac_quit_key(&cmd("q", true), Os::Linux), None);
        let mut shifted = cmd("q", true);
        shifted.modifiers.shift = true;
        assert_eq!(mac_quit_key(&shifted, Os::MacOs), None);
        assert_eq!(mac_quit_key(&ev("q", true), Os::MacOs), None);
    }

    #[test]
    fn control_is_the_shortcut_key_off_apple_platforms() {
        let k = convert(&ev("s", true), false).unwrap();
        assert!(k.ctrl && k.cmd);
        let k = convert(&ev("r", true), true).unwrap();
        assert!(k.ctrl && !k.cmd);
    }
}
