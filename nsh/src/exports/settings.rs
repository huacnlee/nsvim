//! `SettingsPage`: the state of Settings for `ui/settings/*.tsx`, and the changes its controls
//! ask for. Settings covers the whole window. Its keys stay in Rust (`App::settings_key`).

use std::path::Path;

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind, LineRuns};
use nsh_core::prelude::*;

use crate::app::App;
use crate::app::settings::{self, Ctl, Item, SECTIONS, Section};
use crate::config::Config;
use crate::theme;
use crate::ui::style::code_token;

/// What a row of Settings shows as its control.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum SettingsControl {
    /// A heading over a group of rows. `key` is its text.
    #[default]
    Heading,
    /// A group of options, one of them picked.
    Choice,
    /// A pick of one font from a list.
    Font,
    /// A number with a step down and a step up.
    Stepper,
    /// An on / off checkbox.
    Check,
    /// A value that the user cannot change here.
    Info,
    /// A row of colour chips.
    Chips,
    /// Theme cards, one of them picked.
    Themes,
}

/// One option of a choice, font or theme row.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SettingsOption {
    /// What a pick sets: the argument of `SettingsPage.pick`.
    pub value: String,
    /// What the option shows: a label, a font name or a theme name.
    pub label: String,
    /// Theme rows: the colours of the theme as `#rrggbb`. They are data, not roles.
    pub swatches: Vec<String>,
    /// Font rows: the font family list that shows the option in its own font.
    pub family: String,
}

/// One row of Settings.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SettingsItem {
    /// What the row shows as its control.
    #[prop(default)]
    pub control: SettingsControl,
    /// The config key of the option (`editor.wrap`), or the text of a heading.
    pub key: String,
    /// What the option does.
    pub desc: String,
    /// The current value of a choice, font or theme row, or the text of a stepper or info row.
    pub value: String,
    /// A switch row is on.
    pub on: bool,
    /// The options of a choice, font or theme row.
    pub options: Vec<SettingsOption>,
    /// Font rows: the font family list of the current value.
    pub family: String,
    /// Chips rows: the colours as `#rrggbb`. They are data, not roles.
    pub colors: Vec<String>,
}

/// What Settings shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SettingsState {
    /// The names of the sections, in order.
    pub sections: Vec<String>,
    /// The name of the section that shows.
    pub section: String,
    /// The index in `items` of the row the keyboard is on.
    pub row: u32,
    /// The rows of the section.
    pub items: Vec<SettingsItem>,
    /// The path of config.toml, with `~` for the home folder.
    pub config_path: String,
    /// Why the last write of config.toml failed, or nothing after a good write.
    pub error: Option<String>,
}

/// One run of text in the preview, in one colour.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct PreviewSpan {
    /// The text.
    pub text: String,
    /// The colour token of the text (`code-keyword`, `foreground`, `border`).
    pub ink: String,
    /// The text is in italics (a comment).
    pub italic: bool,
    /// The cursor is on this char: `block`, `bar` or `underline`. Empty for other text.
    pub cursor: String,
}

/// One line of the preview's code.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct PreviewLine {
    /// The line number as the numbers option shows it. Empty when numbers are off.
    pub number: String,
    /// The cursor is on this line.
    pub current: bool,
    /// The line has the cursor line highlight.
    pub highlight: bool,
    /// The text of the line.
    pub spans: Vec<PreviewSpan>,
}

/// One line of config.toml in the preview.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct TomlLine {
    /// The key, as `section.key`.
    pub key: String,
    /// The value, as TOML writes it.
    pub value: String,
    /// The colour token of the value.
    pub ink: String,
}

/// What the preview of Settings shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SettingsPreview {
    /// The font family list of the code font.
    pub family: String,
    /// The size of the code font, in points.
    pub size: f64,
    /// The height of one line of code, in points.
    pub row_height: f64,
    /// The lines of the sample code.
    pub lines: Vec<PreviewLine>,
    /// The config.toml lines that the options write.
    pub toml: Vec<TomlLine>,
}

/// Settings: its state, and the changes that its controls ask for. Each change applies at once
/// and goes to config.toml at once.
#[module]
pub struct SettingsPage;

#[module]
impl SettingsPage {
    /// Calls `listener` with what Settings shows, now and after each change.
    pub fn on_change(#[args(state)] listener: Callback<SettingsState>) -> Stop {
        crate::bridge::subscribe(state, listener)
    }

    /// Calls `listener` with what the preview shows, now and after each change.
    pub fn on_preview(#[args(preview)] listener: Callback<SettingsPreview>) -> Stop {
        crate::bridge::subscribe(preview, listener)
    }

    /// Shows the section with the name `name`, from its first row.
    pub fn show_section(name: String) {
        if let Some(&(section, _)) = SECTIONS.iter().find(|(_, n)| *n == name) {
            crate::bridge::update(move |a, _| a.settings_section(section));
        }
    }

    /// Picks `value` for the choice, font or theme row `key`.
    pub fn pick(key: String, value: String) {
        crate::bridge::update(move |a, _| a.settings_pick(&key, &value));
    }

    /// Turns the switch row `key` on or off.
    pub fn toggle(key: String) {
        crate::bridge::update(move |a, _| a.settings_toggle(&key));
    }

    /// Steps the number row `key` down (`dir` -1) or up (`dir` 1).
    pub fn step(key: String, dir: f64) {
        crate::bridge::update(move |a, _| a.settings_step(&key, dir.signum() as f32));
    }

    /// Closes Settings and opens config.toml as a buffer.
    pub fn open_config() {
        crate::bridge::update(|a, _| a.open_path(crate::config::config_path(), None));
    }

    /// Closes Settings. Each change is on disk already.
    pub fn close() {
        crate::bridge::update(|a, _| a.close_settings());
    }
}

fn hex(c: crate::color::Color) -> String {
    c.to_hex()
}

fn state(a: &App) -> SettingsState {
    let (section, row, error) = a.settings.as_ref().map_or((Section::Appearance, 0, None), |st| (st.section, st.row, st.error.clone()));
    let items = settings::items(section, &a.cfg, a.omarchy.as_ref(), dark()).into_iter().map(item).collect();
    SettingsState {
        sections: SECTIONS.iter().map(|(_, n)| n.to_string()).collect(),
        section: SECTIONS.iter().find(|(s, _)| *s == section).map_or("", |(_, n)| n).to_string(),
        row: row as u32,
        items,
        config_path: crate::app::tilde(&crate::config::config_path()),
        error,
    }
}

fn item(i: Item) -> SettingsItem {
    let (key, desc, ctl) = match i {
        Item::Head(t) => return SettingsItem { key: t.to_string(), ..Default::default() },
        Item::Row { key, desc, ctl } => (key.to_string(), desc, ctl),
    };
    let base = SettingsItem { key, desc, ..Default::default() };
    match ctl {
        Ctl::Seg { opts, cur } => SettingsItem {
            control: SettingsControl::Choice,
            value: cur,
            options: opts.into_iter().map(|(value, label)| SettingsOption { value, label, ..Default::default() }).collect(),
            ..base
        },
        Ctl::Font { opts, cur } => SettingsItem {
            control: SettingsControl::Font,
            family: crate::geometry::family(&cur).to_string(),
            value: cur,
            options: opts
                .into_iter()
                .map(|f| SettingsOption {
                    family: crate::geometry::family(&f).to_string(),
                    label: f.clone(),
                    value: f,
                    ..Default::default()
                })
                .collect(),
            ..base
        },
        Ctl::Step { shown } => SettingsItem { control: SettingsControl::Stepper, value: shown, ..base },
        Ctl::Toggle(on) => SettingsItem { control: SettingsControl::Check, on, ..base },
        Ctl::Info(shown) => SettingsItem { control: SettingsControl::Info, value: shown, ..base },
        Ctl::Chips(colors) => SettingsItem { control: SettingsControl::Chips, colors: colors.into_iter().map(hex).collect(), ..base },
        Ctl::Themes { opts, cur } => SettingsItem {
            control: SettingsControl::Themes,
            value: cur,
            options: opts
                .into_iter()
                .map(|id| SettingsOption {
                    label: theme::name(&id).to_string(),
                    swatches: theme::swatches(&id).into_iter().map(hex).collect(),
                    value: id,
                    ..Default::default()
                })
                .collect(),
            ..base
        },
    }
}

// The preview.

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

/// The config.toml keys the preview shows.
const ECHO: &[(&str, &[&str])] = &[
    ("theme", &["mode", "dark", "light"]),
    ("font", &["family", "size", "line_height"]),
    ("editor", &["numbers"]),
    ("indent", &["width"]),
    ("format", &["trim_whitespace"]),
    ("keys", &["map"]),
];

/// The pieces of a line of `len` chars from `from` on: char ranges and their kinds, with the
/// gaps between the runs as plain text.
pub(crate) fn pieces(runs: &LineRuns, len: usize, from: usize) -> Vec<(usize, usize, Option<Kind>)> {
    let mut col = from;
    let mut out = Vec::new();
    for (r, k) in runs {
        let (a, b) = (r.start.min(len), r.end.min(len));
        if a > col {
            out.push((col, a, None));
        }
        out.push((a.max(col), b, Some(*k)));
        col = col.max(b);
    }
    if col < len {
        out.push((col, len, None));
    }
    out.retain(|(a, b, _)| a < b);
    out
}

fn span(chars: &[char], kind: Option<Kind>) -> PreviewSpan {
    PreviewSpan {
        text: chars.iter().collect(),
        ink: code_token(kind).to_string(),
        italic: kind == Some(Kind::Comment),
        cursor: String::new(),
    }
}

fn preview(a: &App) -> SettingsPreview {
    let cfg = &a.cfg;
    let tab = cfg.indent.width.max(1) as usize;
    let src: Vec<String> = SNIPPET.iter().map(|(lvl, t)| format!("{}{t}", " ".repeat(lvl * tab))).collect();
    let runs = highlight::runs(&Buffer::from_str(&src.join("\n")), Some(Path::new("motion.rs")), 0, src.len());
    let cursor = match cfg.editor.cursor.as_str() {
        "bar" | "underline" => cfg.editor.cursor.clone(),
        _ => "block".to_string(),
    };
    let lines = src
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let n = i + 1;
            let current = n == CURSOR_LINE;
            let number = match cfg.editor.numbers.as_str() {
                "off" => String::new(),
                "absolute" => n.to_string(),
                "relative" => n.abs_diff(CURSOR_LINE).to_string(),
                _ if current => n.to_string(),
                _ => n.abs_diff(CURSOR_LINE).to_string(),
            };
            let chars: Vec<char> = line.chars().collect();
            let lead = chars.iter().take_while(|c| **c == ' ').count();
            let mut spans = Vec::new();
            if cfg.editor.whitespace && lead > 0 {
                spans.push(PreviewSpan { text: "·".repeat(lead), ink: "border".into(), ..Default::default() });
            }
            // The cursor sits on the fifth char after the indent.
            let at = if current { Some(lead + 4) } else { None };
            let from = if cfg.editor.whitespace { lead } else { 0 };
            let empty = Vec::new();
            for (a, b, kind) in pieces(runs.get(i).unwrap_or(&empty), chars.len(), from) {
                match at.filter(|c| (a..b).contains(c)) {
                    Some(c) => {
                        if a < c {
                            spans.push(span(&chars[a..c], kind));
                        }
                        spans.push(PreviewSpan { cursor: cursor.clone(), ..span(&chars[c..c + 1], kind) });
                        if c + 1 < b {
                            spans.push(span(&chars[c + 1..b], kind));
                        }
                    }
                    None => spans.push(span(&chars[a..b], kind)),
                }
            }
            PreviewLine { number, current, highlight: current && cfg.editor.cursorline, spans }
        })
        .collect();
    SettingsPreview {
        family: a.font_family.to_string(),
        size: cfg.font.size as f64,
        row_height: crate::geometry::row_height(a) as f64,
        lines,
        toml: toml_lines(cfg),
    }
}

fn toml_lines(cfg: &Config) -> Vec<TomlLine> {
    let doc: toml::Table = toml::from_str(&cfg.to_toml()).unwrap_or_default();
    ECHO.iter()
        .flat_map(|(section, keys)| keys.iter().map(move |k| (*section, *k)))
        .map(|(section, k)| {
            let value = doc.get(section).and_then(|t| t.get(k)).map(|v| v.to_string()).unwrap_or_default();
            let ink = if value.starts_with('"') {
                "code-string"
            } else if value == "true" || value == "false" {
                "code-keyword"
            } else {
                "code-number"
            };
            TomlLine { key: format!("{section}.{k}"), value, ink: ink.into() }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_marks_the_cursor_and_the_whitespace() {
        let mut a = App::new(Config::default(), std::path::PathBuf::from("/p"), false, None);
        a.cfg.editor.whitespace = true;
        a.cfg.editor.numbers = "absolute".into();
        let p = preview(&a);
        assert_eq!(p.lines.len(), SNIPPET.len());
        let line = &p.lines[CURSOR_LINE - 1];
        assert!(line.current);
        assert_eq!(line.number, "3");
        assert_eq!(line.spans[0].text, "····");
        let text: String = line.spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(text, "····let mut pos = from;");
        assert_eq!(line.spans.iter().filter(|s| !s.cursor.is_empty()).count(), 1);
        assert!(p.toml.iter().any(|t| t.key == "editor.numbers" && t.value == "\"absolute\"" && t.ink == "code-string"));
    }

    #[test]
    fn pieces_fill_the_gaps_between_runs() {
        let runs: LineRuns = vec![(2..4, Kind::Keyword)];
        assert_eq!(pieces(&runs, 6, 0), vec![(0, 2, None), (2, 4, Some(Kind::Keyword)), (4, 6, None)]);
        assert_eq!(pieces(&runs, 6, 3), vec![(3, 4, Some(Kind::Keyword)), (4, 6, None)]);
    }
}
