//! `SetupFlow`: the state of first-launch setup for `src/setup/*.tsx`, and the picks its cards,
//! switches and buttons ask for. The keys of setup stay in Rust (`App::setup_key`).

use std::path::Path;

use editor::buffer::Buffer;
use editor::syntax::highlight::{self, Kind};
use nsh_core::prelude::*;

use crate::app::setup::{Choice, FONTS, LOOKS, SIZES, STEPS, Setup};
use crate::app::{App, Screen};
use crate::theme::{self, Palette};

/// Every switch of setup, as `Setup::toggle_on` knows it.
const TOGGLES: &[&str] = &["leader", "import", "multi", "autoclose", "click", "drag", "double", "right", "vim", "env", "git", "default"];

/// One run of text in a look's sample, in one colour.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SampleSpan {
    /// The text.
    pub text: String,
    /// The colour as `#rrggbb`. It is data: the look is not the theme that shows now.
    pub color: String,
}

/// One line of a look's sample.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SampleLine {
    /// The runs of the line.
    pub spans: Vec<SampleSpan>,
}

/// One look of the look step: a few lines of code in its own colours.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SetupLook {
    /// The look's id in config.toml: `light`, `dark` or `system`.
    pub id: String,
    /// The background colour of the sample, as `#rrggbb`.
    pub background: String,
    /// The border colour of the sample, as `#rrggbb`.
    pub border: String,
    /// The lines of the sample.
    pub lines: Vec<SampleLine>,
}

/// What first-launch setup shows.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct SetupState {
    /// The step that shows, from 1, as its place in `steps`.
    pub step: u32,
    /// The names of the steps that show, in order. Windows has no `Terminal` step.
    pub steps: Vec<String>,
    /// The id of the choice the keyboard is on (see `SetupFlow.choose`).
    pub focus: String,
    /// The keys show for macOS (⌘), not for Linux and Windows (Ctrl).
    pub mac: bool,
    /// The Neovim keymap is the pick, not the classic one.
    pub vim: bool,
    /// The switches that are on, by name (`leader`, `git`).
    pub on: Vec<String>,
    /// The mouse is on in the text area.
    pub mouse: bool,
    /// The theme mode: `light`, `dark` or `system`.
    pub theme_mode: String,
    /// The name of the code font.
    pub font: String,
    /// The font family list of the code font.
    pub font_family: String,
    /// The size of the code font, in points.
    pub font_size: f64,
    /// The fonts the look step offers.
    pub fonts: Vec<String>,
    /// The font sizes the look step offers, in points.
    pub sizes: Vec<f64>,
    /// The looks the look step offers, in order.
    pub looks: Vec<SetupLook>,
    /// The name of the Omarchy theme, on Omarchy.
    pub omarchy: Option<String>,
    /// This binary's full path, as the shell names it. The aliases and `$EDITOR` name it.
    pub program: String,
    /// The default-app choice shows: Linux only.
    pub desktop_entry: bool,
    /// This is a development build. It has no terminal step.
    pub dev_build: bool,
    /// The commands that finishing setup runs. A line that starts with `# ` is a note.
    pub commands: Vec<String>,
    /// Setup runs again over the editor (`:Setup`): it can be left with nothing changed.
    pub again: bool,
}

/// First-launch setup: its state, and the picks of its controls. A pick works as Enter does on
/// the same choice.
#[module]
pub struct SetupFlow;

#[module]
impl SetupFlow {
    /// Calls `listener` with what setup shows, now and after each change.
    pub fn on_change(#[args(state)] listener: Callback<SetupState>) -> Stop {
        crate::bridge::subscribe(|a| state(a).unwrap_or_default(), listener)
    }

    /// Picks the choice with the id `id`: `keymap:vim`, `keymap:classic`, `platform:mac`,
    /// `platform:other`, `toggle:<name>`, `mouse:on`, `mouse:off`, `look:<index>`,
    /// `font:<index>`, `size:<index>`, `change:<index>` or `open`.
    pub fn choose(id: String) {
        if let Some(c) = parse(&id) {
            crate::bridge::update(move |a, tx| a.setup_choose(c, tx));
        }
    }

    /// Moves `by` steps: -1 back, 1 on.
    pub fn go(by: i32) {
        crate::bridge::update(move |a, _| a.setup_go(by as isize));
    }

    /// Goes to the last step, where the user reads the summary.
    pub fn skip() {
        crate::bridge::update(|a, _| a.setup_go(STEPS.len() as isize));
    }

    /// Writes config.toml, runs the terminal steps that are on, and opens the editor.
    pub fn finish() {
        crate::bridge::update(|a, tx| a.finish_setup(tx));
    }

    /// Leaves setup run again (`:Setup`) with nothing changed. First launch has no editor to
    /// go back to, so there it does nothing.
    pub fn cancel() {
        crate::bridge::update(|a, _| a.cancel_setup());
    }
}

/// The id of `c`, as `SetupFlow.choose` takes it.
fn id(c: Choice) -> String {
    match c {
        Choice::Keymap(vim) => format!("keymap:{}", if vim { "vim" } else { "classic" }),
        Choice::Platform(mac) => format!("platform:{}", if mac { "mac" } else { "other" }),
        Choice::Toggle(k) => format!("toggle:{k}"),
        Choice::Mouse(on) => format!("mouse:{}", if on { "on" } else { "off" }),
        Choice::Look(i) => format!("look:{i}"),
        Choice::Font(i) => format!("font:{i}"),
        Choice::Size(i) => format!("size:{i}"),
        Choice::Change(i) => format!("change:{i}"),
        Choice::Open => "open".into(),
    }
}

/// The choice with the id `s`.
fn parse(s: &str) -> Option<Choice> {
    let (kind, arg) = s.split_once(':').unwrap_or((s, ""));
    let index = || arg.parse::<usize>().ok();
    Some(match kind {
        "keymap" => Choice::Keymap(arg == "vim"),
        "platform" => Choice::Platform(arg == "mac"),
        "toggle" => Choice::Toggle(TOGGLES.iter().find(|k| **k == arg)?),
        "mouse" => Choice::Mouse(arg == "on"),
        "look" => Choice::Look(index().filter(|i| *i < LOOKS.len())?),
        "font" => Choice::Font(index().filter(|i| *i < FONTS.len())?),
        "size" => Choice::Size(index().filter(|i| *i < SIZES.len())?),
        "change" => Choice::Change(index().filter(|i| *i < STEPS.len())?),
        "open" => Choice::Open,
        _ => return None,
    })
}

fn state(a: &App) -> Option<SetupState> {
    let Screen::Setup(s) = &a.screen else { return None };
    Some(of(s, a, dark()))
}

fn of(s: &Setup, app: &App, system_dark: bool) -> SetupState {
    let choices = s.choices();
    let focus = choices.get(s.focus.min(choices.len().saturating_sub(1))).copied();
    let looks = LOOKS
        .iter()
        .map(|look| {
            let p = theme::resolve(theme::Source::parse(look), "dark", "light", system_dark, app.omarchy.as_ref()).0;
            let comment = match *look {
                "system" => s.omarchy_name.clone().unwrap_or_else(|| "follows the OS".into()),
                other => other.to_string(),
            };
            SetupLook { id: look.to_string(), background: p.bg.to_hex(), border: p.bd.to_hex(), lines: sample(&p, &comment) }
        })
        .collect();
    SetupState {
        // Without the terminal step, the steps after it move up one place.
        step: s.steps().iter().position(|t| *t == STEPS[s.step - 1]).map_or(s.step, |i| i + 1) as u32,
        steps: s.steps().iter().map(|t| t.to_string()).collect(),
        focus: focus.map(id).unwrap_or_default(),
        mac: s.mac,
        vim: s.vim(),
        on: TOGGLES.iter().filter(|k| s.toggle_on(k)).map(|k| k.to_string()).collect(),
        mouse: s.cfg.mouse.enabled,
        theme_mode: s.cfg.theme.mode.clone(),
        font: s.cfg.font.family.clone(),
        font_family: crate::geometry::family(&s.cfg.font.family).to_string(),
        font_size: s.cfg.font.size as f64,
        fonts: FONTS.iter().map(|f| f.to_string()).collect(),
        sizes: SIZES.iter().map(|z| *z as f64).collect(),
        looks,
        omarchy: s.omarchy_name.clone(),
        program: s.program(),
        desktop_entry: s.desktop_entry,
        dev_build: s.dev_build,
        commands: if s.step >= 4 { s.commands() } else { Vec::new() },
        again: s.prior.is_some(),
    }
}

/// A few lines of Rust in the colours of `p`, with `comment` in the comment line.
fn sample(p: &Palette, comment: &str) -> Vec<SampleLine> {
    let src = format!("fn class(c: char) {{\n    // {comment}\n    \"word\"\n}}");
    let lines: Vec<&str> = src.lines().collect();
    let runs = highlight::runs(&Buffer::from_str(&src), Some(Path::new("look.rs")), 0, lines.len());
    let empty = Vec::new();
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let chars: Vec<char> = line.chars().collect();
            let spans = super::settings::pieces(runs.get(i).unwrap_or(&empty), chars.len(), 0)
                .into_iter()
                .map(|(a, b, kind)| SampleSpan {
                    text: chars[a..b].iter().collect(),
                    color: kind.map_or(p.fg, |k| kind_color(p, k)).to_hex(),
                })
                .collect();
            SampleLine { spans }
        })
        .collect()
}

fn kind_color(p: &Palette, k: Kind) -> crate::color::Color {
    match k {
        Kind::Keyword => p.kw,
        Kind::Function => p.fun,
        Kind::Type => p.ty,
        Kind::String => p.str_,
        Kind::Number => p.num,
        Kind::Comment => p.com,
        Kind::Macro => p.mac,
        Kind::Property => p.prop,
        Kind::Punct => p.pun,
        Kind::Text => p.fg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_choice_id_reads_back_as_its_choice() {
        let mut s = Setup::new(None);
        for step in 1..=STEPS.len() {
            s.step = step;
            for c in s.choices() {
                assert_eq!(parse(&id(c)), Some(c), "{}", id(c));
            }
        }
        assert_eq!(parse("toggle:nope"), None);
        assert_eq!(parse("toggle:path"), None, "setup adds no command");
        assert_eq!(parse("look:9"), None);
    }

    #[test]
    fn the_samples_keep_the_text_of_each_line() {
        let p = theme::builtin("dark");
        let lines = sample(&p, "dark");
        let text: Vec<String> = lines.iter().map(|l| l.spans.iter().map(|s| s.text.as_str()).collect()).collect();
        assert_eq!(text, ["fn class(c: char) {", "    // dark", "    \"word\"", "}"]);
    }
}
