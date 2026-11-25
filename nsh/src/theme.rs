//! The app's theme: `theme.json` loaded as tokens, the code font, and the token of each
//! colour role the editor draws with. Views read colours only through `color`.

use nsh_base::{Theme, theme};
use nsh_core::prelude::*;

use editor::syntax::highlight::Kind;

/// The theme file, the one place with raw colours.
const THEME_JSON: &str = include_str!("../theme.json");

/// The tokens of `theme.json`. A root built with them draws in the editor's colours under
/// `cargo run` as well as `nsh run`.
pub fn tokens() -> Tokens {
    Theme::from_json(THEME_JSON).expect("theme.json is valid").into_tokens()
}

/// The code font: the family list, then the size in points.
pub const CODE_FAMILY: &str = "JetBrains Mono, JetBrainsMono Nerd Font, ui-monospace, monospace";
/// The editor's own font size (`font.size` in config.toml). Code text does not follow the
/// system text size, because the grid of cells is measured once from this size.
pub const CODE_SIZE: f32 = 13.0;
/// Row height over font size (`font.line_height` in config.toml).
pub const CODE_LINE_HEIGHT: f32 = 1.7;

/// The colour of a token of `theme.json`. A token the theme does not set falls back to the
/// foreground, so a typo shows as plain text and not as nothing.
pub fn color(name: &str) -> AdaptiveColor {
    let t = theme();
    t.color(name).unwrap_or(t.foreground)
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
