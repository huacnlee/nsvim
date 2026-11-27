//! `Find`: the part of the app that the find bar shows (`src/find-bar.tsx`), and the changes its
//! clicks ask for. Its keys stay in Rust (`App::find_key`).

use nsh_core::prelude::*;

use crate::app::App;
use crate::app::find::{Field, Toggle};

/// What the find bar shows.
#[derive(Debug, Clone, Default, PartialEq, Eq, JsObject)]
pub struct FindState {
    /// The bar shows.
    pub shows: bool,
    /// The text to find.
    pub query: String,
    /// The text that replaces a match.
    pub with: String,
    /// The replace field shows.
    pub replace: bool,
    /// The find field has the keys: it shows the caret.
    pub on_find: bool,
    /// The replace field has the keys.
    pub on_replace: bool,
    /// Match case.
    pub case: bool,
    /// Whole word.
    pub word: bool,
    /// Regular expression.
    pub regex: bool,
    /// `3 of 12`, `12 results`, `No results`, or empty with no query.
    pub count: String,
    /// The query finds nothing, or as a regular expression it does not compile.
    pub none: bool,
}

/// The find bar's data.
pub fn find_of(app: &App) -> FindState {
    let Some(bar) = &app.find else { return FindState::default() };
    if !app.ed.has_doc() {
        return FindState::default();
    }
    let pattern = bar.pattern();
    let (at, total) = editor::find::count(&app.ed, &pattern);
    let count = match (at, total) {
        _ if bar.query.is_empty() => String::new(),
        (_, 0) => "No results".to_string(),
        (Some(i), n) => format!("{i} of {n}"),
        (None, 1) => "1 result".to_string(),
        (None, n) => format!("{n} results"),
    };
    FindState {
        shows: true,
        query: bar.query.clone(),
        with: bar.with.clone(),
        replace: bar.replace,
        on_find: bar.keys && bar.field == Field::Find,
        on_replace: bar.keys && bar.field == Field::Replace,
        case: bar.opts.case,
        word: bar.opts.word,
        regex: bar.opts.regex,
        none: !bar.query.is_empty() && total == 0,
        count,
    }
}

/// The part of the app that the find bar shows, and the changes it asks for.
#[module]
pub struct Find;

#[module]
impl Find {
    /// Calls `listener` with what the find bar shows, now and after each change.
    pub fn on_change(#[args(find)] listener: Callback<FindState>) -> Stop {
        crate::bridge::subscribe(find_of, listener)
    }

    /// Flips a toggle: `case`, `word` or `regex`.
    pub fn toggle(name: String) {
        let t = match name.as_str() {
            "case" => Toggle::Case,
            "word" => Toggle::Word,
            "regex" => Toggle::Regex,
            _ => return,
        };
        crate::bridge::update(move |a, _| a.find_toggle(t));
    }

    /// Goes to the next match, or the one before it with `back`.
    pub fn step(back: bool) {
        crate::bridge::update(move |a, _| a.find_step(back));
    }

    /// Replaces the match at the cursor, or every match with `all`.
    pub fn replace(all: bool) {
        crate::bridge::update(move |a, _| a.find_replace(all));
    }

    /// Gives the keys to the find field, or to the replace field with `replace`.
    pub fn focus(replace: bool) {
        crate::bridge::update(move |a, _| a.find_focus(if replace { Field::Replace } else { Field::Find }));
    }

    /// Shows or hides the replace field.
    pub fn show_replace(on: bool) {
        crate::bridge::update(move |a, _| a.find_show_replace(on));
    }

    /// Closes the bar. The keys go back to the text.
    pub fn close() {
        crate::bridge::update(|a, _| a.close_find());
    }
}
