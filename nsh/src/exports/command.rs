//! `Cmdline`: the part of the app that the command line shows (`ui/command-line.tsx`), and the
//! changes it asks for.
//!
//! `command_of` makes the part from the app, so tests can check it without a window.

use editor::editor::{Mode, VisualKind};
use nsh_core::prelude::*;

use crate::app::{App, Overlay};

/// How a command line message looks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum MessageTone {
    /// A message or the file's size: secondary text.
    #[default]
    Quiet,
    /// An error.
    Error,
    /// The mode, as `-- INSERT --`: bold.
    Mode,
}

/// What shows at the end of the command line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum KeyHint {
    /// Nothing.
    #[default]
    None,
    /// The leader menu is open: `<space>`.
    Leader,
    /// Keys typed so far of a command, as vim's `showcmd`. They are in `pending`.
    Pending,
    /// A button that opens the leader menu.
    Keys,
}

/// What the command line shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, JsObject)]
pub struct CommandState {
    /// The line shows: always in vim, in the classic keymap only for a command or a message.
    pub shows: bool,
    /// The user types a command (`:`) or a search (`/`, `?`).
    pub typing: bool,
    /// While typing: the prompt and the text before the caret.
    pub before: String,
    /// While typing: the text after the caret.
    pub after: String,
    /// When not typing: the message, the error or the mode.
    pub text: String,
    /// How `text` looks.
    pub tone: MessageTone,
    /// What shows at the end of the line.
    pub hint: KeyHint,
    /// The keys typed so far, when `hint` is `pending`.
    pub pending: String,
}

/// The command line's data.
pub fn command_of(app: &App) -> CommandState {
    let shows = app.is_vim() || app.ed.mode == Mode::Command || app.ed.msg.is_some();
    let mut info = CommandState { shows, ..CommandState::default() };
    if app.ed.mode == Mode::Command
        && let Some(c) = &app.ed.cmd
    {
        let before: String = c.text.chars().take(c.caret).collect();
        info.typing = true;
        info.before = format!("{}{before}", c.kind);
        info.after = c.text.chars().skip(c.caret).collect();
        return info;
    }
    let vim = app.is_vim();
    (info.text, info.tone) = if let Some(m) = &app.ed.msg {
        (m.text.clone(), if m.error { MessageTone::Error } else { MessageTone::Quiet })
    } else {
        match app.ed.mode {
            Mode::Insert if vim && editor::vim::replacing(&app.ed.vim) => ("-- REPLACE --".into(), MessageTone::Mode),
            Mode::Insert if vim => ("-- INSERT --".into(), MessageTone::Mode),
            Mode::Visual(VisualKind::Line) => ("-- VISUAL LINE --".into(), MessageTone::Mode),
            Mode::Visual(VisualKind::Block) => ("-- VISUAL BLOCK --".into(), MessageTone::Mode),
            Mode::Visual(_) => ("-- VISUAL --".into(), MessageTone::Mode),
            _ if app.ed.has_doc() => {
                let d = app.ed.doc();
                let name = d.buf.path.as_deref().map(|p| app.rel(p)).unwrap_or_else(|| "[No Name]".into());
                (format!("\"{name}\" {}L, {}", d.buf.line_count(), crate::app::human_bytes(d.buf.len_bytes())), MessageTone::Quiet)
            }
            _ => (String::new(), MessageTone::Quiet),
        }
    };
    let pending = editor::vim::showcmd(&app.ed.vim);
    info.hint = if matches!(app.overlay, Overlay::Leader(_)) {
        KeyHint::Leader
    } else if !pending.is_empty() {
        info.pending = pending;
        KeyHint::Pending
    } else if vim && matches!(app.overlay, Overlay::None) && app.ed.vim.leader == ' ' {
        KeyHint::Keys
    } else {
        KeyHint::None
    };
    info
}

/// The part of the app that the command line shows, and the changes it asks for.
#[module]
pub struct Cmdline;

#[module]
impl Cmdline {
    /// Calls `listener` with what the command line shows, now and after each change.
    pub fn on_change(#[args(command)] listener: Callback<CommandState>) -> Stop {
        crate::bridge::subscribe(command_of, listener)
    }

    /// Opens the leader menu, which lists the keys.
    pub fn show_keys() {
        crate::bridge::update(|a, _| a.overlay = Overlay::Leader(None));
    }
}
