//! The status line (mode, file, language, position) and the command line under it.

use nsh_core::prelude::*;

use crate::model::{Bottom, Tone};
use crate::store::Session;
use crate::ui::style::color;

pub fn status_line(s: Session) -> impl IntoElement {
    let status = computed(move || s.frame.with(|f| f.status.clone()));
    let tone = move || match status.with(|st| st.tone) {
        Tone::Normal => "primary",
        Tone::Insert => "mode-insert",
        Tone::Visual => "mode-visual",
        Tone::Command => "mode-command",
        Tone::Replace => "destructive",
    };
    view()
        .flex_row()
        .items_center()
        .min_h(px(24.0))
        .bg(move || color("muted"))
        .child(
            view().px_3().self_stretch().justify_center().bg(move || color(tone())).child(
                text(move || status.with(|st| st.mode))
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(move || color("primary-foreground"))
                    .test_id("mode"),
            ),
        )
        .child(
            text(move || status.with(|st| if st.dirty { format!("{} [+]", st.file) } else { st.file.clone() }))
                .text_xs()
                .px_3()
                .flex_1()
                .truncate()
                .text_color(move || color("foreground"))
                .test_id("file"),
        )
        .child(text(move || status.with(|st| st.lang)).text_xs().px_3().text_color(move || color("muted-foreground")))
        .child(
            text(move || status.with(|st| st.pos.clone()))
                .text_xs()
                .px_3()
                .text_color(move || color("muted-foreground"))
                .test_id("position"),
        )
}

pub fn command_line(s: Session) -> impl IntoElement {
    let bottom = computed(move || s.frame.with(|f| f.bottom.clone()));
    view().flex_row().items_center().min_h(px(24.0)).px_3().bg(move || color("background")).child(
        text(move || match bottom.get() {
            Bottom::Empty => String::new(),
            Bottom::Command { text, .. } => text,
            Bottom::Message(m) | Bottom::Error(m) => m,
        })
        .text_xs()
        .truncate()
        .text_color(move || color(if matches!(bottom.get(), Bottom::Error(_)) { "destructive" } else { "foreground" }))
        .test_id("command-line"),
    )
}
