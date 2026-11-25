//! The Settings page: sections on the left, the options of one section in the middle and, on a
//! wide window, a live preview and the config.toml lines on the right. A change applies at
//! once. `:w` (or Ctrl+S) writes config.toml through the writer worker.

use nsh_base::button;
use nsh_core::prelude::*;

use crate::app::settings::{self, Ctl, FONTS, Item, SECTIONS, Section};
use crate::store::Session;
use crate::theme;
use crate::ui::settings_controls::{act, chips, choice, font_select, stepper, toggle};
use crate::ui::settings_preview::preview;
use crate::ui::style::color;

/// Wider than this, the page shows the preview.
const PREVIEW_FROM: f32 = 1100.0;

pub fn settings(s: Session, keys: ElementRef) -> impl IntoElement {
    let section = s.view(|a| a.settings.as_ref().map_or(Section::Appearance, |st| st.section));
    let row = s.view(|a| a.settings.as_ref().map_or(0, |st| st.row));
    let list = s.view(|a| {
        let section = a.settings.as_ref().map_or(Section::Appearance, |st| st.section);
        settings::items(section, &a.cfg, a.omarchy.as_ref(), dark())
    });
    let width = signal(0.0_f32);
    view()
        .flex_row()
        .flex_1()
        .min_w(px(0.0))
        .bg(move || color("background"))
        .test_id("settings")
        .on_layout(move |b: &Bounds<Points>| width.set(b.size.width.0))
        .child(nav(s, keys, section))
        .child(scroll().flex_1().min_w(px(0.0)).child(view().px(px(24.0)).pt(px(18.0)).pb(px(24.0)).child(each(
            list,
            item_key,
            move |item, index| option(s, keys, item, index, row),
        ))))
        .child(when(move || width.get() > PREVIEW_FROM, move || preview(s)))
}

/// The sections, as tabs, and the path of config.toml under them.
fn nav(s: Session, keys: ElementRef, section: Memo<Section>) -> impl IntoElement {
    let mut tabs = view().pt(px(6.0)).role(AccessibilityRole::Tablist);
    for &(sec, label) in SECTIONS {
        let on = move || section.get() == sec;
        tabs = tabs.child(
            button()
                .role(AccessibilityRole::Tab)
                .selected(on)
                .accessibility_label(label)
                .test_id(&format!("section:{label}"))
                .on_press(act(s, keys, move |a, _| a.settings_section(sec)))
                .flex_row()
                .h(px(26.0))
                .items_center()
                .bg(move || color(if on() { "selection" } else { "muted" }))
                .hover(|st| st.bg(move || color("surface-raised")))
                .child(view().w(px(3.0)).h(px(16.0)).mr(px(12.0)).bg(move || color(if on() { "primary" } else { "muted" })))
                .child(text(label).text_sm().text_color(move || color("foreground"))),
        );
    }
    let path = crate::app::tilde(&crate::config::config_path());
    view()
        .w(px(230.0))
        .flex_shrink_0()
        .bg(move || color("muted"))
        .border_r_1()
        .border_color(move || color("border"))
        .child(tabs.flex_1())
        .child(
            view()
                .gap(px(6.0))
                .pt(px(12.0))
                .pb(px(14.0))
                .px(px(18.0))
                .border_t_1()
                .border_color(move || color("border"))
                .child(text(path).text_xs().text_color(move || color("muted-foreground")))
                .child(
                    button()
                        .accessibility_label("open as a buffer")
                        .on_press(act(s, keys, |a, _| a.open_path(crate::config::config_path(), None)))
                        .self_start()
                        .hover(|st| st.opacity(0.8))
                        .child(text("open as a buffer").text_xs().text_color(move || color("primary"))),
                ),
        )
}

fn item_key(item: &Item) -> String {
    match item {
        Item::Head(t) => format!("head:{t}"),
        Item::Row { key, .. } => (*key).to_string(),
    }
}

/// One item: a heading, or an option with its control. A row keeps its key, so its control
/// kind stays the same and only the values in it follow the app.
fn option(s: Session, keys: ElementRef, item: ReadSignal<Item>, index: ReadSignal<usize>, row: Memo<usize>) -> AnyElement {
    let (key, ctl) = match item.get_untracked() {
        Item::Head(t) => {
            let first = index.get_untracked() == 0;
            return text(t)
                .text_xs()
                .pt(px(if first { 0.0 } else { 18.0 }))
                .pb(px(6.0))
                .text_color(move || color("muted-foreground"))
                .into_element();
        }
        Item::Row { key, ctl, .. } => (key, ctl),
    };
    let desc = move || item.with(|i| if let Item::Row { desc, .. } = i { desc.clone() } else { String::new() });
    let label = view()
        .flex_1()
        .min_w(px(200.0))
        .gap(px(2.0))
        .child(text(key).text_sm().text_color(move || color("foreground")))
        .child(text(desc).text_xs().text_color(move || color("muted-foreground")));
    let kb = move || row.get() == index.get();
    let frame = view()
        .w_full()
        .py(px(4.0))
        .border_b_1()
        .border_color(move || color("border"))
        .bg(move || color(if kb() { "surface-raised" } else { "background" }))
        .test_id(&format!("row:{key}"));
    if let Ctl::Themes { opts, .. } = &ctl {
        return frame.gap(px(10.0)).py(px(10.0)).child(label).child(themes(s, keys, key, opts.clone(), item)).into_element();
    }
    let control: AnyElement = match ctl {
        Ctl::Seg { opts, .. } => {
            let mut group = view().flex_row().gap(px(4.0)).flex_wrap();
            for (v, l) in opts {
                let picked = v.clone();
                let on = computed(move || cur(item) == v);
                group = group.child(choice(
                    &format!("{key}={picked}"),
                    l,
                    move || on.get(),
                    || false,
                    act(s, keys, move |a, _| a.settings_pick(key, &picked)),
                ));
            }
            group.into_element()
        }
        Ctl::Font { .. } => font_select(
            key,
            FONTS,
            move || cur(item),
            move |v| {
                let v = v.to_string();
                s.update(|a, _| a.settings_pick(key, &v));
                keys.focus();
            },
        )
        .into_element(),
        Ctl::Step { .. } => stepper(
            key,
            move || shown(item),
            move |dir| {
                s.update(|a, _| a.settings_step(key, dir));
                keys.focus();
            },
        )
        .into_element(),
        Ctl::Toggle(_) => {
            let on = move || item.with(|i| matches!(i, Item::Row { ctl: Ctl::Toggle(true), .. }));
            toggle(key, on, act(s, keys, move |a, _| a.settings_toggle(key))).into_element()
        }
        Ctl::Info(_) => {
            text(move || shown(item)).text_sm().text_color(move || color("foreground")).test_id(&format!("{key}=value")).into_element()
        }
        Ctl::Chips(_) => dynamic(move || {
            let colors = item.with(|i| if let Item::Row { ctl: Ctl::Chips(c), .. } = i { c.clone() } else { Vec::new() });
            chips(colors, 11.0, 22.0)
        }),
        Ctl::Themes { .. } => unreachable!("themes return above"),
    };
    frame.flex_row().min_h(px(40.0)).gap(px(24.0)).items_center().flex_wrap().child(label).child(control).into_element()
}

/// The current value of a segmented, font or theme row.
fn cur(item: ReadSignal<Item>) -> String {
    item.with(|i| match i {
        Item::Row { ctl: Ctl::Seg { cur, .. } | Ctl::Font { cur, .. } | Ctl::Themes { cur, .. }, .. } => cur.clone(),
        _ => String::new(),
    })
}

/// The text of a number or information row.
fn shown(item: ReadSignal<Item>) -> String {
    item.with(|i| match i {
        Item::Row { ctl: Ctl::Step { shown } | Ctl::Info(shown), .. } => shown.clone(),
        _ => String::new(),
    })
}

/// The themes of a row as cards: the theme's swatches and its name.
fn themes(s: Session, keys: ElementRef, key: &'static str, opts: Vec<String>, item: ReadSignal<Item>) -> impl IntoElement {
    let mut grid = view().flex_row().flex_wrap().gap(px(8.0)).w_full();
    for id in opts {
        let name = theme::name(&id).to_string();
        let picked = id.clone();
        let on = computed(move || cur(item) == id);
        grid = grid.child(
            button()
                .selected(move || on.get())
                .accessibility_label(SharedString::from(name.clone()))
                .test_id(&format!("{key}={picked}"))
                .on_press(act(s, keys, {
                    let picked = picked.clone();
                    move |a, _| a.settings_pick(key, &picked)
                }))
                .flex_row()
                .w(px(196.0))
                .h(px(34.0))
                .px(px(8.0))
                .gap(px(10.0))
                .items_center()
                .rounded_md()
                .border(move || px(if on.get() { 2.0 } else { 1.0 }))
                .border_color(move || color(if on.get() { "primary" } else { "border" }))
                .hover(|st| st.opacity(0.85))
                .child(chips(theme::swatches(&picked).to_vec(), 7.0, 18.0))
                .child(text(name).text_sm().text_color(move || color(if on.get() { "foreground" } else { "muted-foreground" }))),
        );
    }
    grid
}
