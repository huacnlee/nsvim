//! `Overlays`: the state of the overlays over the editor, for `ui/overlays/*.tsx`. The leader
//! key menu, the finder with its pickers (files, project search, buffers, commands, locations,
//! code actions, keys) and the in-app context menu.
//!
//! Keys reach the overlays through `App::key`. The TSX views only show the state and send
//! clicks and the wheel here.

#[path = "overlays_parts.rs"]
mod parts;
#[cfg(test)]
#[path = "overlays_tests.rs"]
mod tests;

use editor::editor::KeyInput;
use nsh_core::prelude::*;

use crate::app::Overlay;
use crate::bridge;

pub use parts::{
    CodeRun, LeaderItem, LeaderState, MenuEntry, MenuState, NameRun, OverlayKind, PickHead, PickList, PickRow, PreviewLine, PreviewState,
};

/// The overlays over the editor: what shows, and the clicks on them.
#[module]
pub struct Overlays;

#[module]
impl Overlays {
    /// Calls `listener` with the overlay that shows, now and after each change.
    pub fn on_change(#[args(kind)] listener: Callback<OverlayKind>) -> Stop {
        bridge::subscribe(parts::kind, listener)
    }

    /// Calls `listener` with the leader menu's title and keys, now and after each change.
    pub fn on_leader(#[args(leader)] listener: Callback<LeaderState>) -> Stop {
        bridge::subscribe(parts::leader, listener)
    }

    /// Calls `listener` with the in-app context menu, now and after each change.
    pub fn on_menu(#[args(menu)] listener: Callback<MenuState>) -> Stop {
        bridge::subscribe(parts::menu, listener)
    }

    /// Calls `listener` with the open picker's title, query and count, now and after each
    /// change.
    pub fn on_pick_head(#[args(head)] listener: Callback<PickHead>) -> Stop {
        bridge::subscribe(parts::head, listener)
    }

    /// Calls `listener` with the rows of the open picker that show, now and after each change.
    /// The list sends only the rows on screen, from its scroll position.
    pub fn on_pick_rows(#[args(list)] listener: Callback<PickList>) -> Stop {
        bridge::subscribe(parts::list, listener)
    }

    /// Calls `listener` with the index of the picked row, now and after each change.
    pub fn on_pick_sel(#[args(index)] listener: Callback<u32>) -> Stop {
        bridge::subscribe(|a| parts::pick(a).map_or(0, |p| p.sel as u32), listener)
    }

    /// Calls `listener` with the preview of the picked file, now and after each change.
    pub fn on_preview(#[args(preview)] listener: Callback<PreviewState>) -> Stop {
        bridge::subscribe(parts::preview(), listener)
    }

    /// Closes the overlay that shows.
    pub fn close() {
        bridge::update(|a, _| a.overlay = Overlay::None);
    }

    /// Runs the leader menu's entry for `key`, as a press of that key does.
    pub fn leader_press(key: String) {
        let Some(k) = key.chars().next() else { return };
        bridge::update(move |a, tx| a.key(KeyInput::ch(k), tx));
    }

    /// Runs the context menu's item `id`. A disabled item does nothing.
    pub fn menu_press(id: String) {
        bridge::update(move |a, tx| {
            if parts::menu_enabled(a, &id) {
                a.menu_pick(&id, tx);
            }
        });
    }

    /// Picks the row `index` of the open picker and opens it, as Enter does.
    pub fn pick_press(index: u32) {
        bridge::update(move |a, tx| {
            if let Overlay::Pick(p) = &mut a.overlay {
                p.sel = index as usize;
            }
            a.pick_accept(tx);
        });
    }

    /// Tells the open picker how many rows fit in its list.
    pub fn set_pick_fit(rows: u32) {
        bridge::update(move |a, _| {
            let len = a.pick_rows();
            if let Overlay::Pick(p) = &mut a.overlay
                && p.fit != rows as usize
            {
                p.settle(len);
                p.fit = (rows as usize).max(1);
            }
        });
    }

    /// Scrolls the open picker's list by `dy` points (positive: toward the end). The picked row
    /// stays where it is.
    pub fn pick_wheel(dy: f64) {
        bridge::update(move |a, _| {
            let len = a.pick_rows();
            if let Overlay::Pick(p) = &mut a.overlay {
                p.wheel(dy as f32, len);
            }
        });
    }
}
