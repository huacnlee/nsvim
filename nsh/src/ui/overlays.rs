//! TODO(port): owned by one agent of the NSH port. A stub until then.

use nsh_core::prelude::*;

use crate::app::fx::{PopupMenu, Then};
use crate::store::Session;

pub fn overlays(s: Session) -> impl IntoElement {
    let _ = s;
    view()
}

/// Shows `menu` as the platform's own context menu at (`x`, `y`) in the window, then runs
/// `then` with the id of the picked item, or `None`. A stub until then: it picks nothing.
pub fn native_menu(s: Session, menu: PopupMenu, x: f32, y: f32, then: Then<Option<String>>) {
    let _ = (menu, x, y);
    s.update(|a, tx| then(a, None, tx));
}
