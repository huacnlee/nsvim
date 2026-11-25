//! An agent's proposed change, drawn as a diff in place of the text area: removed lines tinted
//! red, added lines tinted green, unchanged runs folded, and a banner with the keys. `y`
//! accepts, `n` rejects, `]c` and `[c` go between changes (`App::proposal_key`).

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_core::prelude::*;

use crate::app::App;
use crate::app::review::Row as DiffKind;
use crate::store::Session;
use crate::ui::style::{code_token, color};
use crate::ui::text_area::code_text;

/// What a row of the diff is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Line {
    Same,
    Added,
    Removed,
    Fold,
}

/// A row of the diff as it shows.
#[derive(Clone, Debug, PartialEq)]
pub struct DiffRow {
    pub line: Line,
    /// The line number in the new text, or empty.
    pub number: String,
    pub segs: Vec<(String, Option<Kind>)>,
}

/// The proposal that shows, from its first row on screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub label: String,
    pub added: usize,
    pub removed: usize,
    /// The change at the top of the screen, from 1, and how many there are.
    pub at: usize,
    pub changes: usize,
    /// The file is not open yet.
    pub loading: bool,
    pub rows: Vec<DiffRow>,
    /// The proposal changes nothing.
    pub same: bool,
}

/// The proposal for the active file, as `rows` rows of the text area show it. One row is the
/// banner.
pub fn review(app: &App, rows: usize) -> Option<Review> {
    let p = &app.proposals[app.proposal_here()?];
    let shown: Vec<&DiffKind> = p.rows.iter().skip(p.top).take(rows.saturating_sub(1).max(1)).collect();
    let text = |r: &DiffKind| match r {
        DiffKind::Same { text, .. } | DiffKind::Added { text, .. } | DiffKind::Removed { text } => text.clone(),
        DiffKind::Fold(n) => format!("⋯ {n} unchanged lines"),
    };
    // The rows' colours, from the rows on screen as one text.
    let body = shown.iter().map(|r| text(r)).collect::<Vec<_>>().join("\n");
    let runs = highlight::runs(&Buffer::from_str(&body), Some(&p.path), 0, shown.len());
    let rows = shown
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let (line, number) = match r {
                DiffKind::Same { new, .. } => (Line::Same, (new + 1).to_string()),
                DiffKind::Added { new, .. } => (Line::Added, (new + 1).to_string()),
                DiffKind::Removed { .. } => (Line::Removed, String::new()),
                DiffKind::Fold(_) => (Line::Fold, String::new()),
            };
            let t = text(r);
            let segs = if line == Line::Fold { vec![(t, None)] } else { segments(&t, runs.get(i).map_or(&[][..], |r| r.as_slice())) };
            DiffRow { line, number, segs }
        })
        .collect();
    let at = p.starts.iter().filter(|&&s| s <= p.top + 3).count().clamp(1, p.starts.len().max(1));
    Some(Review {
        label: p.label.clone(),
        added: p.added,
        removed: p.removed,
        at,
        changes: p.starts.len(),
        loading: p.base == u64::MAX,
        rows,
        same: p.rows.is_empty(),
    })
}

/// `text` cut where its kind of code changes.
fn segments(text: &str, runs: &[(std::ops::Range<usize>, Kind)]) -> Vec<(String, Option<Kind>)> {
    let chars: Vec<char> = text.chars().collect();
    let mut kinds: Vec<Option<Kind>> = vec![None; chars.len()];
    for (range, k) in runs {
        for slot in kinds.iter_mut().take(range.end.min(chars.len())).skip(range.start) {
            *slot = Some(*k);
        }
    }
    let mut out: Vec<(String, Option<Kind>)> = Vec::new();
    for (ch, k) in chars.into_iter().zip(kinds) {
        match out.last_mut() {
            Some((s, last)) if *last == k => s.push(ch),
            _ => out.push((ch.to_string(), k)),
        }
    }
    out
}

/// The diff view of the proposal for the active file.
pub fn diff_view(s: Session) -> impl IntoElement {
    let review = s.view(|a| review(a, a.ed.rows.max(1)));
    view().flex_1().test_id("review").child(dynamic(move || {
        let Some(r) = review.get() else { return view().into_element() };
        draw(s, r).into_element()
    }))
}

fn draw(s: Session, r: Review) -> View {
    let g = s.grid();
    let font = s.font.get();
    let small = crate::ui::style::CodeFont { size: (font.size - 1.0).max(9.0), ..font.clone() };
    let banner = view()
        .flex_row()
        .h(px(g.cell_h))
        .px(px(12.0))
        .gap(px(14.0))
        .items_center()
        .bg(move || color("muted"))
        .border_b(px(1.0))
        .border_color(move || color("border"))
        .child(code_text(&small, "PROPOSAL").font_weight(FontWeight::BOLD).text_color(move || color("mode-visual")))
        .child(code_text(&small, r.label).flex_shrink(1.0).text_color(move || color("foreground")))
        .child(code_text(&small, format!("+{}", r.added)).text_color(move || color("mode-insert")))
        .child(code_text(&small, format!("-{}", r.removed)).text_color(move || color("destructive")))
        .child(view().flex_1())
        .child(code_text(&small, format!("{}/{} changes", r.at, r.changes)).text_color(move || color("muted-foreground")))
        .child(code_text(&small, "y accept · n reject · ]c next · [c previous").text_color(move || color("text-faint")));
    let mut area = view().flex_1().child(banner);
    if r.loading {
        return area.child(
            view()
                .h(px(g.cell_h))
                .px(px(12.0))
                .justify_center()
                .child(code_text(&small, "opening the file…").text_color(move || color("muted-foreground"))),
        );
    }
    if r.same {
        return area.child(
            view().px(px(12.0)).py(px(8.0)).child(
                code_text(&small, "the proposal matches the buffer: nothing to change").text_color(move || color("muted-foreground")),
            ),
        );
    }
    for row in r.rows {
        let (sign, sign_token) = match row.line {
            Line::Added => ("+", "mode-insert"),
            Line::Removed => ("-", "destructive"),
            Line::Same | Line::Fold => (" ", "text-faint"),
        };
        let fill = move || match row.line {
            Line::Added => color("mode-insert").opacity(0.13),
            Line::Removed => color("destructive").opacity(0.13),
            Line::Fold => color("muted"),
            Line::Same => color("background"),
        };
        let ink = match row.line {
            Line::Fold => "text-faint",
            Line::Same => "muted-foreground",
            _ => "foreground",
        };
        let mut code = code_text(&font, "");
        for (t, k) in row.segs {
            let token = if k.is_some() { code_token(k) } else { ink };
            code = code.child(code_text(&font, t).text_color(move || color(token)));
        }
        area = area.child(
            view()
                .flex_row()
                .h(px(g.cell_h))
                .items_center()
                .bg(fill)
                .child(view().w(px(2.0 * g.cell_w)).items_center().child(code_text(&font, sign).text_color(move || color(sign_token))))
                .child(
                    view()
                        .w(px(5.0 * g.cell_w))
                        .pr(px(2.0 * g.cell_w))
                        .items_end()
                        .child(code_text(&font, row.number).text_color(move || color("text-faint"))),
                )
                .child(code),
        );
    }
    area
}
