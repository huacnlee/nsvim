//! What the app asks of the window, and the input events it reads. The app's handlers take a
//! `Tx` and record effects on it. The NSH shell (`crate::store`) carries them out after each
//! update: it copies to the clipboard, reads it, shows a menu, sets the title, quits or wakes
//! the app later. This keeps the app's state code free of NSH, so its tests need no window.

use std::time::Duration;

use super::App;

/// A callback that runs with the app when an answer from the platform arrives.
pub type Then<T> = Box<dyn FnOnce(&mut App, T, &mut Tx)>;

/// The modifier keys held during an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

/// A mouse button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PointerButton {
    #[default]
    Primary,
    Middle,
    Secondary,
}

/// What the clipboard held.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clip(pub String);

impl Clip {
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// An item of a native context menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopupItem {
    pub id: String,
    pub title: String,
    pub enabled: bool,
}

impl PopupItem {
    pub fn new(id: &str, title: String) -> PopupItem {
        PopupItem { id: id.to_string(), title, enabled: true }
    }

    pub fn enabled(mut self, on: bool) -> PopupItem {
        self.enabled = on;
        self
    }
}

/// A native context menu: items, with `None` for a separator.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PopupMenu {
    pub items: Vec<Option<PopupItem>>,
}

impl PopupMenu {
    pub fn new() -> PopupMenu {
        PopupMenu::default()
    }

    pub fn item(mut self, item: PopupItem) -> PopupMenu {
        self.items.push(Some(item));
        self
    }

    pub fn separator(mut self) -> PopupMenu {
        self.items.push(None);
        self
    }
}

/// What a button of a [`Confirm`] dialog means: the platform places and draws it by its role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Default,
    Cancel,
    Destructive,
}

/// A native alert with a title, a description and buttons. Its answer is the id of the
/// button chosen, or None when it closed without one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Confirm {
    pub title: String,
    pub description: String,
    /// The buttons in order: id, label, role.
    pub buttons: Vec<(String, String, Role)>,
}

impl Confirm {
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Confirm {
        Confirm { title: title.into(), description: description.into(), buttons: Vec::new() }
    }

    pub fn button(mut self, id: &str, label: &str, role: Role) -> Confirm {
        self.buttons.push((id.to_string(), label.to_string(), role));
        self
    }
}

/// A thing the app asks the window to do.
pub enum Effect {
    Quit,
    SetTitle(String),
    Copy(String),
    ReadClipboard(Then<Clip>),
    /// A native menu at a point of the window. The callback gets the picked item's id.
    PopupMenu {
        menu: PopupMenu,
        x: f32,
        y: f32,
        then: Then<Option<String>>,
    },
    /// Run `App::poll` again after this delay.
    WakeAfter(Duration),
    /// Open this folder as a project, in a window of its own.
    OpenProject(std::path::PathBuf),
    /// Show the system's folder dialog, starting in this folder. The callback gets the folder
    /// picked, None when cancelled, or why the dialog could not show.
    ChooseFolder(std::path::PathBuf, Then<Result<Option<std::path::PathBuf>, String>>),
    /// Show the system's save dialog, starting in this folder. The callback gets the path
    /// chosen, None when cancelled, or why the dialog could not show.
    ChooseSavePath(std::path::PathBuf, Then<Result<Option<std::path::PathBuf>, String>>),
    /// Show a native alert. The callback gets the id of the button chosen, None when it
    /// closed without one, or why it could not show.
    Confirm(Confirm, Then<Result<Option<String>, String>>),
    /// Show a native alert with a text field that starts with this text. The callback gets
    /// the text when the user pressed the default button, None when cancelled, or why it
    /// could not show.
    Prompt(Confirm, String, Then<Result<Option<String>, String>>),
}

/// The effects of one update, and the state of the input event that caused it.
#[derive(Default)]
pub struct Tx {
    pub effects: Vec<Effect>,
    pub mods: Mods,
    pub button: PointerButton,
}

impl Tx {
    pub fn new() -> Tx {
        Tx::default()
    }

    /// A Tx for an input event with these modifiers and this button.
    pub fn with_input(mods: Mods, button: PointerButton) -> Tx {
        Tx { effects: Vec::new(), mods, button }
    }

    pub fn mods(&self) -> Mods {
        self.mods
    }

    pub fn button(&self) -> PointerButton {
        self.button
    }

    /// Asks for no easing. NSH changes in the next frame, so this does nothing.
    pub fn instant(&mut self) {}

    pub fn quit(&mut self) {
        self.effects.push(Effect::Quit);
    }

    pub fn set_title(&mut self, title: String) {
        self.effects.push(Effect::SetTitle(title));
    }

    pub fn copy(&mut self, text: String) {
        self.effects.push(Effect::Copy(text));
    }

    pub fn read_clipboard(&mut self, then: impl FnOnce(&mut App, Clip, &mut Tx) + 'static) {
        self.effects.push(Effect::ReadClipboard(Box::new(then)));
    }

    pub fn popup_menu(&mut self, menu: PopupMenu, at: (f32, f32), then: impl FnOnce(&mut App, Option<&str>, &mut Tx) + 'static) {
        let then: Then<Option<String>> = Box::new(move |app, picked: Option<String>, tx| then(app, picked.as_deref(), tx));
        self.effects.push(Effect::PopupMenu { menu, x: at.0, y: at.1, then });
    }

    pub fn open_project(&mut self, root: std::path::PathBuf) {
        self.effects.push(Effect::OpenProject(root));
    }

    pub fn choose_folder(
        &mut self,
        start: std::path::PathBuf,
        then: impl FnOnce(&mut App, Result<Option<std::path::PathBuf>, String>, &mut Tx) + 'static,
    ) {
        self.effects.push(Effect::ChooseFolder(start, Box::new(then)));
    }

    pub fn choose_save_path(
        &mut self,
        start: std::path::PathBuf,
        then: impl FnOnce(&mut App, Result<Option<std::path::PathBuf>, String>, &mut Tx) + 'static,
    ) {
        self.effects.push(Effect::ChooseSavePath(start, Box::new(then)));
    }

    pub fn confirm(&mut self, dialog: Confirm, then: impl FnOnce(&mut App, Result<Option<String>, String>, &mut Tx) + 'static) {
        self.effects.push(Effect::Confirm(dialog, Box::new(then)));
    }

    /// Asks for a line of text in a native dialog. `dialog` holds the buttons: the answer is the
    /// text when a `Role::Default` button closed it, else None.
    pub fn prompt(
        &mut self,
        dialog: Confirm,
        text: impl Into<String>,
        then: impl FnOnce(&mut App, Result<Option<String>, String>, &mut Tx) + 'static,
    ) {
        self.effects.push(Effect::Prompt(dialog, text.into(), Box::new(then)));
    }

    pub fn wake_after(&mut self, delay: Duration) {
        self.effects.push(Effect::WakeAfter(delay));
    }
}

/// A drag's phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Down,
    Move,
    Up,
}

/// A press and drag on the text area, in points from its top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub x: f32,
    pub y: f32,
    /// Where the press started.
    pub start_x: f32,
    pub start_y: f32,
    pub phase: Phase,
    /// 1 for a click, 2 for a double click, 3 for a triple click.
    pub clicks: u32,
    /// What the press selects by, kept for the drag that follows: 0 chars, 1 words, 2
    /// lines, 3 gutter lines.
    pub data: f32,
}

/// A wheel turn. Positive `dy` moves the content down (scrolls up).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WheelEvent {
    pub dx: f32,
    pub dy: f32,
    pub mods: Mods,
}

/// A secondary press: in the text area (`x`, `y`) and in the window (`window_x`, `window_y`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerEvent {
    pub x: f32,
    pub y: f32,
    pub window_x: f32,
    pub window_y: f32,
}

impl PointerEvent {
    /// Where a native menu opens.
    pub fn window(&self) -> (f32, f32) {
        (self.window_x, self.window_y)
    }
}

/// NSH shows a native context menu on every desktop. The in-app menu stays for tests.
pub const NATIVE_MENU: bool = true;
