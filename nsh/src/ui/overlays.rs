//! The overlays over the editor: the leader key menu, the finder (files, project search,
//! buffers, commands, locations, code actions, keys) and the in-app context menu. Also the
//! platform's own context menu (`native_menu`).
//!
//! Keys reach the overlays through `App::key`. The views here only show the state and take
//! clicks.

#[path = "overlays_picker.rs"]
mod picker;
#[cfg(test)]
#[path = "overlays_tests.rs"]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;

use editor::editor::KeyInput;
use nsh_core::element::desktop::{MenuItem, popup_menu_at};
use nsh_core::prelude::*;

use crate::app::fx::{PopupMenu, Then};
use crate::app::keys::{LEADER, LEADER_AGENT, LEADER_GIT, LEADER_LSP};
use crate::app::menu::ITEMS;
use crate::app::{App, Overlay};
use crate::store::Session;
use crate::ui::style::color;

/// Which overlay shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shown {
    None,
    Leader,
    Pick,
    Menu,
}

/// The overlay that shows now, over the whole editor screen. `keys` is the view that takes
/// the keys. A press takes the focus off it, so each press here gives it back.
pub fn overlays(s: Session, keys: ElementRef) -> impl IntoElement {
    let shown = s.view(|a| match &a.overlay {
        Overlay::None => Shown::None,
        Overlay::Leader(_) => Shown::Leader,
        Overlay::Pick(_) => Shown::Pick,
        Overlay::Menu(_) => Shown::Menu,
    });
    dynamic(move || {
        let what = shown.get();
        untracked(move || match what {
            Shown::None => view().into_element(),
            Shown::Leader => layer(s, keys, leader(s, keys)).into_element(),
            Shown::Pick => layer(s, keys, picker::picker(s, keys)).into_element(),
            Shown::Menu => layer(s, keys, menu(s, keys)).into_element(),
        })
    })
}

/// A full-window layer: a scrim that closes the overlay on a press, with `content` over it.
fn layer(s: Session, keys: ElementRef, content: impl IntoElement) -> View {
    view().absolute().inset(px(0.0)).child(scrim(s, keys)).child(content)
}

/// A full-window press target that closes the overlay.
fn scrim(s: Session, keys: ElementRef) -> View {
    view().absolute().inset(px(0.0)).label("Close").test_id("overlay-scrim").on_press(move || {
        keys.focus();
        s.update(|a, _| a.overlay = Overlay::None);
    })
}

/// A box with a 1 pt border and its title in the top edge, as the design draws panels.
pub(crate) fn titled(title: impl IntoReactive<SharedString>, size: f32) -> View {
    view().rounded(px(6.0)).bg(move || color("background")).child(crate::ui::style::frame(6.0)).child(
        view()
            .absolute()
            .left(px(12.0))
            .top(px(-9.0))
            .z_index(1)
            .px(px(5.0))
            .bg(move || color("background"))
            .child(text(title).text_size(px(size)).number_of_lines(1).text_color(move || color("primary"))),
    )
}

/// What the leader menu shows: its title, its entries, and the space under it.
#[derive(Clone, Debug, PartialEq)]
struct LeaderPart {
    title: &'static str,
    items: &'static [(char, &'static str, bool)],
    bottom: f32,
}

fn leader_part(a: &App) -> LeaderPart {
    let group = match a.overlay {
        Overlay::Leader(g) => g,
        _ => None,
    };
    let (title, items) = match group {
        Some('g') => ("SPC g", LEADER_GIT),
        Some('l') => ("SPC l", LEADER_LSP),
        Some('a') => ("SPC a", LEADER_AGENT),
        _ => (if a.ed.vim.leader == ' ' { "SPC" } else { "\\" }, LEADER),
    };
    let bottom = if a.is_vim() || a.ed.msg.is_some() { 54.0 } else { 32.0 };
    LeaderPart { title, items, bottom }
}

/// The leader key menu: a grid of keys and what they do, at the bottom of the window.
fn leader(s: Session, keys: ElementRef) -> impl IntoElement {
    let part = s.view(leader_part);
    let size = 12.5;
    let grid = dynamic(move || {
        let items = part.with(|p| p.items);
        untracked(move || {
            let mut grid = view().flex_row().flex_wrap().w_full();
            for &(k, label, group) in items {
                grid = grid.child(
                    view()
                        .flex_row()
                        .w(px(250.0))
                        .h(px(22.0))
                        .px(px(8.0))
                        .gap(px(8.0))
                        .items_center()
                        .cursor_pointer()
                        .hover(|st| st.bg(move || color("surface-raised")))
                        .role(AccessibilityRole::Menuitem)
                        .label(label)
                        .test_id(&format!("leader-{k}"))
                        .on_press(move || {
                            keys.focus();
                            s.update(|a, tx| a.key(KeyInput::ch(k), tx));
                        })
                        .child(text(k.to_string()).text_size(px(size)).font_weight(FontWeight::BOLD).text_color(move || color("primary")))
                        .child(text("→").text_size(px(size)).text_color(move || color("text-faint")))
                        .child(text(label).text_size(px(size)).text_color(move || color(if group { "mode-visual" } else { "foreground" }))),
                );
            }
            grid.into_element()
        })
    });
    let panel = titled(move || SharedString::from(part.with(|p| p.title)), size)
        .w_full()
        .pt(px(14.0))
        .pb(px(10.0))
        .px(px(8.0))
        .shadow_lg()
        .role(AccessibilityRole::Menu)
        .test_id("leader")
        .child(grid);
    view().absolute().left(px(12.0)).right(px(12.0)).bottom(move || px(part.with(|p| p.bottom))).on_press(move || keys.focus()).child(panel)
}

/// What the in-app context menu shows.
#[derive(Clone, Debug, PartialEq)]
struct MenuPart {
    x: f32,
    y: f32,
    sel: usize,
    vim: bool,
    has_sel: bool,
}

fn menu_part(a: &App) -> MenuPart {
    let (x, y, sel) = match &a.overlay {
        Overlay::Menu(m) => (m.x, m.y, m.sel),
        _ => (0.0, 0.0, 0),
    };
    MenuPart { x, y, sel, vim: a.is_vim(), has_sel: a.selection_chars().is_some() }
}

/// The menu's height: rows of 24 points, separators of 9, and 8 of padding.
fn menu_height() -> f32 {
    8.0 + ITEMS.iter().map(|i| if i.0 == "-" { 9.0 } else { 24.0 }).sum::<f32>()
}

/// The in-app context menu at the pointer, kept inside the window.
fn menu(s: Session, keys: ElementRef) -> impl IntoElement {
    let part = s.view(menu_part);
    let mut list = view()
        .w(px(260.0))
        .py(px(4.0))
        .rounded(px(6.0))
        .bg(move || color("muted"))
        .border_1()
        .border_color(move || color("border"))
        .shadow_lg()
        .role(AccessibilityRole::Menu)
        .test_id("context-menu");
    for (i, &(id, title, vk, ck)) in ITEMS.iter().enumerate() {
        if id == "-" {
            list = list.child(view().h(px(1.0)).w_full().my(px(4.0)).bg(move || color("border")));
            continue;
        }
        let enabled = move || part.with(|p| p.has_sel) || !matches!(id, "cut" | "copy");
        let on = move || part.with(|p| p.sel) == i;
        list = list.child(
            view()
                .flex_row()
                .h(px(24.0))
                .px(px(12.0))
                .items_center()
                .style_when(on, |st| st.bg(move || color("surface-raised")))
                .hover(|st| st.bg(move || color("surface-raised")))
                .role(AccessibilityRole::Menuitem)
                .label(title)
                .accessibility_state(move || AccessibilityState { selected: on(), ..Default::default() })
                .disabled(move || !enabled())
                .on_press(move || {
                    keys.focus();
                    if enabled() {
                        s.update(|a, tx| a.menu_pick(id, tx));
                    }
                })
                .child(text(title).text_sm().flex_1().text_color(move || color(if enabled() { "foreground" } else { "text-faint" })))
                .child(text(move || if part.with(|p| p.vim) { vk } else { ck }).text_xs().text_color(move || color("text-faint"))),
        );
    }
    let h = menu_height();
    let x = move || {
        let win = window();
        px(part.with(|p| p.x).min(win.width.0 - 268.0).max(4.0))
    };
    let y = move || {
        let win = window();
        let y = part.with(|p| p.y);
        px(if y + h > win.height.0 { (y - h).max(4.0) } else { y })
    };
    view().absolute().left(x).top(y).on_press(move || keys.focus()).child(list)
}

/// Shows `menu` as the platform's own context menu at (`x`, `y`) in the window, then runs
/// `then` with the id of the picked item, or `None`.
pub fn native_menu(s: Session, menu: PopupMenu, x: f32, y: f32, then: Then<Option<String>>) {
    let picked: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let items = menu
        .items
        .into_iter()
        .map(|item| match item {
            None => MenuItem::separator(),
            Some(item) => {
                let picked = picked.clone();
                let id = item.id;
                MenuItem::new(item.title).disabled(!item.enabled).on_select(move || *picked.borrow_mut() = Some(id.clone()))
            }
        })
        .collect();
    let answer = popup_menu_at(items, point(px(x), px(y)));
    // The answer comes when the menu closes, after the picked item's `on_select`.
    spawn(async move {
        answer.await;
        let id = picked.borrow_mut().take();
        s.update(|a, tx| then(a, id, tx));
    });
}
