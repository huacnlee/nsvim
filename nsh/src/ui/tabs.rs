//! The buffer line: the project folder over the tree and a tab per open file. A click on a tab
//! shows its file, a click on its close button or a middle click closes it. A file with unsaved changes shows a dot in place of the close mark.

use nsh_core::prelude::*;

use crate::app::App;
use crate::store::Session;
use crate::ui::chrome::{UiFont, clear, ui_font, ui_text};
use crate::ui::icons::{Lucide, labeled_icon, src, src_with_stroke_width};
use crate::ui::style::color;

/// The height of the buffer line, in points.
pub const HEIGHT: f32 = 28.0;

/// The line width that fills `Lucide::Dot` to a solid dot, in its 24 units.
const DOT_STROKE: f32 = 9.0;

/// One tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabInfo {
    pub name: String,
    pub dirty: bool,
    /// The tab that shows.
    pub on: bool,
}

/// The tabs of the open files, as the buffer line shows them.
pub fn tabs_of(app: &App) -> Vec<TabInfo> {
    app.ed.docs.iter().enumerate().map(|(i, d)| TabInfo { name: d.buf.name(), dirty: d.buf.is_dirty(), on: i == app.ed.active }).collect()
}

/// Shows the file of tab `i`.
pub fn switch_to(app: &mut App, i: usize) {
    if i >= app.ed.docs.len() {
        return;
    }
    app.ed.active = i;
    app.ed.mode = app.ed.rest_mode();
    app.apply_doc_opts();
}

pub fn buffer_line(s: Session) -> impl IntoElement {
    let font = ui_font(s);
    let tree = s.view(|a| a.tree.show.then(|| (crate::app::tilde(&a.ed.root), a.tree.width)));
    let tabs = s.view(tabs_of);
    view()
        .flex_row()
        .items_center()
        .h(px(HEIGHT))
        .flex_shrink_0()
        .overflow_hidden()
        .bg(move || color("muted"))
        .role(AccessibilityRole::Tablist)
        .test_id("buffer-line")
        .child(when(
            move || tree.with(Option::is_some),
            move || {
                view()
                    .flex_row()
                    .flex_shrink_0()
                    .self_stretch()
                    .items_center()
                    .w(move || px(tree.with(|t| t.as_ref().map_or(0.0, |t| t.1))))
                    .px_3()
                    .border_r_1()
                    .border_color(move || color("border"))
                    .test_id("project")
                    .child(
                        ui_text(font, 0.0, move || tree.with(|t| t.as_ref().map(|t| t.0.clone()).unwrap_or_default()))
                            .truncate()
                            .text_color(move || color("muted-foreground")),
                    )
            },
        ))
        .child(each(
            move || (0..tabs.with(Vec::len)).collect::<Vec<usize>>(),
            |i: &usize| *i,
            move |i, _| tab(s, font, tabs, i.get_untracked()),
        ))
}

/// A tab's frame: a line along the bottom in the accent colour on the tab that shows. A clear
/// line of the same width at the top keeps the name in the middle.
fn accent_line(v: View, on: impl Fn() -> bool + Copy + 'static) -> View {
    v.pl(px(12.0))
        .border_t(px(2.0))
        .border_t_color(clear())
        .border_b(px(2.0))
        .border_b_color(move || if on() { color("primary") } else { clear() })
}

/// Tab `i`. It reads its own entry of `tabs`, so a change to one file redraws one tab.
fn tab(s: Session, font: Memo<UiFont>, tabs: Memo<Vec<TabInfo>>, i: usize) -> impl IntoElement {
    let info = computed(move || tabs.with(|t| t.get(i).cloned().unwrap_or(TabInfo { name: String::new(), dirty: false, on: false })));
    let on = move || info.with(|t| t.on);
    let dirty = move || info.with(|t| t.dirty);
    accent_line(view(), on)
        .flex_row()
        .flex_shrink_0()
        .self_stretch()
        .items_center()
        .bg(move || if on() { color("background") } else { clear() })
        .cursor(Cursor::Pointer)
        .role(AccessibilityRole::Tab)
        .label(move || SharedString::from(info.with(|t| t.name.clone())))
        .accessibility_state(move || AccessibilityState { selected: on(), ..AccessibilityState::default() })
        .test_id(&format!("tab-{i}"))
        .on_press(move || s.update(|a, _| switch_to(a, i)))
        .on_mouse_up(move |e: &MouseEvent| {
            if e.button == Some(MouseButton::Middle) {
                s.update(|a, _| a.close_tab(i));
            }
        })
        .child(
            ui_text(font, 0.0, move || info.with(|t| t.name.clone()))
                .text_color(move || color(if on() { "foreground" } else { "muted-foreground" })),
        )
        .child(
            view()
                .self_stretch()
                .justify_center()
                .px(px(10.0))
                .cursor(Cursor::Pointer)
                .hover(|st| st.bg(move || color("surface-raised")))
                .role(AccessibilityRole::Button)
                .label("Close")
                .test_id(&format!("tab-close-{i}"))
                .on_press(move || s.update(|a, _| a.close_tab(i)))
                .child({
                    // A thick line fills the dot, so it reads as a solid mark.
                    let dot = src_with_stroke_width(Lucide::Dot, DOT_STROKE);
                    let close = src(Lucide::X);
                    labeled_icon(
                        move || if dirty() { dot.clone() } else { close.clone() },
                        move || SharedString::from(if dirty() { "Unsaved changes" } else { "Close" }),
                    )
                    .size(move || px(font.get().size + 1.0))
                    .text_color(move || color(if dirty() { "mode-command" } else { "muted-foreground" }))
                    .test_id(&format!("tab-mark-{i}"))
                }),
        )
}
