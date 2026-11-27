//! What `App.tsx` imports from `nsvim-native`.
//!
//! - `Workspace`: the window's content. It makes the session, sets the theme tokens, starts
//!   the workers, and takes every key. The screens go inside it.
//! - `TextArea` and the other Rust components: the parts of the screen that Rust draws.
//! - Modules (`Shell`, `MainMenu`, ...): listeners of parts of the app, and the changes TSX asks for.

mod areas;
mod buffer_line;
mod command;
mod find;
mod icon;
mod main_menu;
mod overlays;
mod settings;
mod setup;
mod status;
mod tree;
mod welcome;
mod workspace;

use std::cell::RefCell;

use nsh_core::prelude::*;

use crate::store::Session;

pub use areas::*;
pub use buffer_line::*;
pub use command::*;
pub use find::*;
pub use icon::*;
pub use main_menu::*;
pub use overlays::*;
pub use settings::*;
pub use setup::*;
pub use status::*;
pub use tree::*;
pub use welcome::*;
pub use workspace::*;

/// Registers every export in `registry`.
pub fn register(registry: &mut Registry) {
    registry
        .register::<Workspace>()
        .register::<TextArea>()
        .register::<Icon>()
        .module::<Shell>()
        .module::<Overlays>()
        .module::<FileTree>()
        .module::<Tabs>()
        .module::<MainMenu>()
        .module::<Status>()
        .module::<Cmdline>()
        .module::<Find>()
        .module::<SettingsPage>()
        .module::<SetupFlow>()
        .module::<Welcome>();
}

thread_local! {
    /// The session of this engine thread and the view that takes every key. `Workspace` sets
    /// them. The components inside it read them.
    static CURRENT: RefCell<Option<(Session, ElementRef)>> = const { RefCell::new(None) };
}

/// The session and the key view of the `Workspace` around the caller.
///
/// # Panics
///
/// When no `Workspace` was made on this thread.
pub fn current() -> (Session, ElementRef) {
    CURRENT.with(|c| *c.borrow()).expect("a component of nsvim-native inside <Workspace>")
}

/// Gives the keyboard back to the editor's key view, when a `Workspace` is on this thread.
/// A native dialog or a close press can leave the window with no focused view: then no key,
/// `:` included, reaches the editor until the user clicks the text.
pub(crate) fn refocus_keys() {
    if let Some((_, keys)) = CURRENT.with(|c| *c.borrow()) {
        keys.focus();
    }
}

pub(crate) fn set_current(s: Session, keys: ElementRef) {
    CURRENT.with(|c| *c.borrow_mut() = Some((s, keys)));
}
