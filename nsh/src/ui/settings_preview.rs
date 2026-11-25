//! The live preview of Settings: a short Rust file drawn with the current font, line numbers,
//! cursor and whitespace options, and the config.toml lines the options write.

use std::path::Path;

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_core::prelude::*;

use crate::config::Config;
use crate::store::Session;
use crate::ui::style::{CodeFont, code_token, color};

const SNIPPET: &[(usize, &str)] = &[
    (0, "/// w: start of the next word."),
    (0, "pub fn word_forward(buf: &Buffer, from: Pos) -> Target {"),
    (1, "let mut pos = from;"),
    (1, "let kind = match buf.char_at(pos) {"),
    (2, "c if c != ' ' => Class::Word,"),
    (2, "_ => Class::Blank,"),
    (1, "};"),
    (1, "while class(buf.char_at(pos)) == kind && !buf.at_end(pos) {"),
    (2, "pos = buf.next(pos);"),
    (1, "}"),
    (1, "Target { pos, inclusive: false }"),
    (0, "}"),
];

/// The line the preview's cursor is on, from 1.
const CURSOR_LINE: usize = 3;

/// The config.toml keys the preview echoes.
const ECHO: &[(&str, &[&str])] = &[
    ("theme", &["mode", "dark", "light"]),
    ("font", &["family", "size", "line_height"]),
    ("editor", &["numbers"]),
    ("indent", &["width"]),
    ("format", &["trim_whitespace"]),
    ("keys", &["map"]),
];

/// What the preview shows: the options it follows.
#[derive(Clone, PartialEq)]
struct Look {
    numbers: String,
    cursor: String,
    cursorline: bool,
    whitespace: bool,
    tab: usize,
    toml: Vec<(String, String)>,
}

fn look(cfg: &Config) -> Look {
    let doc: toml::Table = toml::from_str(&cfg.to_toml()).unwrap_or_default();
    let toml = ECHO
        .iter()
        .flat_map(|(section, keys)| {
            let doc = &doc;
            keys.iter().map(move |k| {
                let v = doc.get(*section).and_then(|t| t.get(*k)).map(|v| v.to_string()).unwrap_or_default();
                (format!("{section}.{k}"), v)
            })
        })
        .collect();
    Look {
        numbers: cfg.editor.numbers.clone(),
        cursor: cfg.editor.cursor.clone(),
        cursorline: cfg.editor.cursorline,
        whitespace: cfg.editor.whitespace,
        tab: cfg.indent.width.max(1) as usize,
        toml,
    }
}

pub fn preview(s: Session) -> impl IntoElement {
    let look = s.view(|a| look(&a.cfg));
    let font = s.font;
    view()
        .w(px(480.0))
        .flex_shrink_0()
        .pt(px(18.0))
        .pr(px(20.0))
        .gap(px(18.0))
        .test_id("settings-preview")
        .child(panel("preview · motion.rs", dynamic(move || code(&look.get(), &font.get()))))
        .child(panel("config.toml", dynamic(move || toml_lines(&look.get(), &font.get()))))
}

fn panel(title: &'static str, body: AnyElement) -> impl IntoElement {
    crate::ui::settings_controls::titled(title).w_full().child(body)
}

/// The snippet's lines, numbered as `numbers` asks.
fn code(l: &Look, font: &CodeFont) -> AnyElement {
    let src: Vec<String> = SNIPPET.iter().map(|(lvl, t)| format!("{}{t}", " ".repeat(lvl * l.tab))).collect();
    let runs = highlight::runs(&Buffer::from_str(&src.join("\n")), Some(Path::new("motion.rs")), 0, src.len());
    let mut out = view().w_full().pt(px(14.0)).pb(px(12.0)).overflow_hidden();
    for (i, line) in src.iter().enumerate() {
        let n = i + 1;
        let current = n == CURSOR_LINE;
        let num = match l.numbers.as_str() {
            "off" => String::new(),
            "absolute" => n.to_string(),
            "relative" => n.abs_diff(CURSOR_LINE).to_string(),
            _ if current => n.to_string(),
            _ => n.abs_diff(CURSOR_LINE).to_string(),
        };
        let lead = line.chars().take_while(|c| *c == ' ').count();
        let chars: Vec<char> = line.chars().collect();
        let mut body = mono(font, "");
        if l.whitespace && lead > 0 {
            body = body.child(mono(font, "·".repeat(lead)).text_color(move || color("border")));
        }
        // The cursor sits on the fifth char after the indent.
        let at = if current { Some(lead + 4) } else { None };
        let mut col = if l.whitespace { lead } else { 0 };
        let line_runs = runs.get(i).cloned().unwrap_or_default();
        let mut pieces: Vec<(usize, usize, Option<Kind>)> = Vec::new();
        for (r, k) in line_runs {
            let (a, b) = (r.start.min(chars.len()), r.end.min(chars.len()));
            if a > col {
                pieces.push((col, a, None));
            }
            pieces.push((a.max(col), b, Some(k)));
            col = col.max(b);
        }
        if col < chars.len() {
            pieces.push((col, chars.len(), None));
        }
        for (a, b, kind) in pieces {
            if a >= b {
                continue;
            }
            // Split a piece at the cursor, so the cursor's char gets its own look.
            match at.filter(|c| (a..b).contains(c)) {
                Some(c) => {
                    if a < c {
                        body = body.child(span(font, &chars[a..c], kind));
                    }
                    body = body.child(cursor(font, chars[c], &l.cursor));
                    if c + 1 < b {
                        body = body.child(span(font, &chars[c + 1..b], kind));
                    }
                }
                None => body = body.child(span(font, &chars[a..b], kind)),
            }
        }
        let cursorline = current && l.cursorline;
        out = out.child(
            view()
                .flex_row()
                .h(px(font.row_h))
                .items_center()
                .bg(move || color(if cursorline { "surface-raised" } else { "background" }))
                .child(
                    view()
                        .w(px(font.size * 3.0))
                        .mr(px(font.size * 1.2))
                        .items_end()
                        .child(mono(font, num).text_color(move || color(if current { "foreground" } else { "text-faint" }))),
                )
                .child(body),
        );
    }
    out.into_element()
}

fn span(font: &CodeFont, chars: &[char], kind: Option<Kind>) -> Text {
    let token = code_token(kind);
    let t = mono(font, chars.iter().collect::<String>()).text_color(move || color(token));
    if kind == Some(Kind::Comment) { t.italic() } else { t }
}

/// The cursor's char: a block fills it, a bar and an underline mark it.
fn cursor(font: &CodeFont, c: char, shape: &str) -> Text {
    let t = mono(font, c.to_string());
    match shape {
        "bar" => t.text_color(move || color("primary")).bg(move || color("surface-raised")),
        "underline" => t.underline().text_color(move || color("foreground")),
        _ => t.bg(move || color("foreground")).text_color(move || color("background")),
    }
}

fn toml_lines(l: &Look, font: &CodeFont) -> AnyElement {
    let small = CodeFont { size: 12.0, ..font.clone() };
    let mut out = view().w_full().pt(px(16.0)).pb(px(12.0)).px(px(16.0)).gap(px(2.0)).test_id("settings-toml");
    for (k, v) in &l.toml {
        let token = if v.starts_with('"') {
            "code-string"
        } else if v == "true" || v == "false" {
            "code-keyword"
        } else {
            "code-number"
        };
        out = out.child(
            mono(&small, "")
                .child(mono(&small, k.clone()).text_color(move || color("code-property")))
                .child(mono(&small, " = ").text_color(move || color("code-punctuation")))
                .child(mono(&small, v.clone()).text_color(move || color(token))),
        );
    }
    out.into_element()
}

/// Text in the code font, on one line, with its spaces kept.
fn mono(font: &CodeFont, content: impl Into<SharedString>) -> Text {
    text(content.into()).font_family(font.family).text_size(px(font.size)).number_of_lines(1).allow_font_scaling(false)
}
