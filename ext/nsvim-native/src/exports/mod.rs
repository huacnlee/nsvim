//! What `App.tsx` imports from `nsvim-native`.
//!
//! - `Workspace`: the window's content. It makes the session, sets the theme tokens, starts
//!   the workers, and takes every key. The screens go inside it.
//! - `TextArea` and the other Rust components: the parts of the screen that Rust draws.
//! - Modules (`Shell`, ...): listeners of parts of the app, and the changes TSX asks for.

mod areas;
mod buffer_line;
mod command;
mod icon;
mod overlays;
mod settings;
mod setup;
mod status;
mod tree;
mod workspace;

use std::cell::RefCell;

use nsh_core::prelude::*;

use crate::store::Session;

pub use areas::*;
pub use buffer_line::*;
pub use command::*;
pub use icon::*;
pub use overlays::*;
pub use settings::*;
pub use setup::*;
pub use status::*;
pub use tree::*;
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
        .module::<Status>()
        .module::<Cmdline>()
        .module::<SettingsPage>()
        .module::<SetupFlow>();
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

pub(crate) fn set_current(s: Session, keys: ElementRef) {
    CURRENT.with(|c| *c.borrow_mut() = Some((s, keys)));
}
