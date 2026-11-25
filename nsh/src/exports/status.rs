//! `Status`: the parts of the app that the status line shows (`ui/status-line.tsx`), and the
//! changes it asks for.
//!
//! The position changes on almost every key, so it is a part of its own. The rest changes
//! less often. `status_of` and `position_of` make the parts from the app, so tests can check
//! them without a window.

use editor::buffer::LineEnding;
use editor::editor::Mode;
use nsh_core::prelude::*;

use crate::app::App;
use crate::ui::font::UiFont;

/// The colour of a mode. Each is a theme token.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PropEnum)]
pub enum ModeColor {
    /// Normal mode: `primary`.
    #[default]
    Primary,
    /// Insert mode: `mode-insert`.
    ModeInsert,
    /// Visual mode: `mode-visual`.
    ModeVisual,
    /// Command mode, a `:s///c` confirm and the Settings tab: `mode-command`.
    ModeCommand,
    /// Replace mode: `destructive`.
    Destructive,
}

/// What the status line shows, except the position.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct StatusState {
    /// The text of the mode badge: the vim mode, or `Settings` on the Settings tab. None in the classic keymap.
    pub badge: Option<String>,
    /// The colour of the mode badge.
    pub badge_color: ModeColor,
    /// The colour of the mode. The position takes it in vim.
    pub mode_color: ModeColor,
    /// The keymap is vim.
    pub vim: bool,
    /// The git branch, when the project is a git repository.
    pub branch: Option<String>,
    /// The file relative to the project, with ` [+]` when it has unsaved changes.
    pub file: String,
    /// On the Settings tab: the theme mode and name.
    pub settings: Option<String>,
    /// The number of errors in the file.
    pub errors: u32,
    /// The number of warnings in the file.
    pub warnings: u32,
    /// The number of edits that an agent proposes.
    pub proposals: u32,
    /// The number of files that changed outside the editor.
    pub outside: u32,
    /// The language of the file. Empty when no file shows.
    pub lang: String,
    /// The encoding and the line endings, as `UTF-8 · LF`.
    pub encoding: String,
}

/// Where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct PositionState {
    /// `All`, `Top`, `Bot` or a percent. Empty when no file shows.
    pub scroll: String,
    /// The line and the column: `3:14` in vim, `Ln 3, Col 14` in the classic keymap. Empty
    /// when no file shows.
    pub pos: String,
}

/// The interface font.
#[derive(Debug, Clone, Default, PartialEq, JsObject)]
pub struct FontState {
    /// The family of the interface text, from config.toml's `[ui]`.
    pub family: String,
    /// The size of the interface text in points, from config.toml's `[ui]`.
    pub size: f32,
    /// The family of the code, from config.toml's `[font]`.
    pub mono: String,
}

/// The status line's data, except the position.
pub fn status_of(app: &App) -> StatusState {
    let vim = app.is_vim();
    let mode = app.ed.mode;
    let replacing = vim && editor::vim::replacing(&app.ed.vim);
    let confirming = vim && editor::vim::confirming(&app.ed.vim);
    let mode_color = match mode {
        _ if replacing => ModeColor::Destructive,
        _ if confirming => ModeColor::ModeCommand,
        Mode::Normal => ModeColor::Primary,
        Mode::Insert => ModeColor::ModeInsert,
        Mode::Visual(_) => ModeColor::ModeVisual,
        Mode::Command => ModeColor::ModeCommand,
    };
    // The badge shows the mode as vim writes it: `NORMAL`, `V-LINE`.
    let label = if replacing {
        "REPLACE"
    } else if confirming {
        "CONFIRM"
    } else {
        mode.label()
    };
    let mut st = StatusState { mode_color, vim, branch: app.git.branch.clone(), ..StatusState::default() };
    if app.on_settings {
        st.badge = Some("Settings".into());
        st.badge_color = ModeColor::ModeCommand;
        let dirty = if app.cfg != app.disk_cfg { " [+]" } else { "" };
        st.file = format!("config.toml{dirty}");
        let t = &app.cfg.theme;
        let resolved = crate::theme::resolve(crate::theme::Source::parse(&t.mode), &t.dark, &t.light, true, app.omarchy.as_ref()).1;
        st.settings = Some(format!("{} · {}", sentence(&t.mode), crate::theme::name(&resolved)));
        return st;
    }
    if vim {
        st.badge = Some(label.into());
        st.badge_color = mode_color;
    }
    if !app.ed.has_doc() {
        st.file = "[No Name]".into();
        return st;
    }
    let d = app.ed.doc();
    let meta = d.buf.path.as_ref().and_then(|p| app.meta.get(p));
    let name = d.buf.path.as_deref().map(|p| app.rel(p)).unwrap_or_else(|| "[No Name]".into());
    let dirty = if d.buf.is_dirty() { " [+]" } else { "" };
    let conflict = if meta.is_some_and(|m| m.conflict) { " [Changed on disk]" } else { "" };
    st.file = format!("{name}{dirty}{conflict}");
    st.lang = language_name(meta.and_then(|m| m.lang).map_or("plain", crate::highlight::lang_label)).to_string();
    let ending = match d.buf.line_ending {
        LineEnding::Lf => "LF",
        LineEnding::CrLf => "CRLF",
    };
    st.encoding = format!("UTF-8 · {ending}");
    let diags = meta.map(|m| m.diags.as_slice()).unwrap_or(&[]);
    let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    st.errors = count(diags.iter().filter(|d| d.severity <= 1).count());
    st.warnings = count(diags.iter().filter(|d| d.severity == 2).count());
    st.proposals = count(app.proposals.len());
    st.outside = count(app.outside.len());
    st
}

/// `text` with its first letter in capitals: `system` is `System`.
fn sentence(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map(|c| c.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

/// The name of a language as people write it, for the status line: `rust` is `Rust`.
pub fn language_name(id: &str) -> &'static str {
    match id {
        "rust" => "Rust",
        "python" => "Python",
        "javascript" => "JavaScript",
        "typescript" => "TypeScript",
        "tsx" => "TSX",
        "jsx" => "JSX",
        "go" => "Go",
        "c" => "C",
        "cpp" => "C++",
        "java" => "Java",
        "ruby" => "Ruby",
        "bash" => "Bash",
        "json" => "JSON",
        "lua" => "Lua",
        "toml" => "TOML",
        "yaml" => "YAML",
        "markdown" => "Markdown",
        "html" => "HTML",
        "css" => "CSS",
        "scss" => "SCSS",
        "makefile" => "Makefile",
        "dockerfile" => "Dockerfile",
        "cmake" => "CMake",
        "sql" => "SQL",
        "zig" => "Zig",
        "swift" => "Swift",
        "kotlin" => "Kotlin",
        "php" => "PHP",
        "csharp" => "C#",
        "haskell" => "Haskell",
        "ocaml" => "OCaml",
        "elixir" => "Elixir",
        "erlang" => "Erlang",
        "nix" => "Nix",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "xml" => "XML",
        "diff" => "Diff",
        "fish" => "Fish",
        "zsh" => "Zsh",
        _ => "Plain text",
    }
}

/// Where the cursor is. Both texts are empty on the Settings tab and when no file shows.
pub fn position_of(app: &App) -> PositionState {
    if app.on_settings || !app.ed.has_doc() {
        return PositionState::default();
    }
    let d = app.ed.doc();
    let total = d.buf.line_count().max(1);
    let line = d.view.cursor.line + 1;
    let scroll = if d.view.top == 0 && total <= app.ed.rows {
        "All".into()
    } else if line <= 1 {
        "Top".into()
    } else if line >= total {
        "Bot".into()
    } else {
        format!("{}%", line * 100 / total)
    };
    let col = d.view.cursor.col + 1;
    let pos = if app.is_vim() { format!("{line}:{col}") } else { format!("Ln {line}, Col {col}") };
    PositionState { scroll, pos }
}

/// The interface font of `app`.
pub fn font_of(app: &App) -> FontState {
    let f = UiFont::of(app);
    FontState { family: f.family.into(), size: f.size, mono: f.mono.into() }
}

/// The parts of the app that the status line shows, and the changes it asks for.
#[module]
pub struct Status;

#[module]
impl Status {
    /// Calls `listener` with what the status line shows, except the position, now and after
    /// each change.
    pub fn on_change(#[args(status)] listener: Callback<StatusState>) -> Stop {
        crate::bridge::subscribe(status_of, listener)
    }

    /// Calls `listener` with where the cursor is, now and after each change.
    pub fn on_position(#[args(position)] listener: Callback<PositionState>) -> Stop {
        crate::bridge::subscribe(position_of, listener)
    }

    /// Calls `listener` with the interface font, now and after each change.
    pub fn on_font(#[args(font)] listener: Callback<FontState>) -> Stop {
        crate::bridge::subscribe(font_of, listener)
    }

    /// Writes the settings to config.toml.
    pub fn write_config() {
        crate::bridge::update(|a, _| a.write_config());
    }
}
