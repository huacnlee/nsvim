//! TODO(port): owned by one agent of the NSH port. A stub until then.

use nsh_core::prelude::*;

use crate::store::Session;

pub fn buffer_line(s: Session) -> impl IntoElement {
    let _ = s;
    view()
}
