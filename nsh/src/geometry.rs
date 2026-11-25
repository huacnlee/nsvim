//! Sizes that both input handling and the views use: the code font's family list, the row
//! height and the gutter width.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::app::App;

/// The CSS family list for a configured font, with monospace fallbacks after it. Each distinct
/// name is kept for the life of the process: a handful at most, one per font the user tries.
pub fn family(name: &str) -> &'static str {
    static NAMES: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);
    let mut names = NAMES.lock().unwrap_or_else(|e| e.into_inner());
    let map = names.get_or_insert_with(HashMap::new);
    if let Some(f) = map.get(name) {
        return f;
    }
    let clean: String = name.chars().filter(|c| !matches!(c, ',' | '"' | '\'')).collect();
    let list: &'static str =
        Box::leak(format!("{clean}, JetBrainsMono Nerd Font, JetBrains Mono, ui-monospace, monospace").into_boxed_str());
    map.insert(name.to_string(), list);
    list
}

/// Rows of code are this tall: the font size times the line height, on whole points.
pub fn row_height(app: &App) -> f32 {
    (app.cfg.font.size * app.cfg.font.line_height).round().max(app.cfg.font.size + 2.0)
}

/// Gutter width in chars: 2 for signs, then numbers right-aligned with 2 chars after them.
pub fn gutter_cols(app: &App) -> usize {
    if app.cfg.editor.numbers == "off" {
        return 2;
    }
    let lines = if app.ed.has_doc() { app.ed.buf().line_count() } else { 1 };
    let digits = lines.to_string().len().max(3);
    2 + digits + 1 + 2
}
