//! Settings as a buffer: sections on the left, options in the middle, a live preview and the
//! config.toml it writes on the right. `:w` writes, `:q` closes. Changes apply at once.

use super::fx::Tx;
use crate::color::Color;
use editor::editor::{Action, KeyCode, KeyInput};

use super::App;
use crate::config::Config;
use crate::theme::{self, Omarchy};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    Appearance,
    Font,
    Editor,
    Formatting,
    Keys,
}

pub const SECTIONS: &[(Section, &str)] = &[
    (Section::Appearance, "appearance"),
    (Section::Font, "font"),
    (Section::Editor, "editor"),
    (Section::Formatting, "formatting"),
    (Section::Keys, "keys & mouse"),
];

/// The fonts Settings offers. Any installed monospace font works in config.toml.
pub const FONTS: &[&str] = &["JetBrains Mono", "IBM Plex Mono", "Fira Code", "Source Code Pro", "Roboto Mono"];

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub section: Section,
    /// The row the keyboard is on, an index into the section's items.
    pub row: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ctl {
    Seg { opts: Vec<(String, String)>, cur: String },
    Font { opts: Vec<String>, cur: String },
    Step { shown: String },
    Toggle(bool),
    Info(String),
    Chips(Vec<Color>),
    Themes { opts: Vec<String>, cur: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Head(&'static str),
    Row { key: &'static str, desc: String, ctl: Ctl },
}

impl Item {
    pub fn is_control(&self) -> bool {
        matches!(self, Item::Row { ctl, .. } if !matches!(ctl, Ctl::Info(_) | Ctl::Chips(_)))
    }
}

fn seg(key: &'static str, desc: &str, cur: &str, opts: &[(&str, &str)]) -> Item {
    Item::Row {
        key,
        desc: desc.into(),
        ctl: Ctl::Seg { opts: opts.iter().map(|(v, l)| (v.to_string(), l.to_string())).collect(), cur: cur.into() },
    }
}

fn step(key: &'static str, desc: &str, shown: String) -> Item {
    Item::Row { key, desc: desc.into(), ctl: Ctl::Step { shown } }
}

fn tog(key: &'static str, desc: &str, on: bool) -> Item {
    Item::Row { key, desc: desc.into(), ctl: Ctl::Toggle(on) }
}

fn info(key: &'static str, desc: &str, v: String) -> Item {
    Item::Row { key, desc: desc.into(), ctl: Ctl::Info(v) }
}

/// The items of `section` for `cfg`.
pub fn items(section: Section, cfg: &Config, omarchy: Option<&Omarchy>, system_dark: bool) -> Vec<Item> {
    let mut out = Vec::new();
    match section {
        Section::Appearance => {
            out.push(Item::Head("-- theme"));
            out.push(seg(
                "theme",
                "System follows your OS. Omarchy follows the desktop theme.",
                &cfg.theme.mode,
                &[("light", "light"), ("dark", "dark"), ("system", "system"), ("omarchy", "omarchy")],
            ));
            let mode = cfg.theme.mode.as_str();
            let themes = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            if mode == "dark" || mode == "system" {
                let desc = if mode == "system" { "Used when the OS is dark" } else { "Used in dark mode" };
                out.push(Item::Row {
                    key: "theme.dark",
                    desc: desc.into(),
                    ctl: Ctl::Themes { opts: themes(theme::DARK_THEMES), cur: cfg.theme.dark.clone() },
                });
            }
            if mode == "light" || mode == "system" {
                let desc = if mode == "system" { "Used when the OS is light" } else { "Used in light mode" };
                out.push(Item::Row {
                    key: "theme.light",
                    desc: desc.into(),
                    ctl: Ctl::Themes { opts: themes(theme::LIGHT_THEMES), cur: cfg.theme.light.clone() },
                });
            }
            if mode == "system" {
                out.push(info("system.now", "What the OS asks for right now", if system_dark { "dark".into() } else { "light".into() }));
            }
            if mode == "omarchy" {
                match omarchy {
                    Some(o) => {
                        out.push(info("omarchy.current", "Read from the theme's colors.toml", o.name.clone()));
                        let p = o.palette;
                        out.push(Item::Row {
                            key: "omarchy.palette",
                            desc: "Background, text, accent and syntax colors".into(),
                            ctl: Ctl::Chips(vec![p.bg, p.fg, p.ac, p.ins, p.vis, p.cmd, p.kw, p.str_, p.ty, p.err]),
                        });
                        out.push(info("omarchy.source", "Watched for changes; the editor re-themes live", crate::app::tilde(&o.dir)));
                        out.push(info("omarchy.switch", "Change the theme from Omarchy", "Super Ctrl Shift Space".into()));
                        out.push(info(
                            "omarchy.light",
                            "A theme with a light.mode file gets the light variant",
                            if p.light { "light".into() } else { "dark".into() },
                        ));
                    }
                    None => out.push(info("omarchy.current", "No Omarchy theme found. The dark theme applies", "not found".into())),
                }
            }
            out.push(Item::Head("-- window"));
            out.push(step(
                "window.opacity",
                "Below 100% lets a compositor blur show through (next launch)",
                format!("{}%", cfg.window.opacity),
            ));
        }
        Section::Font => {
            out.push(Item::Head("-- editor font"));
            out.push(Item::Row {
                key: "font.family",
                desc: "Any installed monospace font works".into(),
                ctl: Ctl::Font { opts: FONTS.iter().map(|s| s.to_string()).collect(), cur: cfg.font.family.clone() },
            });
            out.push(step("font.size", "Points", format!("{} pt", fmt_num(cfg.font.size))));
            out.push(step("font.line_height", "Multiple of the font size", format!("{:.2}", cfg.font.line_height)));
            out.push(step(
                "font.letter_spacing",
                "Pixels between characters",
                format!("{}{} px", if cfg.font.letter_spacing > 0.0 { "+" } else { "" }, fmt_num(cfg.font.letter_spacing)),
            ));
            out.push(seg(
                "font.weight",
                "Regular reads lighter on light themes",
                &cfg.font.weight.to_string(),
                &[("400", "regular"), ("500", "medium"), ("700", "bold")],
            ));
            out.push(tog("font.ligatures", "Joins -> != => into single glyphs", cfg.font.ligatures));
            out.push(Item::Head("-- interface"));
            out.push(seg(
                "ui.font",
                "Tree, tabs and status line",
                &cfg.ui.font,
                &[("editor", "same as editor"), ("system", "system sans")],
            ));
            out.push(step("ui.size", "Interface text size", format!("{} pt", fmt_num(cfg.ui.size))));
        }
        Section::Editor => {
            out.push(Item::Head("-- display"));
            out.push(seg(
                "editor.numbers",
                "Hybrid shows the cursor line as absolute",
                &cfg.editor.numbers,
                &[("hybrid", "hybrid"), ("relative", "relative"), ("absolute", "absolute"), ("off", "off")],
            ));
            out.push(seg(
                "editor.cursor",
                "Insert mode always uses a bar",
                &cfg.editor.cursor,
                &[("block", "block"), ("bar", "bar"), ("underline", "underline")],
            ));
            out.push(tog("editor.cursorline", "Highlight the line the cursor is on", cfg.editor.cursorline));
            out.push(seg(
                "editor.ruler",
                "A thin guide at this column",
                &cfg.editor.ruler.to_string(),
                &[("0", "off"), ("80", "80"), ("100", "100"), ("120", "120")],
            ));
            out.push(tog("editor.whitespace", "Show leading spaces as dots", cfg.editor.whitespace));
            out.push(tog("editor.wrap", "Soft-wrap long lines", cfg.editor.wrap));
            out.push(step("editor.scrolloff", "Lines kept visible above and below the cursor", cfg.editor.scrolloff.to_string()));
            out.push(Item::Head("-- indentation"));
            out.push(seg(
                "indent.width",
                "Columns per indent level. .editorconfig wins per file",
                &cfg.indent.width.to_string(),
                &[("2", "2"), ("4", "4"), ("8", "8")],
            ));
            out.push(tog("indent.spaces", "Insert spaces when Tab is pressed", cfg.indent.spaces));
        }
        Section::Formatting => {
            out.push(Item::Head("-- on save"));
            out.push(tog("format.trim_whitespace", "Remove trailing spaces", cfg.format.trim_whitespace));
            out.push(tog("format.final_newline", "End every file with a newline", cfg.format.final_newline));
            out.push(seg("format.line_endings", "Line endings for new files", &cfg.format.line_endings, &[("lf", "LF"), ("crlf", "CRLF")]));
        }
        Section::Keys => {
            out.push(Item::Head("-- keys"));
            out.push(seg("keys.map", "Change any time with :set keymap", &cfg.keys.map, &[("neovim", "neovim"), ("classic", "classic")]));
            out.push(seg("keys.leader", "Neovim keymap only", &cfg.keys.leader, &[("space", "space"), ("backslash", "\\")]));
            out.push(tog("keys.auto_close", "Typing ( also types )", cfg.keys.auto_close));
            out.push(Item::Head("-- mouse"));
            out.push(tog("mouse.enabled", "Click, drag to select, resize the file tree", cfg.mouse.enabled));
            out.push(tog("mouse.scroll", "The wheel scrolls", cfg.mouse.scroll));
            out.push(tog("mouse.right_click", "Right-click opens a context menu", cfg.mouse.right_click));
            out.push(Item::Head("-- agents"));
            out.push(tog("ipc.enabled", "Serve editor ctl and editor mcp for agents and scripts (next launch)", cfg.ipc.enabled));
            out.push(tog("ipc.allow_edits", "Agents may edit buffers and run commands. Proposals work either way", cfg.ipc.allow_edits));
        }
    }
    out
}

fn fmt_num(v: f32) -> String {
    if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") }
}

/// Picks `value` for a segmented, font or theme row.
pub fn pick(cfg: &mut Config, key: &str, value: &str) {
    let v = value.to_string();
    match key {
        "theme" => cfg.theme.mode = v,
        "theme.dark" => cfg.theme.dark = v,
        "theme.light" => cfg.theme.light = v,
        "font.family" => cfg.font.family = v,
        "font.weight" => cfg.font.weight = v.parse().unwrap_or(400),
        "ui.font" => cfg.ui.font = v,
        "editor.numbers" => cfg.editor.numbers = v,
        "editor.cursor" => cfg.editor.cursor = v,
        "editor.ruler" => cfg.editor.ruler = v.parse().unwrap_or(0),
        "indent.width" => cfg.indent.width = v.parse().unwrap_or(4),
        "format.line_endings" => cfg.format.line_endings = v,
        "keys.map" => cfg.keys.map = v,
        "keys.leader" => cfg.keys.leader = v,
        _ => {}
    }
}

/// Steps a number row down (`-1`) or up (`1`).
pub fn step_by(cfg: &mut Config, key: &str, dir: f32) {
    let clamp = |v: f32, lo: f32, hi: f32| (v.clamp(lo, hi) * 100.0).round() / 100.0;
    match key {
        "window.opacity" => cfg.window.opacity = (cfg.window.opacity as f32 + 5.0 * dir).clamp(70.0, 100.0) as u8,
        "font.size" => cfg.font.size = clamp(cfg.font.size + dir, 9.0, 24.0),
        "font.line_height" => cfg.font.line_height = clamp(cfg.font.line_height + 0.05 * dir, 1.0, 2.4),
        "font.letter_spacing" => cfg.font.letter_spacing = clamp(cfg.font.letter_spacing + 0.5 * dir, -1.0, 2.0),
        "ui.size" => cfg.ui.size = clamp(cfg.ui.size + 0.5 * dir, 10.0, 16.0),
        "editor.scrolloff" => cfg.editor.scrolloff = (cfg.editor.scrolloff as f32 + dir).clamp(0.0, 20.0) as u16,
        _ => {}
    }
}

pub fn toggle(cfg: &mut Config, key: &str) {
    match key {
        "font.ligatures" => cfg.font.ligatures ^= true,
        "editor.cursorline" => cfg.editor.cursorline ^= true,
        "editor.whitespace" => cfg.editor.whitespace ^= true,
        "editor.wrap" => cfg.editor.wrap ^= true,
        "indent.spaces" => cfg.indent.spaces ^= true,
        "format.trim_whitespace" => cfg.format.trim_whitespace ^= true,
        "format.final_newline" => cfg.format.final_newline ^= true,
        "keys.auto_close" => cfg.keys.auto_close ^= true,
        "mouse.enabled" => cfg.mouse.enabled ^= true,
        "mouse.scroll" => cfg.mouse.scroll ^= true,
        "mouse.right_click" => cfg.mouse.right_click ^= true,
        "ipc.enabled" => cfg.ipc.enabled ^= true,
        "ipc.allow_edits" => cfg.ipc.allow_edits ^= true,
        _ => {}
    }
}

/// The TOML line Settings echoes on the command line after a change.
fn echo(cfg: &Config, key: &str) -> String {
    let doc: toml::Table = toml::from_str(&cfg.to_toml()).unwrap_or_default();
    let mut parts = key.split('.');
    let (Some(a), b) = (parts.next(), parts.next()) else { return String::new() };
    let b = if a == "theme" && b.is_none() { Some("mode") } else { b };
    let v = b.and_then(|b| doc.get(a)?.get(b)).map(|v| v.to_string()).unwrap_or_default();
    format!("{a}.{} = {v}", b.unwrap_or(""))
}

impl App {
    /// Shows `section` in Settings, from its first row. The pointer's way to pick a section.
    pub fn settings_section(&mut self, section: Section) {
        if let Some(st) = self.settings.as_mut() {
            st.section = section;
            st.row = 0;
        }
    }

    /// Picks `value` for a segmented, font or theme row, from the pointer.
    pub fn settings_pick(&mut self, key: &str, value: &str) {
        pick(&mut self.cfg, key, value);
        self.settings_changed(key);
    }

    /// Turns a switch row on or off, from the pointer.
    pub fn settings_toggle(&mut self, key: &str) {
        toggle(&mut self.cfg, key);
        self.settings_changed(key);
    }

    /// Steps a number row down (`-1`) or up (`1`), from the pointer.
    pub fn settings_step(&mut self, key: &str, dir: f32) {
        step_by(&mut self.cfg, key, dir);
        self.settings_changed(key);
    }

    /// Applies a change made in Settings (by key or pointer).
    pub fn settings_changed(&mut self, key: &str) {
        let line = echo(&self.cfg, key);
        self.config_changed();
        self.ed.message(line);
    }

    pub fn settings_key(&mut self, k: KeyInput, tx: &mut Tx) {
        let Some(st) = self.settings.clone() else { return };
        let list = items(st.section, &self.cfg, self.omarchy.as_ref(), true);
        let controls: Vec<usize> = (0..list.len()).filter(|&i| list[i].is_control()).collect();
        let cur = controls.iter().position(|&i| i == st.row).unwrap_or(0);
        let row_key = controls.get(cur).and_then(|&i| match &list[i] {
            Item::Row { key, .. } => Some(*key),
            _ => None,
        });
        let sec_i = SECTIONS.iter().position(|(s, _)| *s == st.section).unwrap_or(0);
        let set_row = |app: &mut App, i: usize| {
            if let Some(s) = app.settings.as_mut() {
                s.row = controls.get(i.min(controls.len().saturating_sub(1))).copied().unwrap_or(0);
            }
        };
        let set_section = |app: &mut App, i: usize| {
            if let Some(s) = app.settings.as_mut() {
                s.section = SECTIONS[i % SECTIONS.len()].0;
                s.row = 0;
            }
        };
        let vim = self.is_vim();
        match k.code {
            KeyCode::Char(':') if vim => editor::vim::open_cmdline(&mut self.ed, ':', ""),
            KeyCode::Char('s') if k.cmd && !vim => self.write_config(),
            KeyCode::Char('w') if k.cmd && !vim => self.close_settings(),
            _ if !vim && super::keys::classic_window_key(&k).is_some() => {
                let actions = super::keys::classic_window_key(&k).into_iter().collect();
                self.run_actions(actions, tx);
            }
            // The leader opens its menu here too, so Space never changes an option.
            KeyCode::Char(c) if c == self.ed.vim.leader && vim => self.run_actions(vec![Action::LeaderMenu], tx),
            KeyCode::Char('j') | KeyCode::Down if !k.ctrl => set_row(self, cur + 1),
            KeyCode::Char('k') | KeyCode::Up if !k.ctrl => set_row(self, cur.saturating_sub(1)),
            KeyCode::Tab if k.shift => set_section(self, sec_i + SECTIONS.len() - 1),
            KeyCode::Tab | KeyCode::Char('J') => set_section(self, sec_i + 1),
            KeyCode::Char('K') => set_section(self, sec_i + SECTIONS.len() - 1),
            KeyCode::Char(c @ '1'..='5') => set_section(self, c as usize - '1' as usize),
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('l') | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
                let dir: i32 = if matches!(k.code, KeyCode::Char('h') | KeyCode::Left) { -1 } else { 1 };
                let Some(i) = controls.get(cur) else { return };
                let Item::Row { key, ctl, .. } = &list[*i] else { return };
                let key: &'static str = key;
                match ctl {
                    Ctl::Toggle(_) => toggle(&mut self.cfg, key),
                    Ctl::Step { .. } => step_by(&mut self.cfg, key, dir as f32),
                    Ctl::Seg { opts, cur } => cycle(&mut self.cfg, key, opts.iter().map(|(v, _)| v.clone()).collect(), cur, dir),
                    Ctl::Font { opts, cur } | Ctl::Themes { opts, cur } => cycle(&mut self.cfg, key, opts.clone(), cur, dir),
                    _ => {}
                }
                self.settings_changed(key);
            }
            KeyCode::Esc => {}
            _ => {
                let _ = row_key;
            }
        }
    }
}

fn cycle(cfg: &mut Config, key: &str, opts: Vec<String>, cur: &str, dir: i32) {
    if opts.is_empty() {
        return;
    }
    let i = opts.iter().position(|o| o == cur).unwrap_or(0) as i32;
    let next = (i + dir).rem_euclid(opts.len() as i32) as usize;
    pick(cfg, key, &opts[next]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_clamp_and_echo_the_toml_line() {
        let mut cfg = Config::default();
        for _ in 0..40 {
            step_by(&mut cfg, "font.size", 1.0);
        }
        assert_eq!(cfg.font.size, 24.0);
        assert_eq!(echo(&cfg, "font.size"), "font.size = 24.0");
        pick(&mut cfg, "theme", "omarchy");
        assert_eq!(echo(&cfg, "theme"), "theme.mode = \"omarchy\"");
    }
}
