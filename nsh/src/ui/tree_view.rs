//! The file tree: the project's folders and files, folders first. A click opens a folder or
//! closes it, or opens a file. The wheel scrolls it. Its edge drags to change its width. The
//! tree's keys go through `App::key`, which sends them to `App::tree_key` while it has focus.

use nsh_core::prelude::*;

use crate::app::App;
use crate::app::fx::{Mods, WheelEvent as AppWheel};
use crate::app::tree::ROW_H;
use crate::store::Session;
use crate::ui::chrome::{UiFont, clear, ui_font, ui_text};
use crate::ui::style::color;

/// Lines one notch of a mouse wheel scrolls.
const WHEEL_LINES: f32 = 3.0;

/// The narrowest and widest the tree can be, in points.
const MIN_W: f32 = 160.0;
const MAX_W: f32 = 640.0;

/// One row of the tree on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowInfo {
    pub path: String,
    pub name: String,
    pub depth: usize,
    pub dir: bool,
    pub open: bool,
    /// The file that shows in the text area.
    pub active: bool,
    /// The keyboard is on this row.
    pub picked: bool,
    /// The git letter: `M`, `U`, `A`, `D`, or `•` on a folder with changes inside.
    pub git: Option<char>,
}

impl RowInfo {
    /// The token of the name: the accent for folders, else the git status colour.
    pub fn name_token(&self) -> &'static str {
        if self.dir {
            return "primary";
        }
        match self.git {
            Some('U') | Some('A') => "mode-insert",
            Some('M') => "mode-command",
            Some('D') => "destructive",
            _ => "foreground",
        }
    }

    /// The token of the git letter.
    pub fn git_token(&self) -> &'static str {
        match self.git {
            Some('U') | Some('A') => "mode-insert",
            Some('D') => "destructive",
            _ => "mode-command",
        }
    }
}

/// The rows on screen: from the scroll position, as many as fit and one more.
pub fn rows_of(app: &App) -> Vec<RowInfo> {
    let rows = app.tree.index.rows(&app.tree.open);
    let active = if app.ed.has_doc() && !app.on_settings {
        app.ed.buf().path.as_ref().and_then(|p| p.strip_prefix(&app.ed.root).ok()).map(|r| r.to_string_lossy().replace('\\', "/"))
    } else {
        None
    };
    let start = app.tree_top(&rows);
    rows.iter()
        .skip(start)
        .take(app.tree_visible() + 1)
        .map(|r| RowInfo {
            path: r.path.clone(),
            name: r.name.clone(),
            depth: r.depth,
            dir: r.dir,
            open: r.open,
            active: active.as_deref() == Some(r.path.as_str()),
            picked: app.tree.focus && app.tree.sel.as_deref() == Some(r.path.as_str()),
            git: app.git_letter(r),
        })
        .collect()
}

fn mods(m: nsh_core::keymap::Modifiers) -> Mods {
    Mods { shift: m.shift, ctrl: m.ctrl, alt: m.alt, cmd: m.cmd }
}

/// `keys` is the view that takes the keys. A click on a row gives it the focus back.
pub fn tree(s: Session, keys: ElementRef) -> impl IntoElement {
    let font = ui_font(s);
    let rows = s.view(rows_of);
    let width = s.view(|a| a.tree.width);
    view()
        .w(move || px(width.get()))
        .flex_shrink_0()
        .self_stretch()
        .min_h(px(0.0))
        .overflow_hidden()
        .bg(move || color("muted"))
        .border_r_1()
        .border_color(move || color("border"))
        .role(AccessibilityRole::Tree)
        .label("Files")
        .test_id("tree")
        .on_wheel(move |e: &WheelEvent| {
            let dy = match e.delta {
                WheelDelta::Lines(d) => d.y * WHEEL_LINES * ROW_H,
                WheelDelta::Points(d) => d.y.0,
            };
            s.update(|a, _| a.tree_wheel(AppWheel { dx: 0.0, dy: -dy, mods: mods(e.modifiers) }));
        })
        .child(view().pt(px(4.0)).child(each(rows, |r: &RowInfo| r.path.clone(), move |r, _| row(s, keys, font, r))))
        .child(edge(s, width))
}

fn row(s: Session, keys: ElementRef, font: Memo<UiFont>, r: ReadSignal<RowInfo>) -> impl IntoElement {
    let info = computed(move || r.get());
    view()
        .flex_row()
        .items_center()
        .h(px(ROW_H))
        .pl(px(8.0))
        .pr_3()
        .bg(move || {
            info.with(|r| {
                if r.picked {
                    color("selection")
                } else if r.active {
                    color("selection").opacity(0.6)
                } else {
                    clear()
                }
            })
        })
        .hover(|st| st.bg(move || color("surface-raised")))
        .cursor(Cursor::Pointer)
        .role(AccessibilityRole::TreeItem)
        .label(move || SharedString::from(info.with(|r| r.name.clone())))
        .accessibility_level(move || info.with(|r| r.depth as u32 + 1))
        .accessibility_state(move || {
            info.with(|r| AccessibilityState {
                selected: r.picked || r.active,
                expanded: r.dir.then_some(r.open),
                ..AccessibilityState::default()
            })
        })
        .on_press(move || {
            let (path, dir) = info.with_untracked(|r| (r.path.clone(), r.dir));
            s.update(|a, _| {
                a.tree.focus = false;
                a.tree_press(&path, dir);
            });
            keys.focus();
        })
        .child(ui_text(font, 0.0, move || "│ ".repeat(info.with(|r| r.depth))).text_color(move || color("border")))
        .child(
            ui_text(font, 0.0, move || {
                info.with(|r| {
                    if !r.dir {
                        "  "
                    } else if r.open {
                        "▾ "
                    } else {
                        "▸ "
                    }
                })
            })
            .text_color(move || color("muted-foreground")),
        )
        .child(
            ui_text(font, 0.0, move || info.with(|r| r.name.clone()))
                .flex_shrink(1.0)
                .truncate()
                .text_color(move || color(info.with(RowInfo::name_token))),
        )
        .child(view().flex_1())
        .child(when(
            move || info.with(|r| r.git.is_some()),
            move || {
                ui_text(font, 0.0, move || info.with(|r| r.git.map(String::from).unwrap_or_default()))
                    .pl(px(8.0))
                    .text_color(move || color(info.with(RowInfo::git_token)))
            },
        ))
}

/// The tree's right edge: a drag changes the tree's width, and the release keeps it in the
/// config.
fn edge(s: Session, width: Memo<f32>) -> impl IntoElement {
    let start = stored(0.0_f32);
    let resize = move |dx: f32, done: bool| {
        s.update(|a, _| {
            if !a.cfg.mouse.enabled {
                return;
            }
            a.tree.width = (start.with(|w| *w) + dx).clamp(MIN_W, MAX_W);
            if done {
                a.cfg.ui.tree_width = a.tree.width;
            }
        });
    };
    view()
        .absolute()
        .top(px(0.0))
        .bottom(px(0.0))
        .left(move || px(width.get() - 3.0))
        .w(px(6.0))
        .cursor(Cursor::ColResize)
        .test_id("tree-edge")
        .claim_touches(TouchClaim::on_start())
        .on_responder_grant(move |_: &ResponderEvent| start.with(|w| *w = width.get_untracked()))
        .on_responder_move(move |e: &ResponderEvent| resize(e.gesture.delta.x.0, false))
        .on_responder_release(move |e: &ResponderEvent| resize(e.gesture.delta.x.0, true))
}
