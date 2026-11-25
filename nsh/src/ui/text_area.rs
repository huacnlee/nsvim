//! The text area: a fixed set of row slots, one per visible row. Each slot reads its own row
//! of the frame, so a key that changes one line rebuilds one row. The cursor is a view drawn
//! over the rows at its cell.

use editor::syntax::highlight::Kind;
use nsh_core::prelude::*;

use crate::app::fx::{Mods, Phase};
use crate::model::{Caret, Row, Seg, Shape};
use crate::store::{Grid, Session};
use crate::ui::style::{CodeFont, code_token, color};

/// Lines one notch of a mouse wheel scrolls.
const WHEEL_LINES: f32 = 3.0;

/// The advance of one char of `font`. 100 chars average out rounding.
pub fn cell_width(font: &CodeFont) -> f32 {
    (measure_text(&format!("{}px {}", font.size, font.family), &"0".repeat(100)).0 / 100.0).max(1.0)
}

fn mods(m: nsh_core::keymap::Modifiers) -> Mods {
    Mods { shift: m.shift, ctrl: m.ctrl, alt: m.alt, cmd: m.cmd }
}

/// `keys` is the view that takes the keys. A click gives it the focus back.
pub fn text_area(s: Session, keys: ElementRef) -> impl IntoElement {
    let g = s.grid();
    let slots = computed(move || (0..s.frame.with(|f| f.rows.len())).collect::<Vec<usize>>());
    let gutter = computed(move || s.frame.with(|f| f.gutter));
    let cell_w = computed(move || s.font.with(cell_width));
    let size = stored((0.0_f32, 0.0_f32));
    // A new font changes the cells: fit the grid again.
    effect(move || {
        let w = cell_w.get();
        let (width, height) = size.with(|s| *s);
        if width > 0.0 {
            untracked(|| s.resize(width, height, w));
        }
    });
    view()
        .flex_1()
        .min_h(px(0.0))
        .overflow_hidden()
        .bg(move || color("background"))
        .test_id("text-area")
        .on_layout(move |b: &Bounds<Points>| {
            size.with(|s| *s = (b.size.width.0, b.size.height.0));
            s.resize(b.size.width.0, b.size.height.0, cell_w.get_untracked());
        })
        .on_press_down(move |e: &PressEvent| {
            keys.focus();
            s.drag(e.position.x.0, e.position.y.0, Phase::Down, e.clicks, mods(e.modifiers));
            s.drag(e.position.x.0, e.position.y.0, Phase::Up, e.clicks, mods(e.modifiers));
        })
        .on_wheel(move |e: &WheelEvent| {
            let (dx, dy) = match e.delta {
                WheelDelta::Lines(d) => (d.x * WHEEL_LINES * g.cell_w, d.y * WHEEL_LINES * s.grid().cell_h),
                WheelDelta::Points(d) => (d.x.0, d.y.0),
            };
            s.wheel(dx, dy, mods(e.modifiers));
        })
        .child(each(slots, |i: &usize| *i, move |i, _| slot(s, i.get_untracked(), gutter)))
        .child(when(move || s.frame.with(|f| f.caret.is_some()), move || caret(s, gutter)))
        .child(when(move || s.frame.with(|f| f.empty), empty_state))
}

/// Row slot `i`: rebuilt only when row `i` of the frame changes.
fn slot(s: Session, i: usize, gutter: Memo<usize>) -> impl IntoElement {
    let row = computed(move || s.frame.with(|f| f.rows.get(i).cloned()));
    dynamic(move || {
        let g = s.grid();
        let font = s.font.get();
        let digits = gutter.get().saturating_sub(4);
        match row.get() {
            Some(Row::Line { number, current, segs, cursorline }) => {
                line(g, &font, gutter.get(), digits, number, current, segs, cursorline)
            }
            Some(Row::Past) => past(g, &font, gutter.get()),
            None => view().into_element(),
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn line(
    g: Grid,
    font: &CodeFont,
    gutter: usize,
    digits: usize,
    number: String,
    current: bool,
    segs: Vec<Seg>,
    cursorline: bool,
) -> AnyElement {
    let number_token = if current { "foreground" } else { "text-faint" };
    let mut code = code_text(font, "");
    for seg in segs {
        code = code.child(span(font, seg));
    }
    view()
        .flex_row()
        .h(px(g.cell_h))
        .items_center()
        .bg(move || color(if cursorline { "surface-raised" } else { "background" }))
        .child(
            view().flex_row().w(px(gutter as f32 * g.cell_w)).flex_shrink_0().child(view().w(px(2.0 * g.cell_w))).child(
                view()
                    .flex_row()
                    .w(px(digits as f32 * g.cell_w))
                    .justify_end()
                    .child(code_text(font, number).text_color(move || color(number_token))),
            ),
        )
        .child(code)
        .into_element()
}

fn span(font: &CodeFont, seg: Seg) -> Text {
    let token = code_token(seg.kind);
    let mut t = code_text(font, seg.text).text_color(move || color(token));
    if seg.kind == Some(Kind::Comment) {
        t = t.italic();
    }
    if seg.selected {
        t = t.bg(move || color("selection"));
    }
    t
}

fn past(g: Grid, font: &CodeFont, gutter: usize) -> AnyElement {
    view()
        .flex_row()
        .h(px(g.cell_h))
        .items_center()
        .child(view().w(px(gutter as f32 * g.cell_w)))
        .child(code_text(font, "~").text_color(move || color("border")))
        .into_element()
}

/// Text in the code font, on one line, with its spaces kept.
pub fn code_text(font: &CodeFont, content: impl IntoReactive<SharedString>) -> Text {
    text(content).font_family(font.family).text_size(px(font.size)).number_of_lines(1).allow_font_scaling(false)
}

fn caret(s: Session, gutter: Memo<usize>) -> impl IntoElement {
    let grid = computed(move || s.grid());
    let at = computed(move || s.frame.with(|f| f.caret).unwrap_or(Caret { row: 0, col: 0, width: 1, shape: Shape::Block }));
    let x = move || px((gutter.get() + at.get().col) as f32 * grid.get().cell_w);
    let y = move || {
        let (c, g) = (at.get(), grid.get());
        let below = if c.shape == Shape::Underline { g.cell_h - 2.0 } else { 0.0 };
        px(c.row as f32 * g.cell_h + below)
    };
    let w = move || {
        let (c, g) = (at.get(), grid.get());
        px(if c.shape == Shape::Bar { 2.0 } else { c.width as f32 * g.cell_w })
    };
    let h = move || px(if at.get().shape == Shape::Underline { 2.0 } else { grid.get().cell_h });
    view()
        .absolute()
        .left(x)
        .top(y)
        .w(w)
        .h(h)
        .bg(move || color("primary").opacity(if at.get().shape == Shape::Block { 0.55 } else { 1.0 }))
        .test_id("caret")
}

fn empty_state() -> impl IntoElement {
    view().absolute().inset(px(0.0)).items_center().justify_center().child(text("editor").text_color(move || color("text-faint")))
}
