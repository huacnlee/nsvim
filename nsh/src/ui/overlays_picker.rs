//! The finder overlay: the query line, the ranked rows with their matched chars, and a preview
//! of the picked file. Files, project search, buffers, commands, locations, code actions and
//! the key list all share it.

use std::path::Path;

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_core::prelude::*;

use super::titled;
use crate::app::{App, Overlay, PickKind, Picker};
use crate::store::Session;
use crate::ui::style::{code_token, color};

/// One row's height, in points.
const ROW_H: f32 = 22.0;
/// The text size of rows and the query.
const SIZE: f32 = 12.5;
/// Lines the preview shows at most.
const PREVIEW_LINES: usize = 40;

/// The query line and what is around it.
#[derive(Clone, Debug, PartialEq)]
struct Head {
    title: &'static str,
    query: String,
    count: String,
    /// The list fills the panel: no preview for commands, code actions and keys.
    wide: bool,
    /// What the list shows when it has no rows.
    empty: String,
}

/// One row: the directory, the name, the matched char indices of the name, and any text
/// after it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RowText {
    pub dir: String,
    pub name: String,
    pub hits: Vec<usize>,
    pub extra: String,
}

/// The source the preview shows: its title, the first line shown, the line to mark, and the
/// text up to the last line shown (earlier lines make the colours right).
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewSrc {
    title: String,
    path: Option<std::path::PathBuf>,
    first: usize,
    hit: Option<usize>,
    src: String,
}

/// A preview line: its number, its runs of text by kind, and whether it is the line to mark.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreviewLine {
    pub number: usize,
    pub runs: Vec<(String, Option<Kind>)>,
    pub hit: bool,
}

fn wide(kind: PickKind) -> bool {
    matches!(kind, PickKind::Commands | PickKind::Actions | PickKind::Keys)
}

fn pick(a: &App) -> Option<&Picker> {
    match &a.overlay {
        Overlay::Pick(p) => Some(p),
        _ => None,
    }
}

fn head(a: &App) -> Head {
    let Some(pk) = pick(a) else { return Head { title: "", query: String::new(), count: String::new(), wide: true, empty: String::new() } };
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
        "searching…".to_string()
    } else if let Some(e) = &a.grep.error {
        e.clone()
    } else if grep && pk.query.is_empty() {
        "type to search the project".into()
    } else {
        "no matches".into()
    };
    let title = if locs { a.locations_title } else { pk.kind.title() };
    Head { title, query: pk.query.clone(), count, wide: wide(pk.kind), empty }
}

/// Every row the picker shows now.
pub(super) fn rows(a: &App) -> Vec<RowText> {
    let Some(pk) = pick(a) else { return Vec::new() };
    let n = pk.len(if pk.kind == PickKind::Locations { a.locations.len() } else { a.grep.hits.len() });
    (0..n).map(|i| row_text(a, pk, i)).collect()
}

fn row_text(a: &App, pk: &Picker, i: usize) -> RowText {
    match pk.kind {
        PickKind::Grep => {
            let h = &a.grep.hits[i];
            let name = format!("{}:{}", h.path, h.line + 1);
            RowText { name, extra: h.text.trim().chars().take(160).collect(), ..RowText::default() }
        }
        PickKind::Locations => {
            let l = &a.locations[i];
            let name = format!("{}:{}", a.rel(&l.path), l.pos.line + 1);
            RowText { name, extra: l.text.trim().chars().take(160).collect(), ..RowText::default() }
        }
        _ => {
            let m = &pk.matches[i];
            // A key row is the keys and what they do, then a tab and the section, drawn dim.
            if pk.kind == PickKind::Keys {
                let item = &pk.items[m.index];
                let (main, section) = item.split_once('\t').unwrap_or((item, ""));
                let n = main.chars().count();
                let hits = m.hits.iter().copied().filter(|&h| h < n).collect();
                return RowText { dir: String::new(), name: main.to_string(), hits, extra: section.to_string() };
            }
            let full = if pk.kind == PickKind::Files { &a.files[m.index] } else { &pk.items[m.index] };
            let split = if pk.kind == PickKind::Files { full.rfind('/').map_or(0, |i| i + 1) } else { 0 };
            let dir = full[..split].to_string();
            let dir_chars = dir.chars().count();
            let hits = m.hits.iter().filter(|&&h| h >= dir_chars).map(|h| h - dir_chars).collect();
            RowText { dir, name: full[split..].to_string(), hits, extra: String::new() }
        }
    }
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

/// The preview's lines, coloured as code.
pub(super) fn preview_lines(p: &PreviewSrc) -> Vec<PreviewLine> {
    let Some(path) = &p.path else { return Vec::new() };
    let buf = Buffer::from_str(&p.src);
    let lines: Vec<&str> = p.src.split('\n').collect();
    let count = lines.len().saturating_sub(p.first).min(PREVIEW_LINES);
    let runs = highlight::runs(&buf, Some(Path::new(path)), p.first, count);
    (0..count)
        .map(|j| {
            let li = p.first + j;
            let line = lines[li];
            let chars: Vec<char> = line.chars().collect();
            let mut out = Vec::new();
            let mut at = 0;
            for (r, k) in runs.get(j).map(Vec::as_slice).unwrap_or(&[]) {
                let (a, b) = (r.start.min(chars.len()), r.end.min(chars.len()));
                if a > at {
                    out.push((chars[at..a].iter().collect(), None));
                }
                if b > a.max(at) {
                    out.push((chars[a.max(at)..b].iter().collect(), Some(*k)));
                    at = b;
                }
            }
            if at < chars.len() {
                out.push((chars[at..].iter().collect(), None));
            }
            PreviewLine { number: li + 1, runs: out, hit: p.hit == Some(li) }
        })
        .collect()
}

/// The finder panel, near the top of the window.
pub fn picker(s: Session, keys: ElementRef) -> impl IntoElement {
    let head = s.view(head);
    let width = move || {
        let w = window().width.0;
        (w - 32.0).clamp(320.0, 1040.0)
    };
    let height = move || 560.0f32.min(window().height.0 - 100.0).max(200.0);
    let left = move || px(((window().width.0 - width()) / 2.0).max(16.0));
    let top = move || px(80.0f32.min(window().height.0 * 0.1 + 20.0));
    let list_w = move || px(if head.with(|h| h.wide) { width() - 20.0 } else { (width() * 0.42).max(300.0) });
    let input = titled(move || SharedString::from(head.with(|h| h.title)), SIZE).h(px(40.0)).w_full().child(
        view()
            .flex_row()
            .flex_1()
            .px(px(8.0))
            .items_center()
            .test_id("picker-input")
            .child(text(">").text_size(px(13.0)).w(px(18.0)).text_color(move || color("primary")))
            .child(text(move || head.with(|h| h.query.clone())).text_size(px(13.0)).test_id("picker-query"))
            .child(view().w(px(1.5)).h(px(16.0)).bg(move || color("primary")).test_id("picker-caret"))
            .child(
                text(move || if head.with(|h| h.query.is_empty()) { "  type to filter" } else { "" })
                    .text_size(px(13.0))
                    .text_color(move || color("text-faint")),
            )
            .child(view().flex_1())
            .child(
                text(move || head.with(|h| h.count.clone()))
                    .text_size(px(SIZE))
                    .text_color(move || color("muted-foreground"))
                    .test_id("picker-count"),
            ),
    );
    let results = titled("Results", SIZE).w_full().flex_1().min_h(px(0.0)).child(list(s, keys, head));
    let column = view().w(list_w).h_full().gap(px(14.0)).child(input).child(results);
    let panel = view()
        .flex_row()
        .w(move || px(width()))
        .h(move || px(height()))
        .gap(px(14.0))
        .p(px(10.0))
        .rounded(px(8.0))
        .bg(move || color("background"))
        .border_1()
        .border_color(move || color("border"))
        .shadow_xl()
        .role(AccessibilityRole::Dialog)
        .test_id("picker")
        .on_press(move || keys.focus())
        .child(column)
        .child(when(move || !head.with(|h| h.wide), move || preview(s)));
    view().absolute().left(left).top(top).child(panel)
}

/// The rows, in a virtual list. The list follows the picked row.
fn list(s: Session, keys: ElementRef, head: Memo<Head>) -> impl IntoElement {
    let rows = s.view(rows);
    let sel = s.view(|a| pick(a).map_or(0, |p| p.sel));
    let list = element_ref();
    // The first row on screen and how many rows fit. The list reports the rows it shows
    // after a scroll. Until it does, the first row is the one this list scrolled to last.
    let top = stored(0_usize);
    let fit = stored(1_usize);
    effect(move || {
        let i = sel.get();
        if i >= rows.with(Vec::len) {
            return;
        }
        let (first, n) = (top.with(|t| *t), fit.with(|f| *f));
        if i < first {
            top.with(|t| *t = i);
            list.scroll_to_index(i as u32, percentage(0.0), px(0.0), false);
        } else if i >= first + n {
            top.with(|t| *t = i + 1 - n);
            list.scroll_to_index(i as u32, percentage(1.0), px(0.0), false);
        }
    });
    let body = virtual_list(
        move || rows.with(Vec::len),
        move |at| {
            let row = computed(move || rows.with(|r| r.get(at.index()).cloned().unwrap_or_default()));
            let on = computed(move || sel.get() == at.index());
            view()
                .flex_row()
                .h(px(ROW_H))
                .px(px(8.0))
                .items_center()
                .cursor_pointer()
                .style_when(on, |st| st.bg(move || color("surface-raised")))
                .hover(|st| st.bg(move || color("surface-raised")))
                .role(AccessibilityRole::Option)
                .label(move || row.with(|r| SharedString::from(format!("{}{}", r.dir, r.name))))
                .accessibility_state(move || AccessibilityState { selected: on.get(), ..Default::default() })
                .on_press(move || {
                    keys.focus();
                    s.update(|a, tx| {
                        if let Overlay::Pick(p) = &mut a.overlay {
                            p.sel = at.index();
                        }
                        a.pick_accept(tx);
                    })
                })
                .child(text(move || if on.get() { "> " } else { "  " }).text_size(px(SIZE)).text_color(move || color("primary")))
                .child(dynamic(move || row_line(row.get())))
        },
    )
    .estimated_item_size(px(ROW_H))
    .viewability(Viewability { item_visible_threshold: Some(percentage(1.0)), ..Default::default() })
    .on_viewable_items_change(move |e: &ViewableItems| {
        if let Some((first, last)) = e.viewable {
            top.with(|t| *t = first);
            fit.with(|f| *f = last + 1 - first);
        }
    })
    .empty(text(move || head.with(|h| h.empty.clone())).text_size(px(SIZE)).px(px(24.0)).text_color(move || color("muted-foreground")))
    .flex_1()
    .test_id("picker-list")
    .element_ref(&list);
    view()
        .flex_1()
        .min_h(px(0.0))
        .pt(px(10.0))
        .pb(px(6.0))
        .overflow_hidden()
        .on_layout(move |b: &Bounds<Points>| {
            let rows = ((b.size.height.0 - 16.0) / ROW_H).floor().max(1.0) as usize;
            fit.with(|f| *f = rows);
        })
        .child(body)
}

/// A row's texts: the directory dim, the name with its matched chars bold in the accent
/// colour, then the extra text dim.
fn row_line(r: RowText) -> AnyElement {
    let mut name = text("").text_size(px(SIZE)).number_of_lines(1);
    for (run, hit) in hit_runs(&r.name, &r.hits) {
        let mut t = text(run).text_color(move || color(if hit { "primary" } else { "foreground" }));
        if hit {
            t = t.font_weight(FontWeight::BOLD);
        }
        name = name.child(t);
    }
    let mut line = view()
        .flex_row()
        .flex_1()
        .min_w(px(0.0))
        .items_center()
        .child(text(r.dir).text_size(px(SIZE)).number_of_lines(1).text_color(move || color("muted-foreground")))
        .child(name);
    if !r.extra.is_empty() {
        line = line.child(
            text(r.extra).text_size(px(SIZE)).pl(px(12.0)).flex_1().min_w(px(0.0)).truncate().text_color(move || color("muted-foreground")),
        );
    }
    line.into_element()
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

/// The picked file's start, or the lines around the picked line, coloured as code.
fn preview(s: Session) -> impl IntoElement {
    let src = s.view(preview_src);
    let lines = computed(move || src.with(|p| p.as_ref().map(preview_lines).unwrap_or_default()));
    let title = move || SharedString::from(src.with(|p| p.as_ref().map(|p| p.title.clone()).unwrap_or_default()));
    let font = s.font;
    titled(title, SIZE).flex_1().min_w(px(0.0)).h_full().test_id("picker-preview").child(
        view().flex_1().min_h(px(0.0)).pt(px(12.0)).overflow_hidden().child(dynamic(move || {
            let lines = lines.get();
            let family = font.with(|f| f.family);
            untracked(move || {
                let mut body = view();
                for l in lines {
                    let mut code = text("").text_size(px(SIZE)).font_family(family).number_of_lines(1);
                    for (t, k) in l.runs {
                        let token = code_token(k);
                        code = code.child(text(t).text_color(move || color(token)));
                    }
                    body = body.child(
                        view()
                            .flex_row()
                            .h(px(21.0))
                            .items_center()
                            .style_when(l.hit, |st| st.bg(move || color("selection")))
                            .child(
                                text(l.number.to_string())
                                    .text_size(px(SIZE))
                                    .font_family(family)
                                    .w(px(44.0))
                                    .pr(px(14.0))
                                    .text_right()
                                    .text_color(move || color("text-faint")),
                            )
                            .child(code),
                    );
                }
                body.into_element()
            })
        })),
    )
}
