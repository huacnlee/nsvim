//! The parts of the app that the overlays show, as the TSX views get them. Each is a pure
//! function of the app, so that a listener hears it only when it changes.

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_core::prelude::*;

use crate::app::keys::{LEADER, LEADER_AGENT, LEADER_GIT, LEADER_LSP};
use crate::app::menu::ITEMS;
use crate::app::{App, Overlay, PickKind, Picker};
use crate::ui::style::code_token;

/// Lines the preview shows at most.
const PREVIEW_LINES: usize = 40;

/// Which overlay shows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum OverlayKind {
    /// No overlay.
    #[default]
    None,
    /// The leader key menu.
    Leader,
    /// The finder, with one of its pickers.
    Pick,
    /// The in-app context menu.
    Menu,
}

/// One key of the leader menu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct LeaderItem {
    /// The key to press.
    pub key: String,
    /// What it does.
    pub label: String,
    /// The key opens a group of more keys.
    pub group: bool,
}

/// The leader menu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct LeaderState {
    /// `SPC`, or the group: `SPC g`.
    pub title: String,
    /// The keys, in order.
    pub items: Vec<LeaderItem>,
    /// The space under the menu, in points: the status line and the command line.
    pub bottom: f64,
}

/// One item of the in-app context menu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MenuEntry {
    /// What `Overlays.menuPress` takes.
    pub id: String,
    /// The item's name.
    pub title: String,
    /// The key that does the same, in the keymap in use.
    pub hint: String,
    /// The item can run now.
    pub enabled: bool,
    /// A line between groups of items, with no title.
    pub separator: bool,
}

/// The in-app context menu.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct MenuState {
    /// Where the pointer opened it: points from the window's left edge.
    pub x: f64,
    /// Where the pointer opened it: points from the window's top edge.
    pub y: f64,
    /// The index of the highlighted item.
    pub sel: u32,
    /// The items and separators, in order.
    pub items: Vec<MenuEntry>,
}

/// The open picker's query line and what is around it.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PickHead {
    /// The picker's name: `Find Files`.
    pub title: String,
    /// What the user typed.
    pub query: String,
    /// The matches and the candidates: `12 / 300`.
    pub count: String,
    /// The list fills the panel. Commands, code actions and keys have no preview.
    pub wide: bool,
    /// What the list shows when it has no rows.
    pub empty: String,
}

/// A run of a row's name: matched chars, or chars between them.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct NameRun {
    /// The chars.
    pub text: String,
    /// The chars match the query.
    pub hit: bool,
}

/// One row of the open picker.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PickRow {
    /// The row's place in the whole list.
    pub index: u32,
    /// What a screen reader says: the folder and the name.
    pub label: String,
    /// The folder of a file, drawn dim before the name.
    pub dir: String,
    /// The name in runs of matched and other chars.
    pub name: Vec<NameRun>,
    /// Text after the name, drawn dim: a hit's line, a key's section.
    pub extra: String,
}

/// The rows of the open picker that show.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PickList {
    /// How many rows the picker lists in all.
    pub total: u32,
    /// The rows on screen, from the scroll position.
    pub rows: Vec<PickRow>,
}

/// A run of a preview line, in the colour of its kind of code.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct CodeRun {
    /// The chars.
    pub text: String,
    /// The theme token of its colour: `code-keyword`, `foreground`.
    pub token: String,
}

/// One line of the preview.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PickPreviewLine {
    /// The line number, from 1.
    pub number: u32,
    /// The line the picked row names.
    pub hit: bool,
    /// The line's text in runs of one colour.
    pub runs: Vec<CodeRun>,
}

/// The preview of the picked file.
#[derive(Debug, Clone, PartialEq, JsObject)]
pub struct PreviewState {
    /// The file's path in the project, or `Preview`.
    pub title: String,
    /// The code font's family list.
    pub font: String,
    /// The lines around the picked line, or from the file's start.
    pub lines: Vec<PickPreviewLine>,
}

pub(super) fn kind(a: &App) -> OverlayKind {
    match &a.overlay {
        Overlay::None => OverlayKind::None,
        Overlay::Leader(_) => OverlayKind::Leader,
        Overlay::Pick(_) => OverlayKind::Pick,
        Overlay::Menu(_) => OverlayKind::Menu,
    }
}

pub(super) fn leader(a: &App) -> LeaderState {
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
    let items = items.iter().map(|&(k, label, group)| LeaderItem { key: k.to_string(), label: label.into(), group }).collect();
    let bottom = if a.is_vim() || a.ed.msg.is_some() { 54.0 } else { 32.0 };
    LeaderState { title: title.into(), items, bottom }
}

/// Whether the context menu's item `id` can run now. Cut and copy need a selection.
pub(super) fn menu_enabled(a: &App, id: &str) -> bool {
    !matches!(id, "cut" | "copy") || a.selection_chars().is_some()
}

pub(super) fn menu(a: &App) -> MenuState {
    let Overlay::Menu(m) = &a.overlay else { return MenuState { x: 0.0, y: 0.0, sel: 0, items: Vec::new() } };
    let vim = a.is_vim();
    let items = ITEMS
        .iter()
        .map(|&(id, title, vk, ck)| MenuEntry {
            id: id.into(),
            title: title.into(),
            hint: (if vim { vk } else { ck }).into(),
            enabled: id != "-" && menu_enabled(a, id),
            separator: id == "-",
        })
        .collect();
    MenuState { x: m.x.into(), y: m.y.into(), sel: m.sel as u32, items }
}

fn wide(kind: PickKind) -> bool {
    matches!(kind, PickKind::Commands | PickKind::Actions | PickKind::Keys | PickKind::Projects)
}

pub(super) fn pick(a: &App) -> Option<&Picker> {
    match &a.overlay {
        Overlay::Pick(p) => Some(p),
        _ => None,
    }
}

pub(super) fn head(a: &App) -> PickHead {
    let Some(pk) = pick(a) else {
        return PickHead { title: String::new(), query: String::new(), count: String::new(), wide: true, empty: String::new() };
    };
    let grep = pk.kind == PickKind::Grep;
    let locs = pk.kind == PickKind::Locations;
    let count = if locs {
        a.locations.len().to_string()
    } else if grep {
        a.grep.hits.len().to_string()
    } else {
        format!("{} / {}", pk.total, if pk.kind == PickKind::Files { a.files.len() } else { pk.items.len() })
    };
    let empty = if grep && a.grep.running {
        "Searching".to_string()
    } else if let Some(e) = &a.grep.error {
        e.clone()
    } else if grep && pk.query.is_empty() {
        "Type to search the project".into()
    } else {
        "No matches".into()
    };
    let title = if locs { a.locations_title } else { pk.kind.title() };
    PickHead { title: title.into(), query: pk.query.clone(), count, wide: wide(pk.kind), empty }
}

/// The rows on screen: from the scroll position, as many as fit and one more.
pub(super) fn list(a: &App) -> PickList {
    let Some(pk) = pick(a) else { return PickList { total: 0, rows: Vec::new() } };
    let n = a.pick_rows();
    let top = pk.view_top(n);
    let rows = (top..n.min(top + pk.shown() + 1)).map(|i| row(a, pk, i)).collect();
    PickList { total: n as u32, rows }
}

/// Every row the open picker lists, not only the ones on screen.
#[cfg(test)]
pub(super) fn all_rows(a: &App) -> Vec<PickRow> {
    let Some(pk) = pick(a) else { return Vec::new() };
    (0..a.pick_rows()).map(|i| row(a, pk, i)).collect()
}

fn row(a: &App, pk: &Picker, i: usize) -> PickRow {
    let (dir, name, hits, extra) = match pk.kind {
        PickKind::Grep => {
            let h = &a.grep.hits[i];
            (String::new(), format!("{}:{}", h.path, h.line + 1), Vec::new(), h.text.trim().chars().take(160).collect())
        }
        PickKind::Locations => {
            let l = &a.locations[i];
            (String::new(), format!("{}:{}", a.rel(&l.path), l.pos.line + 1), Vec::new(), l.text.trim().chars().take(160).collect())
        }
        // A key row is the keys and what they do, then a tab and the section, drawn dim.
        PickKind::Keys => {
            let m = &pk.matches[i];
            let item = &pk.items[m.index];
            let (main, section) = item.split_once('\t').unwrap_or((item, ""));
            let n = main.chars().count();
            let hits = m.hits.iter().copied().filter(|&h| h < n).collect();
            (String::new(), main.to_string(), hits, section.to_string())
        }
        _ => {
            let m = &pk.matches[i];
            let full = if pk.kind == PickKind::Files { &a.files[m.index] } else { &pk.items[m.index] };
            let split = if pk.kind == PickKind::Files { full.rfind('/').map_or(0, |i| i + 1) } else { 0 };
            let dir = full[..split].to_string();
            let dir_chars = dir.chars().count();
            let hits = m.hits.iter().filter(|&&h| h >= dir_chars).map(|h| h - dir_chars).collect();
            (dir, full[split..].to_string(), hits, String::new())
        }
    };
    let label = format!("{dir}{name}");
    let name = hit_runs(&name, &hits).into_iter().map(|(text, hit)| NameRun { text, hit }).collect();
    PickRow { index: i as u32, label, dir, name, extra }
}

/// `name` in runs of chars, each with whether its chars are matched ones.
pub(super) fn hit_runs(name: &str, hits: &[usize]) -> Vec<(String, bool)> {
    let mut out: Vec<(String, bool)> = Vec::new();
    for (i, c) in name.chars().enumerate() {
        let hit = hits.contains(&i);
        match out.last_mut() {
            Some((run, h)) if *h == hit => run.push(c),
            _ => out.push((c.to_string(), hit)),
        }
    }
    out
}

/// The source the preview shows: its title, the first line shown, the line to mark, and the
/// text up to the last line shown (earlier lines make the colours right).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewSrc {
    pub title: String,
    pub path: Option<PathBuf>,
    pub first: usize,
    pub hit: Option<usize>,
    pub src: String,
}

pub(super) fn preview_src(a: &App) -> Option<PreviewSrc> {
    let pk = pick(a)?;
    if wide(pk.kind) {
        return None;
    }
    let picked = a.picked();
    let title = picked.as_ref().map(|(p, _)| a.rel(p)).unwrap_or_else(|| "Preview".into());
    let mut out = PreviewSrc { title, path: None, first: 0, hit: None, src: String::new() };
    if let (Some((path, at)), Some((ppath, src))) = (&picked, &pk.preview)
        && path == ppath
    {
        let first = at.map_or(0, |a| a.line.saturating_sub(6));
        let end = src.split('\n').take(first + PREVIEW_LINES).map(|l| l.len() + 1).sum::<usize>().min(src.len());
        out.path = Some(path.clone());
        out.first = first;
        out.hit = at.map(|a| a.line);
        out.src = src[..end].to_string();
    }
    Some(out)
}

/// The part for the preview. It colours the source again only when the source changed: the
/// part runs after every update, and colouring takes far longer than a key.
pub(super) fn preview() -> impl Fn(&App) -> PreviewState {
    let last: RefCell<Option<(Option<PreviewSrc>, Vec<PickPreviewLine>)>> = RefCell::new(None);
    move |a| {
        let src = preview_src(a);
        let mut last = last.borrow_mut();
        if last.as_ref().is_none_or(|(s, _)| *s != src) {
            let lines = src.as_ref().map(preview_lines).unwrap_or_default();
            *last = Some((src.clone(), lines));
        }
        let lines = last.as_ref().map(|(_, l)| l.clone()).unwrap_or_default();
        let title = src.map(|s| s.title).unwrap_or_default();
        PreviewState { title, font: a.font_family.to_string(), lines }
    }
}

/// The preview's lines, coloured as code.
pub(super) fn preview_lines(p: &PreviewSrc) -> Vec<PickPreviewLine> {
    let Some(path) = &p.path else { return Vec::new() };
    let buf = Buffer::from_str(&p.src);
    let lines: Vec<&str> = p.src.split('\n').collect();
    let count = lines.len().saturating_sub(p.first).min(PREVIEW_LINES);
    let runs = highlight::runs(&buf, Some(Path::new(path)), p.first, count);
    (0..count)
        .map(|j| {
            let li = p.first + j;
            let chars: Vec<char> = lines[li].chars().collect();
            let mut out = Vec::new();
            let mut push = |text: String, kind: Option<Kind>| out.push(CodeRun { text, token: code_token(kind).into() });
            let mut at = 0;
            for (r, k) in runs.get(j).map(Vec::as_slice).unwrap_or(&[]) {
                let (a, b) = (r.start.min(chars.len()), r.end.min(chars.len()));
                if a > at {
                    push(chars[at..a].iter().collect(), None);
                }
                if b > a.max(at) {
                    push(chars[a.max(at)..b].iter().collect(), Some(*k));
                    at = b;
                }
            }
            if at < chars.len() {
                push(chars[at..].iter().collect(), None);
            }
            PickPreviewLine { number: li as u32 + 1, hit: p.hit == Some(li), runs: out }
        })
        .collect()
}
