//! Settings: a layer over the whole window. Sections are on the left, options in the middle,
//! and a live preview and the config.toml it writes are on the right. Each change applies at
//! once and goes to config.toml at once. Esc, Ctrl+W (⌘W on macOS) and the close button close
//! it.

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
    (Section::Appearance, "Appearance"),
    (Section::Font, "Font"),
    (Section::Editor, "Editor"),
    (Section::Formatting, "Formatting"),
    (Section::Keys, "Keys & mouse"),
];

/// The fonts Settings offers. Any installed monospace font works in config.toml.
pub const FONTS: &[&str] = &["JetBrains Mono", "IBM Plex Mono", "Fira Code", "Source Code Pro", "Roboto Mono"];

#[derive(Clone, Debug, Default)]
pub struct Settings {
    pub section: Section,
    /// The row the keyboard is on, an index into the section's items.
    pub row: usize,
    /// Why the last write of config.toml failed. A good write clears it.
    pub error: Option<String>,
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
            out.push(Item::Head("Theme"));
            // An old config's "omarchy" is System now.
            let mode = if cfg.theme.mode == "omarchy" { "system" } else { cfg.theme.mode.as_str() };
            out.push(seg(
                "theme",
                "System follows your OS, or the desktop theme on Omarchy.",
                mode,
                &[("system", "System"), ("light", "Light"), ("dark", "Dark")],
            ));
            let themes = |list: &[&str]| list.iter().map(|s| s.to_string()).collect::<Vec<_>>();
            // On Omarchy, System uses the desktop theme, so its dark and light themes do not apply.
            let os_themes = mode == "system" && omarchy.is_none();
            if mode == "dark" || os_themes {
                let desc = if mode == "system" { "Used when the OS is dark" } else { "Used in dark mode" };
                out.push(Item::Row {
                    key: "theme.dark",
                    desc: desc.into(),
                    ctl: Ctl::Themes { opts: themes(theme::DARK_THEMES), cur: cfg.theme.dark.clone() },
                });
            }
            if mode == "light" || os_themes {
                let desc = if mode == "system" { "Used when the OS is light" } else { "Used in light mode" };
                out.push(Item::Row {
                    key: "theme.light",
                    desc: desc.into(),
                    ctl: Ctl::Themes { opts: themes(theme::LIGHT_THEMES), cur: cfg.theme.light.clone() },
                });
            }
            if mode == "system" {
                match omarchy {
                    Some(o) => out.push(info("system.now", "Follows the Omarchy theme", o.name.clone())),
                    None => out.push(info(
                        "system.now",
                        "What the OS asks for right now",
                        if system_dark { "Dark".into() } else { "Light".into() },
                    )),
                }
            }
            out.push(Item::Head("Window"));
            out.push(step(
                "window.opacity",
                "Below 100% lets a compositor blur show through (next launch)",
                format!("{}%", cfg.window.opacity),
            ));
        }
        Section::Font => {
            out.push(Item::Head("Editor font"));
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
                &[("400", "Regular"), ("500", "Medium"), ("700", "Bold")],
            ));
            out.push(tog("font.ligatures", "Joins -> != => into single glyphs", cfg.font.ligatures));
            out.push(Item::Head("Interface"));
            out.push(seg(
                "ui.font",
                "Tree, tabs and status line",
                &cfg.ui.font,
                &[("editor", "Same as editor"), ("system", "System sans")],
            ));
            out.push(step("ui.size", "Interface text size", format!("{} pt", fmt_num(cfg.ui.size))));
        }
        Section::Editor => {
            out.push(Item::Head("Display"));
            out.push(seg(
                "editor.numbers",
                "Hybrid shows the cursor line as absolute",
                &cfg.editor.numbers,
                &[("hybrid", "Hybrid"), ("relative", "Relative"), ("absolute", "Absolute"), ("off", "Off")],
            ));
            out.push(seg(
                "editor.cursor",
                "Insert mode always uses a bar",
                &cfg.editor.cursor,
                &[("block", "Block"), ("bar", "Bar"), ("underline", "Underline")],
            ));
            out.push(tog("editor.cursorline", "Highlight the line the cursor is on", cfg.editor.cursorline));
            out.push(seg(
                "editor.ruler",
                "A thin guide at this column",
                &cfg.editor.ruler.to_string(),
                &[("0", "Off"), ("80", "80"), ("100", "100"), ("120", "120")],
            ));
            out.push(tog("editor.whitespace", "Show leading spaces as dots", cfg.editor.whitespace));
            out.push(tog("editor.wrap", "Soft-wrap long lines", cfg.editor.wrap));
            out.push(step("editor.scrolloff", "Lines kept visible above and below the cursor", cfg.editor.scrolloff.to_string()));
            out.push(Item::Head("Indentation"));
            out.push(seg(
                "indent.width",
                "Columns per indent level. .editorconfig wins per file.",
                &cfg.indent.width.to_string(),
                &[("2", "2"), ("4", "4"), ("8", "8")],
            ));
            out.push(tog("indent.spaces", "Insert spaces when Tab is pressed", cfg.indent.spaces));
        }
        Section::Formatting => {
            out.push(Item::Head("On save"));
            out.push(tog("format.trim_whitespace", "Remove trailing spaces", cfg.format.trim_whitespace));
            out.push(tog("format.final_newline", "End every file with a newline", cfg.format.final_newline));
            out.push(tog("format.on_save", "Format with the language server", cfg.format.on_save));
            out.push(seg("format.line_endings", "Line endings for new files", &cfg.format.line_endings, &[("lf", "LF"), ("crlf", "CRLF")]));
        }
        Section::Keys => {
            out.push(Item::Head("Keys"));
            out.push(seg("keys.map", "Change any time with :set keymap", &cfg.keys.map, &[("neovim", "Neovim"), ("classic", "Classic")]));
            out.push(seg("keys.leader", "Neovim keymap only", &cfg.keys.leader, &[("space", "Space"), ("backslash", "\\")]));
            out.push(tog("keys.auto_close", "Typing ( also types )", cfg.keys.auto_close));
            out.push(Item::Head("Mouse"));
            out.push(tog("mouse.enabled", "Click, drag to select, resize the file tree", cfg.mouse.enabled));
            out.push(tog("mouse.scroll", "The wheel scrolls", cfg.mouse.scroll));
            out.push(tog("mouse.right_click", "Right-click opens a context menu", cfg.mouse.right_click));
            out.push(Item::Head("Agents"));
            out.push(tog("ipc.enabled", "Serve the ctl and mcp commands for agents and scripts (next launch)", cfg.ipc.enabled));
            out.push(tog("ipc.allow_edits", "Agents may edit buffers and run commands. Proposals work either way.", cfg.ipc.allow_edits));
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
        "format.on_save" => cfg.format.on_save ^= true,
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

    /// The io worker wrote config.toml, or failed to. Settings shows a failure in its header.
    /// In Settings, the message line keeps the echo of the change.
    pub fn config_written(&mut self, result: Result<(), String>, tx: &mut Tx) {
        match result {
            Ok(()) => {
                match self.settings.as_mut() {
                    Some(st) => st.error = None,
                    None => self.ed.message(format!("\"{}\" written", super::tilde(&crate::config::config_path()))),
                }
                self.saved_one(tx);
            }
            Err(e) => {
                if let Some(st) = self.settings.as_mut() {
                    st.error = Some(e.clone());
                }
                self.ed.error(e);
                self.quit_after_saves = None;
            }
        }
    }

    /// Applies a change made in Settings (by key or pointer), and writes config.toml.
    pub fn settings_changed(&mut self, key: &str) {
        let line = echo(&self.cfg, key);
        self.config_changed();
        self.write_config();
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
            KeyCode::Esc => self.close_settings(),
            KeyCode::Char('w' | 'W') if k.cmd => self.close_settings(),
            // Settings covers the window, so no key opens a panel under it. Ctrl+Q (⌘Q) still
            // quits in the classic keymap.
            _ if k.cmd => {
                if let Some(quit @ Action::QuitAll { .. }) = super::keys::classic_window_key(&k).filter(|_| !vim) {
                    self.run_actions(vec![quit], tx);
                }
            }
            // `:` and the leader do nothing here. Space never changes an option in Vim.
            KeyCode::Char(':') => {}
            KeyCode::Char(c) if c == self.ed.vim.leader && vim => {}
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
    fn the_theme_row_offers_no_omarchy_mode_and_shows_an_old_one_as_system() {
        let mut cfg = Config::default();
        cfg.theme.mode = "omarchy".into();
        let list = items(Section::Appearance, &cfg, None, true);
        let Some(Item::Row { ctl: Ctl::Seg { opts, cur }, .. }) = list.iter().find(|i| matches!(i, Item::Row { key: "theme", .. })) else {
            panic!("no theme row")
        };
        assert!(opts.iter().all(|(v, _)| v != "omarchy"), "{opts:?}");
        assert_eq!(cur, "system");
        assert!(list.iter().all(|i| !matches!(i, Item::Row { key, .. } if key.starts_with("omarchy."))));
        assert!(list.iter().any(|i| matches!(i, Item::Row { key: "system.now", .. })));
    }

    #[test]
    fn steps_clamp_and_echo_the_toml_line() {
        let mut cfg = Config::default();
        for _ in 0..40 {
            step_by(&mut cfg, "font.size", 1.0);
        }
        assert_eq!(cfg.font.size, 24.0);
        assert_eq!(echo(&cfg, "font.size"), "font.size = 24.0");
        pick(&mut cfg, "theme", "system");
        assert_eq!(echo(&cfg, "theme"), "theme.mode = \"system\"");
    }

    #[test]
    fn system_on_omarchy_names_the_desktop_theme_and_hides_the_os_themes() {
        let mut cfg = Config::default();
        cfg.theme.mode = "system".into();
        let keys = |o: Option<&Omarchy>| -> Vec<&str> {
            items(Section::Appearance, &cfg, o, true)
                .iter()
                .filter_map(|i| if let Item::Row { key, .. } = i { Some(*key) } else { None })
                .collect()
        };
        let o = Omarchy { name: "adventure".into(), palette: theme::builtin("dark"), dir: "/themes/adventure".into() };
        let on = keys(Some(&o));
        assert!(on.contains(&"system.now") && !on.contains(&"theme.dark") && !on.contains(&"theme.light"), "{on:?}");
        let off = keys(None);
        assert!(off.contains(&"theme.dark") && off.contains(&"theme.light"), "{off:?}");
    }
}
