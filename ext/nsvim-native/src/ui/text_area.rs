//! The text area: a fixed set of row slots, one per visible row. Each slot reads its own row
//! of the frame, so a key that changes one line rebuilds one row. The cursor, the ruler and the
//! popups are views drawn over the rows.
//!
//! The mouse: a press places the cursor (two clicks select a word, three a line), a drag
//! selects, the wheel scrolls and the secondary button opens the context menu.

use editor::syntax::highlight::Kind;
use nsh_core::prelude::*;

use crate::app::fx::{Mods, Phase, PointerEvent};
use crate::model::{After, Caret, Level, Row, Seg, Shape, Sign};
use crate::store::{Grid, Session};
use crate::ui::style::{CodeFont, code_token, color};
use crate::ui::{popups, review_view};

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
    let slots = computed(move || (0..s.frame.with(|f| f.rows.len())).collect::<Vec<usize>>());
    let gutter = computed(move || s.frame.with(|f| f.gutter));
    let cell_w = computed(move || s.font.with(cell_width));
    let reviewing = s.view(|a| a.proposal_here().is_some());
    let size = stored((0.0_f32, 0.0_f32));
    // Where the area's top left is in the window, from the press that starts a drag. A drag
    // reports the pointer in the window.
    let origin = stored((0.0_f32, 0.0_f32));
    let dragging = stored(false);
    // A new font changes the cells: fit the grid again.
    effect(move || {
        let w = cell_w.get();
        let (width, height) = size.with(|s| *s);
        if width > 0.0 {
            untracked(|| s.resize(width, height, w));
        }
    });
    popups::host_events(s);
    let release = move |e: &ResponderEvent| {
        if dragging.with(|d| std::mem::replace(d, false)) {
            let (ox, oy) = origin.with(|o| *o);
            s.drag(e.page.x.0 - ox, e.page.y.0 - oy, Phase::Up, 1, mods(e.modifiers));
        }
    };
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
        // The area takes presses, but it is no Tab stop: the keys go to `keys`.
        .focusable(false)
        .on_press(|| {})
        .on_press_down(move |e: &PressEvent| {
            keys.focus();
            if reviewing.get_untracked() {
                return;
            }
            let (x, y) = (e.position.x.0, e.position.y.0);
            origin.with(|o| *o = (e.page_position.x.0 - x, e.page_position.y.0 - y));
            dragging.with(|d| *d = true);
            s.drag(x, y, Phase::Down, e.clicks.max(1), mods(e.modifiers));
        })
        // The mouse selects as it drags: the area takes the pointer's moves.
        .claim_touches(TouchClaim::on_start().ask(|r: &ClaimRequest| r.claim()))
        .keep_touches(true)
        .on_responder_move(move |e: &ResponderEvent| {
            if dragging.with(|d| *d) {
                let (ox, oy) = origin.with(|o| *o);
                s.drag(e.page.x.0 - ox, e.page.y.0 - oy, Phase::Move, 1, mods(e.modifiers));
            }
        })
        .on_responder_release(release)
        .on_responder_terminate(release)
        .on_wheel(move |e: &WheelEvent| {
            let g = s.grid();
            let (dx, dy) = match e.delta {
                WheelDelta::Lines(d) => (d.x * WHEEL_LINES * g.cell_w, d.y * WHEEL_LINES * g.cell_h),
                WheelDelta::Points(d) => (d.x.0, d.y.0),
            };
            s.wheel(dx, dy, mods(e.modifiers));
        })
        .on_context_menu(move |e: &ContextMenuEvent| {
            keys.focus();
            if reviewing.get_untracked() {
                return;
            }
            let p = PointerEvent { x: e.position.x.0, y: e.position.y.0, window_x: e.page_position.x.0, window_y: e.page_position.y.0 };
            s.update(|a, tx| a.context_menu(p, tx));
        })
        .child(when(reviewing, move || review_view::diff_view(s)).otherwise(move || rows(s, slots, gutter)))
        .child(when(move || s.frame.with(|f| f.empty), move || empty_state(s)))
}

/// The rows, the ruler, the cursor and the popups by it.
fn rows(s: Session, slots: Memo<Vec<usize>>, gutter: Memo<usize>) -> impl IntoElement {
    let ruler = computed(move || s.frame.with(|f| f.ruler));
    view()
        .flex_1()
        .child(each(slots, |i: &usize| *i, move |i, _| slot(s, i.get_untracked(), gutter)))
        .child(when(move || ruler.get().is_some(), move || ruler_line(s, ruler, gutter)))
        .child(when(move || s.frame.with(|f| f.caret.is_some()), move || caret(s, gutter)))
        .child(popups::popups(s, gutter))
}

/// Row slot `i`: rebuilt only when row `i` of the frame changes.
fn slot(s: Session, i: usize, gutter: Memo<usize>) -> impl IntoElement {
    let row = computed(move || s.frame.with(|f| f.rows.get(i).cloned()));
    let id = format!("row-{i}");
    dynamic(move || {
        let g = s.grid();
        let font = s.font.get();
        let el = match row.get() {
            Some(r @ Row::Line { .. }) => line(g, &font, gutter.get(), r),
            Some(Row::Past) => past(g, &font, gutter.get()),
            None => view().into_element(),
        };
        view().test_id(&id).child(el).into_element()
    })
}

/// The token of a gutter sign.
fn sign_token(sign: Sign) -> &'static str {
    match sign {
        Sign::Added => "mode-insert",
        Sign::Modified => "mode-command",
        Sign::Deleted | Sign::Diag(Level::Error) => "destructive",
        Sign::Outside => "mode-visual",
        Sign::Diag(Level::Warning) => "warning",
        Sign::Diag(Level::Info) => "muted-foreground",
    }
}

fn level_token(level: Level) -> &'static str {
    match level {
        Level::Error => "destructive",
        Level::Warning => "warning",
        Level::Info => "muted-foreground",
    }
}

fn line(g: Grid, font: &CodeFont, gutter: usize, row: Row) -> AnyElement {
    let Row::Line { number, current, segs, cursorline, sign, full, tint, after, carets } = row else {
        return view().into_element();
    };
    let digits = gutter.saturating_sub(5);
    let number_token = if current { "foreground" } else { "text-faint" };
    let mut code = code_text(font, "");
    for seg in segs {
        code = code.child(span(font, seg));
    }
    let mut number_text = code_text(font, number).text_color(move || color(number_token));
    if current {
        number_text = number_text.font_weight(FontWeight::BOLD);
    }
    let mut gut = view().flex_row().w(px(gutter as f32 * g.cell_w)).flex_shrink_0().child(
        view()
            .flex_row()
            .w(px(2.0 * g.cell_w))
            .justify_center()
            .child(code_text(font, sign.map_or("", Sign::glyph)).text_color(move || color(sign.map_or("text-faint", sign_token)))),
    );
    if digits > 0 {
        gut = gut.child(view().flex_row().w(px(digits as f32 * g.cell_w)).justify_end().child(number_text));
    }
    let mut code_box = view()
        .flex_row()
        .flex_1()
        .min_w(px(0.0))
        .h(px(g.cell_h))
        .items_center()
        .relative()
        .bg(move || if full { color("selection") } else { color("selection").opacity(0.0) })
        .child(code);
    if let Some(a) = after {
        let (glyph, token, msg) = match a {
            After::Diag(l, m) => ("■", level_token(l), m),
            After::Note(m) => ("◆", "mode-visual", m),
        };
        code_box = code_box.child(code_text(font, format!("    {glyph} {msg}")).flex_shrink(1.0).text_color(move || color(token)));
    }
    for c in carets {
        code_box = code_box.child(
            view()
                .absolute()
                .left(px(c as f32 * g.cell_w - 1.0))
                .top(px(2.0))
                .w(px(2.0))
                .h(px(g.cell_h - 4.0))
                .pointer_events(PointerEvents::None)
                .bg(move || color("primary").opacity(0.85))
                .test_id("extra-caret"),
        );
    }
    let fill = move || {
        if cursorline {
            color("surface-raised")
        } else if tint {
            color("mode-visual").opacity(0.12)
        } else {
            color("background")
        }
    };
    view().flex_row().h(px(g.cell_h)).items_center().bg(fill).child(gut).child(code_box).into_element()
}

fn span(font: &CodeFont, seg: Seg) -> Text {
    let token = if seg.ws { "border" } else { code_token(seg.kind) };
    let mut t = code_text(font, seg.text).text_color(move || color(token));
    if seg.kind == Some(Kind::Comment) {
        t = t.italic();
    }
    if seg.selected {
        t = t.bg(move || color("selection"));
    } else if seg.matched {
        t = t.bg(move || color("warning").opacity(0.28));
    }
    if let Some(level) = seg.diag {
        t = t.underline().text_decoration_wavy().text_decoration_color(move || color(level_token(level)));
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

fn ruler_line(s: Session, ruler: Memo<Option<usize>>, gutter: Memo<usize>) -> impl IntoElement {
    view()
        .absolute()
        .top(px(0.0))
        .bottom(px(0.0))
        .left(move || px((gutter.get() + ruler.get().unwrap_or(0)) as f32 * s.grid().cell_w))
        .w(px(1.0))
        .pointer_events(PointerEvents::None)
        .bg(move || color("border").opacity(0.7))
        .test_id("ruler")
}

fn caret(s: Session, gutter: Memo<usize>) -> impl IntoElement {
    let grid = computed(move || s.grid());
    let none = Caret { row: 0, col: 0, width: 1, shape: Shape::Block, ch: ' ', vim: true, visual: false };
    let at = computed(move || s.frame.with(|f| f.caret).unwrap_or(none));
    let x = move || {
        let (c, g) = (at.get(), grid.get());
        let bar = if c.shape == Shape::Bar { 1.0 } else { 0.0 };
        px((gutter.get() + c.col) as f32 * g.cell_w - bar)
    };
    let y = move || {
        let (c, g) = (at.get(), grid.get());
        let below = match c.shape {
            Shape::Underline => g.cell_h - 2.0,
            Shape::Bar => 2.0,
            Shape::Block => 0.0,
        };
        px(c.row as f32 * g.cell_h + below)
    };
    let w = move || {
        let (c, g) = (at.get(), grid.get());
        px(if c.shape == Shape::Bar { 2.0 } else { c.width as f32 * g.cell_w })
    };
    let h = move || {
        let (c, g) = (at.get(), grid.get());
        px(match c.shape {
            Shape::Underline => 2.0,
            Shape::Bar => g.cell_h - 4.0,
            Shape::Block => g.cell_h,
        })
    };
    let fill = move || {
        let c = at.get();
        color(match c.shape {
            Shape::Bar if c.vim => "mode-insert",
            Shape::Bar => "primary",
            Shape::Underline => "foreground",
            Shape::Block if c.visual => "mode-visual",
            Shape::Block => "foreground",
        })
    };
    // A block draws the char under it in the background's colour.
    let ink = move || color(if at.get().visual { "primary-foreground" } else { "background" });
    let under = move || {
        let c = at.get();
        if c.shape == Shape::Block { c.ch.to_string() } else { String::new() }
    };
    let font = s.font;
    view().absolute().left(x).top(y).w(w).h(h).items_center().pointer_events(PointerEvents::None).bg(fill).test_id("caret").child(dynamic(
        move || {
            let f = font.get();
            code_text(&f, move || SharedString::from(under())).text_color(ink).into_element()
        },
    ))
}

/// What the area shows with no file open: the keys to open one.
fn empty_state(s: Session) -> impl IntoElement {
    let hints = s.view(|a| {
        if a.is_vim() {
            let l = if a.ed.vim.leader == ' ' { "SPC" } else { "\\" };
            vec![
                ("find a file", format!("{l} f")),
                ("open a project", format!("{l} p")),
                ("search the project", format!("{l} /")),
                ("every key", l.to_string()),
                ("settings", ":Settings".into()),
            ]
        } else {
            let m = if cfg!(target_os = "macos") { "⌘" } else { "Ctrl" };
            vec![
                ("find a file", format!("{m} P")),
                ("open a project", format!("{m} O")),
                ("search the project", format!("{m} ⇧ F")),
                ("commands", format!("{m} ⇧ P")),
                ("settings", format!("{m} ,")),
            ]
        }
    });
    view().absolute().inset(px(0.0)).items_center().justify_center().bg(move || color("background")).child(dynamic(move || {
        let mut c = view()
            .gap(px(10.0))
            .child(text("no file open").font_weight(FontWeight::BOLD).text_size(px(15.0)).text_color(move || color("foreground")));
        for (what, keys) in hints.get() {
            c = c.child(
                view()
                    .flex_row()
                    .gap(px(16.0))
                    .child(text(what).w(px(180.0)).text_color(move || color("muted-foreground")))
                    .child(text(keys).text_color(move || color("primary"))),
            );
        }
        c.into_element()
    }))
}
