//! The look: the palette as NSH theme tokens, the colour of a token, and the code font.
//!
//! Views read colours only through `color(token)`. The palette comes from `crate::theme`
//! (built-in themes, Omarchy, light / dark / system) and turns into tokens here, so a theme
//! change restyles every view natively with `set_tokens`.
//!
//! The tokens, by role:
//!
//! | Token | Palette | Role |
//! | --- | --- | --- |
//! | `background` | bg | The text area, the active tab |
//! | `muted` | bg2 | Chrome: tab bar, status line, tree, popups |
//! | `surface-raised` | bg3 | The cursor line, hovers, the picked row |
//! | `border` | bd | Lines between areas, `~` rows, whitespace dots |
//! | `foreground` | fg | Text |
//! | `muted-foreground` | fg2 | Secondary text |
//! | `text-faint` | fg3 | Line numbers, hints |
//! | `primary` | ac | Accent, Normal mode, folders, the caret |
//! | `primary-foreground` | on | Text on an accent fill |
//! | `mode-insert`, `mode-visual`, `mode-command` | ins, vis, cmd | Mode colours |
//! | `code-*` | kw fun str num ty mac com pun prop | Syntax colours |
//! | `warning`, `destructive` | warn, err | Warnings and errors |
//! | `selection` | sel | Selected text |

use editor::syntax::highlight::Kind;
use nsh_base::{Theme, theme};
use nsh_core::prelude::*;

use crate::color::Color;
use crate::theme::Palette;

/// The theme file: radius and the shadcn roles `nsh run` falls back to.
const THEME_JSON: &str = include_str!("../../../../theme.json");

fn adaptive(c: Color) -> AdaptiveColor {
    let rgba = Rgba { r: c.r, g: c.g, b: c.b, a: c.a };
    AdaptiveColor::new(rgba, rgba)
}

/// The tokens for `p`. The palette is already the one for the light or dark appearance, so
/// both sides of each token are the same colour.
pub fn tokens(p: &Palette) -> Tokens {
    let mut t = Theme::from_json(THEME_JSON).expect("theme.json is valid");
    let roles: [(&str, Color); 33] = [
        ("background", p.bg),
        ("muted", p.bg2),
        ("card", p.bg2),
        ("popover", p.bg2),
        ("surface-raised", p.bg3),
        ("secondary", p.bg3),
        ("accent", p.bg3),
        ("border", p.bd),
        ("input", p.bd),
        ("foreground", p.fg),
        ("card-foreground", p.fg),
        ("popover-foreground", p.fg),
        ("secondary-foreground", p.fg),
        ("accent-foreground", p.fg),
        ("muted-foreground", p.fg2),
        ("text-faint", p.fg3),
        ("primary", p.ac),
        ("ring", p.ac),
        ("primary-foreground", p.on),
        ("mode-insert", p.ins),
        ("mode-visual", p.vis),
        ("mode-command", p.cmd),
        ("code-keyword", p.kw),
        ("code-function", p.fun),
        ("code-string", p.str_),
        ("code-number", p.num),
        ("code-type", p.ty),
        ("code-macro", p.mac),
        ("code-comment", p.com),
        ("code-punctuation", p.pun),
        ("code-property", p.prop),
        ("warning", p.warn),
        ("destructive", p.err),
    ];
    for (name, c) in roles {
        t.set_color(name, adaptive(c));
    }
    t.set_color("selection", adaptive(p.sel));
    t.into_tokens()
}

/// The colour of a token. A token the theme does not set falls back to the foreground, so a
/// typo shows as plain text and not as nothing.
pub fn color(name: &str) -> AdaptiveColor {
    let t = theme();
    t.color(name).unwrap_or(t.foreground)
}

/// A colour that is data, not a role: a theme's swatch in Settings.
pub fn swatch(c: Color) -> AdaptiveColor {
    adaptive(c)
}

/// The token for a kind of code.
pub fn code_token(kind: Option<Kind>) -> &'static str {
    match kind {
        Some(Kind::Keyword) => "code-keyword",
        Some(Kind::Function) => "code-function",
        Some(Kind::Type) => "code-type",
        Some(Kind::String) => "code-string",
        Some(Kind::Number) => "code-number",
        Some(Kind::Comment) => "code-comment",
        Some(Kind::Macro) => "code-macro",
        Some(Kind::Property) => "code-property",
        Some(Kind::Punct) => "code-punctuation",
        Some(Kind::Text) | None => "foreground",
    }
}

/// The code font of a session, from config.toml: family list, size and row height. Code text
/// does not follow the system text size, because the grid of cells is measured from it.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeFont {
    pub family: &'static str,
    pub size: f32,
    pub row_h: f32,
    pub weight: u16,
}

/// The border of a box with a title in its top edge. It is the box's first child, so the title,
/// a later child, paints over the line. A border on the box itself paints over its children on
/// some platforms, and the line then crosses the title.
pub fn frame(radius: f32) -> View {
    view().absolute().inset(px(0.0)).rounded(px(radius)).border_1().border_color(move || color("border"))
}

/// No fill: a token at zero opacity, for a state that shows nothing.
pub fn clear() -> AdaptiveColor {
    color("background").opacity(0.0)
}
