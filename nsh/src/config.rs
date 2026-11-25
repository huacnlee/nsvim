//! `~/.config/nsvim/config.toml`: every setting the Settings buffer and onboarding change.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub theme: ThemeCfg,
    pub font: FontCfg,
    pub ui: UiCfg,
    pub window: WindowCfg,
    pub editor: EditorCfg,
    pub indent: IndentCfg,
    pub format: FormatCfg,
    pub keys: KeysCfg,
    pub mouse: MouseCfg,
    pub cli: CliCfg,
    pub ipc: IpcCfg,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeCfg {
    /// `light`, `dark` or `system`. An old `omarchy` acts as `system`.
    pub mode: String,
    /// The theme `dark` (and `system` on a dark OS) uses.
    pub dark: String,
    /// The theme `light` (and `system` on a light OS) uses.
    pub light: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontCfg {
    pub family: String,
    #[serde(serialize_with = "two_places")]
    pub size: f32,
    #[serde(serialize_with = "two_places")]
    pub line_height: f32,
    #[serde(serialize_with = "two_places")]
    pub letter_spacing: f32,
    pub weight: u16,
    pub ligatures: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    /// `editor` (the editor font) or `system` (the system sans).
    pub font: String,
    #[serde(serialize_with = "two_places")]
    pub size: f32,
    /// The file tree's width in points.
    #[serde(serialize_with = "two_places")]
    pub tree_width: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowCfg {
    /// 70 to 100. Below 100 a compositor's blur shows through.
    pub opacity: u8,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditorCfg {
    /// `hybrid`, `relative`, `absolute` or `off`.
    pub numbers: String,
    /// `block`, `bar` or `underline`. Insert mode always draws a bar.
    pub cursor: String,
    pub cursorline: bool,
    /// The ruler's column, 0 for none.
    pub ruler: u16,
    pub whitespace: bool,
    pub wrap: bool,
    pub scrolloff: u16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IndentCfg {
    pub width: u8,
    pub spaces: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FormatCfg {
    pub trim_whitespace: bool,
    pub final_newline: bool,
    /// `lf` or `crlf`, for new files.
    pub line_endings: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeysCfg {
    /// `neovim` or `classic`.
    pub map: String,
    /// `space` or `backslash`. Neovim keymap only.
    pub leader: String,
    /// Read the leader from `~/.config/nvim` on first launch.
    pub import_nvim: bool,
    /// Classic keymap: Alt-click adds a cursor.
    pub multi_cursor: bool,
    /// Typing `(` also types `)`.
    pub auto_close: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseCfg {
    pub enabled: bool,
    pub scroll: bool,
    pub click: bool,
    pub drag: bool,
    pub double_click: bool,
    pub right_click: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CliCfg {
    /// The terminal command's name, `nsvim` by default.
    pub name: String,
    pub path: bool,
    pub editor_env: bool,
    pub git: bool,
    pub default_app: bool,
    /// `vi` and `vim` in the shell open nsvim (aliases in the shell's startup file).
    pub vim_aliases: bool,
}

impl Default for ThemeCfg {
    fn default() -> Self {
        ThemeCfg { mode: "dark".into(), dark: "dark".into(), light: "light".into() }
    }
}

impl Default for FontCfg {
    fn default() -> Self {
        FontCfg { family: "JetBrains Mono".into(), size: 13.0, line_height: 1.7, letter_spacing: 0.0, weight: 400, ligatures: true }
    }
}

impl Default for UiCfg {
    fn default() -> Self {
        UiCfg { font: "editor".into(), size: 12.5, tree_width: 260.0 }
    }
}

impl Default for WindowCfg {
    fn default() -> Self {
        WindowCfg { opacity: 100 }
    }
}

impl Default for EditorCfg {
    fn default() -> Self {
        EditorCfg {
            numbers: "hybrid".into(),
            cursor: "block".into(),
            cursorline: true,
            ruler: 100,
            whitespace: false,
            wrap: false,
            scrolloff: 8,
        }
    }
}

impl Default for IndentCfg {
    fn default() -> Self {
        IndentCfg { width: 4, spaces: true }
    }
}

impl Default for FormatCfg {
    fn default() -> Self {
        FormatCfg { trim_whitespace: true, final_newline: true, line_endings: "lf".into() }
    }
}

impl Default for KeysCfg {
    fn default() -> Self {
        KeysCfg { map: "neovim".into(), leader: "space".into(), import_nvim: true, multi_cursor: true, auto_close: true }
    }
}

impl Default for MouseCfg {
    fn default() -> Self {
        MouseCfg { enabled: true, scroll: true, click: true, drag: true, double_click: true, right_click: false }
    }
}

impl Default for CliCfg {
    fn default() -> Self {
        CliCfg { name: "nsvim".into(), path: true, editor_env: true, git: true, default_app: false, vim_aliases: true }
    }
}

/// Writes an `f32` as the short decimal it was set to: `1.7`, not `1.7000000476837158`.
fn two_places<S: serde::Serializer>(v: &f32, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_f64((*v as f64 * 100.0).round() / 100.0)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IpcCfg {
    /// Serve the control socket that `nsvim ctl`, `nsvim mcp` and agents use.
    pub enabled: bool,
    /// Let `edit` and `command` requests change buffers. `propose` works either way.
    pub allow_edits: bool,
}

impl Default for IpcCfg {
    fn default() -> Self {
        IpcCfg { enabled: true, allow_edits: true }
    }
}

/// The name of the editor's folders: config, state and the control sockets.
pub const NAME: &str = "nsvim";

/// The name the folders had before the editor became nsvim. `crate::migrate` copies them.
pub const OLD_NAME: &str = "editor";

fn config_base() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn state_base() -> PathBuf {
    dirs::state_dir().or_else(dirs::data_local_dir).unwrap_or_else(|| PathBuf::from("."))
}

/// `~/.config/nsvim`.
pub fn config_dir() -> PathBuf {
    config_base().join(NAME)
}

/// `~/.config/editor`, the config folder before the rename.
pub fn old_config_dir() -> PathBuf {
    config_base().join(OLD_NAME)
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// `~/.local/state/nsvim`: the session and other files the user doesn't edit.
pub fn state_dir() -> PathBuf {
    state_base().join(NAME)
}

/// `~/.local/state/editor`, the state folder before the rename.
pub fn old_state_dir() -> PathBuf {
    state_base().join(OLD_NAME)
}

/// The folder to read from at startup: the new one, or the old one while the copy to the new
/// one has not run yet (`crate::migrate`).
pub fn readable(new: PathBuf, old: PathBuf) -> PathBuf {
    if !new.exists() && old.is_dir() { old } else { new }
}

/// What reading the config found.
pub enum Loaded {
    /// No config file yet: this is the first launch, so onboarding runs.
    FirstLaunch,
    Ok(Config),
    /// The file exists but doesn't parse. The defaults apply and the error shows once.
    Broken(Config, String),
}

/// Reads config.toml. Runs once at startup, before the first frame.
pub fn load() -> Loaded {
    let path = readable(config_dir(), old_config_dir()).join("config.toml");
    match std::fs::read_to_string(&path) {
        Err(_) => Loaded::FirstLaunch,
        Ok(src) => match parse(&path, &src) {
            Ok(cfg) => Loaded::Ok(cfg),
            Err(e) => Loaded::Broken(Config::default(), e),
        },
    }
}

/// Reads the text of the config file at `path`. The error names the file.
pub fn parse(path: &std::path::Path, src: &str) -> Result<Config, String> {
    toml::from_str::<Config>(src).map_err(|e| format!("{}: {}", path.display(), e.message()))
}

impl Config {
    pub fn to_toml(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    /// The keymap the config picks.
    pub fn keymap(&self) -> editor::editor::Keymap {
        if self.keys.map == "classic" { editor::editor::Keymap::Classic } else { editor::editor::Keymap::Vim }
    }

    pub fn leader(&self) -> char {
        if self.keys.leader == "backslash" { '\\' } else { ' ' }
    }

    /// The editing options this config sets, before `.editorconfig` adds its own.
    pub fn opts(&self) -> editor::editor::Opts {
        editor::editor::Opts {
            tab_width: self.indent.width.max(1) as usize,
            spaces: self.indent.spaces,
            scrolloff: self.editor.scrolloff as usize,
            wrap: self.editor.wrap,
            ignorecase: true,
            smartcase: true,
            auto_close: self.keys.auto_close,
            save: editor::buffer::SaveOpts {
                trim_trailing_whitespace: self.format.trim_whitespace,
                final_newline: self.format.final_newline,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_file_fills_the_rest_with_defaults() {
        let cfg: Config = toml::from_str("[theme]\nmode = \"omarchy\"\n[font]\nsize = 15.0\n").unwrap();
        assert_eq!(cfg.theme.mode, "omarchy");
        assert_eq!(cfg.theme.dark, "dark");
        assert_eq!(cfg.font.size, 15.0);
        assert_eq!(cfg.font.family, "JetBrains Mono");
    }

    #[test]
    fn floats_write_as_set() {
        assert!(Config::default().to_toml().contains("line_height = 1.7\n"));
    }

    #[test]
    fn the_default_round_trips() {
        let cfg = Config::default();
        let back: Config = toml::from_str(&cfg.to_toml()).unwrap();
        assert_eq!(back, cfg);
    }
}
