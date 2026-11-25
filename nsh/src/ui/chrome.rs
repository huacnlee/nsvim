//! The status line (mode, git branch, file, diagnostics, language, position) and the command
//! line under it (the `:` input with its caret, messages and errors, pending keys).
//!
//! Each line reads one memo of plain data. `status_of` and `command_of` make that data from
//! the app, so tests can check it without a window.

use editor::editor::{Mode, VisualKind};
use nsh_core::prelude::*;

use crate::app::{App, Overlay};
use crate::store::Session;
use crate::ui::icons::{Icon, Lucide, icon, src};
use crate::ui::style::color;

/// The interface font: the family and size from config.toml's `[ui]`, and the editor family
/// for the command line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiFont {
    pub family: &'static str,
    pub size: f32,
    pub mono: &'static str,
}

/// A memo of the interface font.
pub fn ui_font(s: Session) -> Memo<UiFont> {
    s.view(|a| UiFont {
        family: if a.cfg.ui.font == "system" { "system-ui" } else { a.font_family },
        size: a.cfg.ui.size,
        mono: a.font_family,
    })
}

/// One line of interface text, `delta` points larger than the configured size. The chrome
/// has fixed heights, so its text follows config.toml's size and not the system text size.
pub fn ui_text(font: Memo<UiFont>, delta: f32, content: impl IntoReactive<SharedString>) -> Text {
    text(content)
        .font_family(move || SharedString::from(font.get().family))
        .text_size(move || px((font.get().size + delta).max(9.0)))
        .number_of_lines(1)
        .allow_font_scaling(false)
}

/// No fill: a token at zero opacity, for a state that shows nothing.
pub fn clear() -> AdaptiveColor {
    color("background").opacity(0.0)
}

/// One line of text in the editor family at `size`.
fn mono_text(font: Memo<UiFont>, size: f32, content: impl IntoReactive<SharedString>) -> Text {
    text(content).font_family(move || SharedString::from(font.get().mono)).text_size(px(size)).number_of_lines(1).allow_font_scaling(false)
}

/// What the status line shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StatusInfo {
    /// The mode badge and its colour token: the vim mode, or `SETTINGS`. None in the classic
    /// keymap.
    pub badge: Option<(&'static str, &'static str)>,
    /// The token of the mode's colour. The position takes it in vim.
    pub mode_token: &'static str,
    pub vim: bool,
    pub branch: Option<String>,
    /// The file relative to the project, with ` [+]` when it has unsaved changes.
    pub file: String,
    /// On the Settings tab: the theme mode and name.
    pub settings: Option<String>,
    pub errors: usize,
    pub warnings: usize,
    pub proposals: usize,
    pub outside: usize,
    pub lang: String,
    pub encoding: String,
    /// `All`, `Top`, `Bot` or a percent.
    pub scroll: String,
    pub pos: String,
}

/// The status line's data.
pub fn status_of(app: &App) -> StatusInfo {
    let vim = app.is_vim();
    let mode = app.ed.mode;
    let replacing = vim && editor::vim::replacing(&app.ed.vim);
    let confirming = vim && editor::vim::confirming(&app.ed.vim);
    let mode_token = match mode {
        _ if replacing => "destructive",
        _ if confirming => "mode-command",
        Mode::Normal => "primary",
        Mode::Insert => "mode-insert",
        Mode::Visual(_) => "mode-visual",
        Mode::Command => "mode-command",
    };
    let label = if replacing {
        "REPLACE"
    } else if confirming {
        "CONFIRM"
    } else {
        mode.label()
    };
    let mut st = StatusInfo { mode_token, vim, branch: app.git.branch.clone(), ..StatusInfo::default() };
    if app.on_settings {
        st.badge = Some(("SETTINGS", "mode-command"));
        let dirty = if app.cfg != app.disk_cfg { " [+]" } else { "" };
        st.file = format!("config.toml{dirty}");
        let t = &app.cfg.theme;
        let resolved = crate::theme::resolve(crate::theme::Source::parse(&t.mode), &t.dark, &t.light, true, app.omarchy.as_ref()).1;
        st.settings = Some(format!("{} · {}", t.mode, crate::theme::name(&resolved)));
        return st;
    }
    if vim {
        st.badge = Some((label, mode_token));
    }
    if !app.ed.has_doc() {
        st.file = "[No Name]".into();
        return st;
    }
    let d = app.ed.doc();
    let meta = d.buf.path.as_ref().and_then(|p| app.meta.get(p));
    let name = d.buf.path.as_deref().map(|p| app.rel(p)).unwrap_or_else(|| "[No Name]".into());
    let dirty = if d.buf.is_dirty() { " [+]" } else { "" };
    let conflict = if meta.is_some_and(|m| m.conflict) { " [changed on disk]" } else { "" };
    st.file = format!("{name}{dirty}{conflict}");
    st.lang = meta.and_then(|m| m.lang).map(crate::highlight::lang_label).unwrap_or("text").to_string();
    st.encoding = format!("utf-8 · {}", d.buf.line_ending.label());
    let total = d.buf.line_count().max(1);
    let line = d.view.cursor.line + 1;
    st.scroll = if d.view.top == 0 && total <= app.ed.rows {
        "All".into()
    } else if line <= 1 {
        "Top".into()
    } else if line >= total {
        "Bot".into()
    } else {
        format!("{}%", line * 100 / total)
    };
    let col = d.view.cursor.col + 1;
    st.pos = if vim { format!("{line}:{col}") } else { format!("Ln {line}, Col {col}") };
    let diags = meta.map(|m| m.diags.as_slice()).unwrap_or(&[]);
    st.errors = diags.iter().filter(|d| d.severity <= 1).count();
    st.warnings = diags.iter().filter(|d| d.severity == 2).count();
    st.proposals = app.proposals.len();
    st.outside = app.outside.len();
    st
}

/// A text of the status line in the secondary colour, shown while it is not empty.
fn tail(st: Memo<StatusInfo>, font: Memo<UiFont>, token: &'static str, id: &'static str, f: fn(&StatusInfo) -> String) -> impl IntoElement {
    tail_with(st, font, token, id, None, f)
}

/// A status line part with an icon before its text.
fn tail_with(
    st: Memo<StatusInfo>,
    font: Memo<UiFont>,
    token: &'static str,
    id: &'static str,
    mark: Option<Icon>,
    f: fn(&StatusInfo) -> String,
) -> impl IntoElement {
    let value = computed(move || st.with(f));
    when(
        move || value.with(|v| !v.is_empty()),
        move || {
            let mut part = view().flex_row().items_center().gap(px(5.0)).px_3().text_color(move || color(token));
            if let Some(m) = mark {
                part = part.child(icon(src(m)).size(move || px(font.get().size - 1.0)).test_id(&format!("{id}-icon")));
            }
            part.child(ui_text(font, -0.5, move || value.get()).text_color(move || color(token)).test_id(id))
        },
    )
}

pub fn status_line(s: Session) -> impl IntoElement {
    let st = s.view(status_of);
    let font = ui_font(s);
    let badge = computed(move || st.with(|s| s.badge));
    let mode_token = move || st.with(|s| s.mode_token);
    view()
        .flex_row()
        .items_center()
        .h(px(24.0))
        .flex_shrink_0()
        .overflow_hidden()
        .bg(move || color("muted"))
        .test_id("status-line")
        .child(when(
            move || badge.with(Option::is_some),
            move || {
                view()
                    .px_3()
                    .self_stretch()
                    .justify_center()
                    .bg(move || color(badge.get().map_or("primary", |b| b.1)))
                    .test_id("mode-badge")
                    .child(
                        ui_text(font, -0.5, move || badge.get().map_or("", |b| b.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(move || color("primary-foreground"))
                            .test_id("mode"),
                    )
            },
        ))
        .child(when(
            move || st.with(|s| s.branch.is_some()),
            move || {
                view()
                    .flex_row()
                    .self_stretch()
                    .items_center()
                    .px_3()
                    .gap(px(6.0))
                    .bg(move || color("surface-raised"))
                    .child(
                        icon(src(Lucide::GitBranch))
                            .size(move || px(font.get().size - 1.0))
                            .text_color(move || color("muted-foreground"))
                            .test_id("branch-icon"),
                    )
                    .child(
                        ui_text(font, -0.5, move || st.with(|s| s.branch.clone().unwrap_or_default()))
                            .text_color(move || color("foreground"))
                            .test_id("branch"),
                    )
            },
        ))
        .child(
            ui_text(font, -0.5, move || st.with(|s| s.file.clone()))
                .px_3()
                .flex_shrink(1.0)
                .truncate()
                .text_color(move || color("foreground"))
                .test_id("file"),
        )
        .child(view().flex_1())
        .child(when(move || st.with(|s| s.settings.is_some()), move || settings_end(s, st, font)))
        .child(tail(st, font, "destructive", "errors", |s| if s.errors > 0 { format!("E {}", s.errors) } else { String::new() }))
        .child(tail(st, font, "warning", "warnings", |s| if s.warnings > 0 { format!("W {}", s.warnings) } else { String::new() }))
        .child(tail_with(st, font, "mode-visual", "proposals", Some(Lucide::GitCompareArrows), |s| match s.proposals {
            0 => String::new(),
            1 => "1 proposal".into(),
            n => format!("{n} proposals"),
        }))
        .child(tail_with(st, font, "mode-visual", "outside", Some(Lucide::Diff), |s| {
            if s.outside > 0 { s.outside.to_string() } else { String::new() }
        }))
        .child(tail(st, font, "muted-foreground", "language", |s| s.lang.clone()))
        .child(tail(st, font, "muted-foreground", "encoding", |s| if s.lang.is_empty() { String::new() } else { s.encoding.clone() }))
        .child(tail(st, font, "muted-foreground", "scroll", |s| if s.lang.is_empty() { String::new() } else { s.scroll.clone() }))
        .child(when(
            move || st.with(|s| !s.pos.is_empty()),
            move || {
                let vim = computed(move || st.with(|s| s.vim));
                view()
                    .self_stretch()
                    .justify_center()
                    .px_3()
                    .bg(move || color(if vim.get() { mode_token() } else { "surface-raised" }))
                    .child(dynamic(move || {
                        let pos = ui_text(font, -0.5, move || st.with(|s| s.pos.clone())).test_id("position");
                        if vim.get() {
                            pos.font_weight(FontWeight::BOLD).text_color(move || color("primary-foreground"))
                        } else {
                            pos.text_color(move || color("foreground"))
                        }
                    }))
            },
        ))
}

/// The end of the status line on the Settings tab: the theme, and `:w` to write config.toml.
fn settings_end(s: Session, st: Memo<StatusInfo>, font: Memo<UiFont>) -> impl IntoElement {
    view()
        .flex_row()
        .self_stretch()
        .items_center()
        .child(
            ui_text(font, -0.5, move || st.with(|s| s.settings.clone().unwrap_or_default()))
                .px_3()
                .text_color(move || color("muted-foreground")),
        )
        .child(
            view()
                .self_stretch()
                .justify_center()
                .px_3()
                .bg(move || color("primary"))
                .cursor(Cursor::Pointer)
                .role(AccessibilityRole::Button)
                .label("Write config.toml")
                .on_press(move || s.update(|a, _| a.write_config()))
                .child(ui_text(font, -0.5, ":w").font_weight(FontWeight::BOLD).text_color(move || color("primary-foreground"))),
        )
}

/// How a command line message looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tone {
    /// A message or the file's size: secondary text.
    #[default]
    Quiet,
    /// An error.
    Error,
    /// The mode, as `-- INSERT --`: bold.
    Mode,
}

/// What shows at the end of the command line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Hint {
    #[default]
    None,
    /// The leader menu is open.
    Leader,
    /// Keys typed so far of a command, as vim's `showcmd`.
    Pending(String),
    /// A button that opens the leader menu.
    Keys,
}

/// What the command line shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandInfo {
    /// The line shows: always in vim, in the classic keymap only for a command or a message.
    pub shows: bool,
    /// While typing a command: the text before the caret (with the `:`) and after it.
    pub input: Option<(String, String)>,
    pub text: String,
    pub tone: Tone,
    pub hint: Hint,
}

/// The command line's data.
pub fn command_of(app: &App) -> CommandInfo {
    let shows = app.is_vim() || app.ed.mode == Mode::Command || app.ed.msg.is_some();
    let mut info = CommandInfo { shows, ..CommandInfo::default() };
    if app.ed.mode == Mode::Command
        && let Some(c) = &app.ed.cmd
    {
        let before: String = c.text.chars().take(c.caret).collect();
        let after: String = c.text.chars().skip(c.caret).collect();
        info.input = Some((format!("{}{before}", c.kind), after));
        return info;
    }
    let vim = app.is_vim();
    (info.text, info.tone) = if let Some(m) = &app.ed.msg {
        (m.text.clone(), if m.error { Tone::Error } else { Tone::Quiet })
    } else if app.on_settings {
        (":Settings".into(), Tone::Quiet)
    } else {
        match app.ed.mode {
            Mode::Insert if vim && editor::vim::replacing(&app.ed.vim) => ("-- REPLACE --".into(), Tone::Mode),
            Mode::Insert if vim => ("-- INSERT --".into(), Tone::Mode),
            Mode::Visual(VisualKind::Line) => ("-- VISUAL LINE --".into(), Tone::Mode),
            Mode::Visual(VisualKind::Block) => ("-- VISUAL BLOCK --".into(), Tone::Mode),
            Mode::Visual(_) => ("-- VISUAL --".into(), Tone::Mode),
            _ if app.ed.has_doc() => {
                let d = app.ed.doc();
                let name = d.buf.path.as_deref().map(|p| app.rel(p)).unwrap_or_else(|| "[No Name]".into());
                (format!("\"{name}\" {}L, {}", d.buf.line_count(), crate::app::human_bytes(d.buf.len_bytes())), Tone::Quiet)
            }
            _ => (String::new(), Tone::Quiet),
        }
    };
    let pending = editor::vim::showcmd(&app.ed.vim);
    info.hint = if matches!(app.overlay, Overlay::Leader(_)) {
        Hint::Leader
    } else if !pending.is_empty() {
        Hint::Pending(pending)
    } else if vim && matches!(app.overlay, Overlay::None) && app.ed.vim.leader == ' ' {
        Hint::Keys
    } else {
        Hint::None
    };
    info
}

/// The size of the command line's text.
const CMD_SIZE: f32 = 12.5;

pub fn command_line(s: Session) -> impl IntoElement {
    let info = s.view(command_of);
    let font = ui_font(s);
    let input = computed(move || info.with(|i| i.input.clone()));
    when(
        move || info.with(|i| i.shows),
        move || {
            view()
                .flex_row()
                .items_center()
                .h(px(22.0))
                .flex_shrink_0()
                .px_2()
                .overflow_hidden()
                .bg(move || color("background"))
                .test_id("command-line")
                .child(
                    when(
                        move || input.with(Option::is_some),
                        move || {
                            let part = move |first: bool| {
                                move || {
                                    input.with(|i| i.as_ref().map(|(a, b)| if first { a.clone() } else { b.clone() }).unwrap_or_default())
                                }
                            };
                            view()
                                .flex_row()
                                .flex_1()
                                .items_center()
                                .child(
                                    mono_text(font, CMD_SIZE, part(true)).text_color(move || color("foreground")).test_id("command-input"),
                                )
                                .child(view().w(px(1.5)).h(px(15.0)).bg(move || color("primary")).test_id("command-caret"))
                                .child(
                                    mono_text(font, CMD_SIZE, part(false)).text_color(move || color("foreground")).test_id("command-after"),
                                )
                        },
                    )
                    .otherwise(move || message(s, info, font)),
                )
        },
    )
}

/// A message, an error or the mode, then the hint at the end.
fn message(s: Session, info: Memo<CommandInfo>, font: Memo<UiFont>) -> impl IntoElement {
    let tone = computed(move || info.with(|i| i.tone));
    let hint = computed(move || info.with(|i| i.hint.clone()));
    let body = move || info.with(|i| i.text.clone());
    view()
        .flex_row()
        .flex_1()
        .items_center()
        .child(dynamic(move || match tone.get() {
            Tone::Error => {
                mono_text(font, CMD_SIZE, body).flex_shrink(1.0).truncate().text_color(move || color("destructive")).test_id("error")
            }
            Tone::Mode => mono_text(font, CMD_SIZE, body)
                .flex_shrink(1.0)
                .truncate()
                .font_weight(FontWeight::BOLD)
                .text_color(move || color("foreground"))
                .test_id("message"),
            Tone::Quiet => {
                mono_text(font, CMD_SIZE, body).flex_shrink(1.0).truncate().text_color(move || color("muted-foreground")).test_id("message")
            }
        }))
        .child(view().flex_1())
        .child(dynamic(move || match hint.get() {
            Hint::None => view().into_element(),
            Hint::Leader => mono_text(font, CMD_SIZE, "<space>").text_color(move || color("foreground")).test_id("pending").into_element(),
            Hint::Pending(p) => mono_text(font, CMD_SIZE, p).text_color(move || color("foreground")).test_id("pending").into_element(),
            Hint::Keys => keys_button(s, font).into_element(),
        }))
}

fn keys_button(s: Session, font: Memo<UiFont>) -> impl IntoElement {
    view()
        .self_stretch()
        .justify_center()
        .px_1()
        .cursor(Cursor::Pointer)
        .hover(|st| st.bg(move || color("surface-raised")))
        .role(AccessibilityRole::Button)
        .label("Show the keys")
        .on_press(move || s.update(|a, _| a.overlay = Overlay::Leader(None)))
        .child(mono_text(font, 12.0, "<space> for keys").text_color(move || color("text-faint")))
}
