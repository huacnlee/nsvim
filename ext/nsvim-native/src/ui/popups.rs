//! The popups by the cursor: the completion list in Insert mode and a language server's hover
//! text. Both show over the text area, below the cursor in the top part of the area and above
//! it lower down.
//!
//! `host_events` lets an embedding host feed the editor as a language server or an agent does.

use nsh_core::prelude::*;

use crate::store::Session;
use crate::ui::style::{CodeFont, color};
use crate::ui::text_area::code_text;

/// The height of a row of the completion list.
const ITEM_H: f32 = 22.0;

/// Hover lines shown at most.
const HOVER_LINES: usize = 14;

/// The completion list as it shows.
#[derive(Clone, Debug, PartialEq)]
struct List {
    /// Chars of the word typed so far: the matched head of each label.
    typed: usize,
    /// Columns from the start of the word to the cursor.
    back: usize,
    /// Label and kind of each item shown.
    items: Vec<(String, &'static str)>,
    sel: usize,
}

/// Listens to the host events that drive the editor from an embedding host:
///
/// - `editor.lsp`: a language server's answer for the active file (`App::host_lsp`).
/// - `editor.request`: a control socket request, such as `propose` (`App::host_request`).
pub fn host_events(s: Session) {
    on_host_event("editor.lsp", move |v: &nsh_core::JsonValue| s.update(|a, _| a.host_lsp(v)));
    on_host_event("editor.request", move |v: &nsh_core::JsonValue| s.update(|a, tx| a.host_request(v, tx)));
}

/// The popup that shows, if any: the completion list, else the hover text.
pub fn popups(s: Session, gutter: Memo<usize>) -> impl IntoElement {
    let list = s.view(|a| {
        let c = a.completion.as_ref()?;
        let col = a.ed.cursor().col;
        Some(List {
            typed: c.start.col.abs_diff(col),
            back: col.saturating_sub(c.start.col),
            items: c.shown.iter().map(|&i| (c.items[i].label.clone(), c.items[i].kind)).collect(),
            sel: c.sel,
        })
    });
    let hover = s.view(|a| a.hover.clone());
    let at = computed(move || s.frame.with(|f| f.caret.map(|c| (c.row, c.col))));
    let place = move |back: usize, nudge: f32| {
        let g = s.grid();
        let (row, col) = at.get().unwrap_or((0, 0));
        let x = ((gutter.get() + col).saturating_sub(back)) as f32 * g.cell_w - nudge;
        let below = (row as f32) < g.rows as f32 * 0.6;
        (x.max(0.0), row, below)
    };
    let shows_list = move || at.get().is_some() && list.with(|l| l.is_some());
    let shows_hover = move || at.get().is_some() && list.with(|l| l.is_none()) && hover.with(|h| h.is_some());
    view()
        .absolute()
        .inset(px(0.0))
        .pointer_events(PointerEvents::BoxNone)
        .child(when(shows_list, move || {
            dynamic(move || {
                let Some(l) = list.get() else { return view().into_element() };
                let (x, row, below) = place(l.back, 8.0);
                anchored(s, x, row, below, completion(s, &s.font.get(), l)).into_element()
            })
        }))
        .child(when(shows_hover, move || {
            dynamic(move || {
                let Some(h) = hover.get() else { return view().into_element() };
                let (x, row, below) = place(0, 10.0);
                anchored(s, x, row, below, hover_box(s, &s.font.get(), &h)).into_element()
            })
        }))
}

/// `popup` at `x`, under row `row` when `below`, else over it.
fn anchored(s: Session, x: f32, row: usize, below: bool, popup: impl IntoElement) -> View {
    let g = s.grid();
    let v = view().absolute().left(px(x));
    let v = if below { v.top(px((row + 1) as f32 * g.cell_h)) } else { v.bottom(px((g.rows.max(row) - row) as f32 * g.cell_h)) };
    v.child(popup)
}

/// The look every popup shares: a raised panel with a border.
fn panel() -> View {
    view().rounded(px(6.0)).border(px(1.0)).border_color(move || color("border")).bg(move || color("muted")).shadow_md()
}

/// The token of a completion item's kind.
fn kind_token(kind: &str) -> &'static str {
    match kind {
        "Function" | "Method" => "code-function",
        "Struct" | "Enum" | "Class" | "Interface" | "TypeParam" => "code-type",
        "Keyword" => "code-keyword",
        "Constant" | "EnumMember" => "code-number",
        _ => "muted-foreground",
    }
}

fn completion(s: Session, font: &CodeFont, l: List) -> impl IntoElement {
    let cw = s.grid().cell_w;
    let small = CodeFont { size: (font.size - 1.0).max(9.0), ..font.clone() };
    let mut list = panel().min_w(px(36.0 * cw)).py(px(3.0)).test_id("completion");
    for (j, (label, kind)) in l.items.into_iter().enumerate() {
        let head: String = label.chars().take(l.typed).collect();
        let rest: String = label.chars().skip(l.typed).collect();
        let on = j == l.sel;
        let name = code_text(font, "")
            .child(code_text(font, head).font_weight(FontWeight::BOLD).text_color(move || color("primary")))
            .child(code_text(font, rest).text_color(move || color("foreground")));
        list = list.child(
            view()
                .flex_row()
                .h(px(ITEM_H))
                .px(px(8.0))
                .gap(px(8.0))
                .items_center()
                .bg(move || if on { color("selection") } else { color("selection").opacity(0.0) })
                .role(AccessibilityRole::Option)
                .on_press(move || s.update(|a, _| a.pick_completion(j)))
                .child(name)
                .child(view().flex_1())
                .child(code_text(&small, kind).pl(px(16.0)).text_color(move || color(kind_token(kind)))),
        );
    }
    list
}

fn hover_box(s: Session, font: &CodeFont, h: &str) -> impl IntoElement {
    let cw = s.grid().cell_w;
    let mut b = panel().max_w(px(80.0 * cw)).px(px(10.0)).py(px(8.0)).gap(px(2.0)).test_id("hover");
    for l in h.lines().filter(|l| !l.trim_start().starts_with("```")).take(HOVER_LINES) {
        b = b.child(
            text(l.to_string())
                .font_family(font.family)
                .text_size(px(font.size))
                .allow_font_scaling(false)
                .text_color(move || color("foreground")),
        );
    }
    b
}
