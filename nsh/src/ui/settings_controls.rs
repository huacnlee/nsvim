//! The styled controls of Settings and Setup, built on `nsh_base`. Each takes its state as a
//! closure, so it follows the app, and an action to run when pressed. The `nsh_base` parts
//! have no look of their own, so every part gets its style here, from theme tokens.

use nsh_base::{ButtonElement, SelectElement, SelectEntry, SelectItem, SwitchElement, button, select, switch};
use nsh_core::prelude::*;

use crate::app::App;
use crate::app::fx::Tx;
use crate::color::Color;
use crate::store::Session;
use crate::ui::icons::{Lucide, icon, src};
use crate::ui::style::{color, swatch};

/// A press that changes the app, then gives the focus back to the view that takes the keys.
pub fn act(s: Session, keys: ElementRef, f: impl Fn(&mut App, &mut Tx) + 'static) -> impl Fn() + 'static {
    move || {
        s.update(|a, tx| f(a, tx));
        keys.focus();
    }
}

/// One option of a group: filled with the accent while it is the current one. `focus` marks
/// the option the keyboard is on.
pub fn choice(
    id: &str,
    label: impl Into<SharedString>,
    on: impl Fn() -> bool + Copy + 'static,
    focus: impl Fn() -> bool + Copy + 'static,
    press: impl Fn() + 'static,
) -> ButtonElement {
    let label: SharedString = label.into();
    button()
        .selected(on)
        .accessibility_label(label.clone())
        .test_id(id)
        .on_press(press)
        .flex_row()
        .h(px(26.0))
        .px_3()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(move || {
            color(if on() {
                "primary"
            } else if focus() {
                "text-faint"
            } else {
                "border"
            })
        })
        .bg(move || color(if on() { "primary" } else { "background" }))
        .hover(|s| s.opacity(0.85))
        .active(|s| s.opacity(0.7))
        .child(text(label).text_xs().text_color(move || color(if on() { "primary-foreground" } else { "muted-foreground" })))
}

/// An on / off switch. Its label is what screen readers say and what tests select it by.
pub fn toggle(label: &str, on: impl Fn() -> bool + Copy + 'static, press: impl Fn() + 'static) -> SwitchElement {
    switch()
        .checked(on)
        .on_change(move |_: &bool| press())
        .accessibility_label(SharedString::from(label.to_string()))
        .test_id(label)
        .flex_row()
        .items_center()
        .gap_2()
        .track_style(|s| {
            s.w(px(34.0)).h(px(20.0)).p(px(2.0)).rounded_full().bg(move || color("border")).checked(|s| s.bg(move || color("mode-insert")))
        })
        .thumb_style(|s| {
            s.size(px(16.0))
                .rounded_full()
                .bg(move || color("muted-foreground"))
                .checked(|s| s.translate_x(px(14.0)).bg(move || color("primary-foreground")))
        })
        .child(text(move || if on() { "on" } else { "off" }).text_xs().text_color(move || color("muted-foreground")))
}

/// A number with a step down and a step up button around it.
pub fn stepper(key: &str, shown: impl Fn() -> String + 'static, step: impl Fn(f32) + Copy + 'static) -> View {
    let half = |sign: &'static str, name: &str, dir: f32| {
        button()
            .accessibility_label(SharedString::from(format!("{name} {key}")))
            .test_id(&format!("{key}{sign}"))
            .on_press(move || step(dir))
            .w(px(28.0))
            .h(px(24.0))
            .items_center()
            .justify_center()
            .hover(|s| s.bg(move || color("surface-raised")))
            .active(|s| s.opacity(0.7))
            .child(
                icon(src(if dir < 0.0 { Lucide::Minus } else { Lucide::Plus }))
                    .size(px(14.0))
                    .text_color(move || color("muted-foreground")),
            )
    };
    view()
        .flex_row()
        .h(px(26.0))
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(move || color("border"))
        .child(half("-", "Decrease", -1.0))
        .child(
            view()
                .min_w(px(64.0))
                .items_center()
                .child(text(shown).text_sm().text_color(move || color("foreground")).test_id(&format!("{key}=value"))),
        )
        .child(half("+", "Increase", 1.0))
}

/// A pick of one font from `fonts`, each item shown in its own font.
pub fn font_select(key: &str, fonts: &[&str], cur: impl Fn() -> String + Copy + 'static, pick: impl Fn(&str) + 'static) -> SelectElement {
    let items: Vec<SelectItem> = fonts.iter().map(|f| SelectItem::new(f.to_string(), f.to_string())).collect();
    select()
        .items(items)
        .value(move || Some(SharedString::from(cur())))
        .on_change(move |v: &Option<SharedString>| {
            if let Some(v) = v {
                pick(v.as_str());
            }
        })
        .accessibility_label(SharedString::from(key.to_string()))
        .test_id(key)
        .offset(px(4.0))
        .popup_style(|s| s.p_1().rounded_md().border_1().border_color(move || color("border")).bg(move || color("popover")).shadow_md())
        .item_class("rounded-md focus:bg-accent selected:font-medium")
        .render_item(|item: &SelectEntry| {
            view().px_2().py_1p5().child(
                text(item.title.clone())
                    .text_sm()
                    .font_family(crate::geometry::family(item.title.as_str()))
                    .text_color(move || color("popover-foreground")),
            )
        })
        .trigger(
            view()
                .flex_row()
                .h(px(30.0))
                .min_w(px(200.0))
                .px_3()
                .gap_2()
                .items_center()
                .rounded_md()
                .border_1()
                .border_color(move || color("border"))
                .bg(move || color("background"))
                .child(
                    text(cur)
                        .flex_1()
                        .text_sm()
                        .font_family(move || SharedString::from(crate::geometry::family(&cur())))
                        .text_color(move || color("foreground")),
                )
                .child(icon(src(Lucide::ChevronDown)).size(px(14.0)).text_color(move || color("muted-foreground"))),
        )
}

/// A row of colour chips. The colours are data (a theme's swatches), not roles.
pub fn chips(colors: Vec<Color>, w: f32, h: f32) -> impl IntoElement {
    let mut row = view().flex_row().rounded(px(3.0)).overflow_hidden().border_1().border_color(move || color("border"));
    for (i, c) in colors.into_iter().enumerate() {
        row = row.child(view().w(px(if i == 0 { w * 2.0 } else { w })).h(px(h)).bg(swatch(c)));
    }
    row
}

/// A box with a title over its top border.
pub fn titled(title: impl Into<SharedString>) -> View {
    let title: SharedString = title.into();
    view().rounded_md().bg(move || color("background")).child(crate::ui::style::frame(6.0)).child(
        view()
            .absolute()
            .left(px(12.0))
            .top(px(-9.0))
            .z_index(1)
            .px(px(5.0))
            .bg(move || color("background"))
            .child(text(title).text_xs().text_color(move || color("muted-foreground"))),
    )
}

/// A key cap: `SPC`, `Ctrl`, `gd`.
pub fn kbd(k: impl Into<SharedString>) -> impl IntoElement {
    view()
        .h(px(20.0))
        .min_w(px(20.0))
        .px(px(5.0))
        .items_center()
        .justify_center()
        .rounded(px(3.0))
        .bg(move || color("surface-raised"))
        .border_1()
        .border_color(move || color("border"))
        .child(text(k.into()).text_xs().text_color(move || color("foreground")))
}

/// A small filled label: a mode name or a modifier key.
pub fn pill(t: impl Into<SharedString>, fill: &'static str, ink: &'static str) -> impl IntoElement {
    view()
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(3.0))
        .bg(move || color(fill))
        .child(text(t.into()).text_xs().font_weight(FontWeight::BOLD).text_color(move || color(ink)))
}
