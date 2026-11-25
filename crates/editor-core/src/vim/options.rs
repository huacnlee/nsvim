//! `:set`: the options the core owns, and the ones it hands to the app.

use crate::editor::{Action, Editor};

type Res = Result<(), String>;

/// `:set`: applies the options the core owns, and hands every option to the app. For an option
/// the app owns, `name?` and `name!` reach it as the values `?` and `!`.
pub(crate) fn set(ed: &mut Editor, args: &str, out: &mut Vec<Action>) -> Res {
    for arg in args.split_whitespace() {
        if let Some(name) = arg.strip_suffix('?') {
            match core_value(ed, canonical(name)) {
                Some(v) => ed.message(format!("  {name}={v}")),
                None => out.push(Action::SetOption(canonical(name).to_string(), "?".into())),
            }
            continue;
        }
        let (name, value) = if let Some((k, v)) = arg.split_once('=') {
            (k, v.to_string())
        } else if let Some(k) = arg.strip_suffix('!') {
            match core_value(ed, canonical(k)) {
                Some(v) if v == "true" || v == "false" => (k, (v == "false").to_string()),
                Some(_) => return Err(format!("E488: Trailing characters: {arg}")),
                None => (k, "!".to_string()),
            }
        } else if let Some(k) = arg.strip_prefix("no").filter(|k| is_bool(k)) {
            (k, "false".to_string())
        } else {
            (arg, "true".to_string())
        };
        let name = canonical(name);
        match name {
            "tabstop" | "shiftwidth" | "softtabstop" => {
                let n: usize = value.parse().map_err(|_| format!("E521: Number required after =: {arg}"))?;
                ed.opts.tab_width = n.clamp(1, 16);
            }
            "expandtab" => ed.opts.spaces = value == "true",
            "scrolloff" => ed.opts.scrolloff = value.parse().map_err(|_| format!("E521: Number required after =: {arg}"))?,
            "wrap" => ed.opts.wrap = value == "true",
            "hlsearch" => ed.vim.hlsearch = value == "true",
            "ignorecase" => ed.opts.ignorecase = value == "true",
            "smartcase" => ed.opts.smartcase = value == "true",
            _ => {}
        }
        out.push(Action::SetOption(name.to_string(), value));
    }
    Ok(())
}

/// The value of an option the core owns, for `name?` and `name!`.
fn core_value(ed: &Editor, name: &str) -> Option<String> {
    Some(match name {
        "tabstop" | "shiftwidth" | "softtabstop" => ed.opts.tab_width.to_string(),
        "expandtab" => ed.opts.spaces.to_string(),
        "scrolloff" => ed.opts.scrolloff.to_string(),
        "wrap" => ed.opts.wrap.to_string(),
        "hlsearch" => ed.vim.hlsearch.to_string(),
        "ignorecase" => ed.opts.ignorecase.to_string(),
        "smartcase" => ed.opts.smartcase.to_string(),
        _ => return None,
    })
}

fn is_bool(name: &str) -> bool {
    matches!(
        canonical(name),
        "expandtab"
            | "wrap"
            | "number"
            | "relativenumber"
            | "cursorline"
            | "list"
            | "hlsearch"
            | "ignorecase"
            | "smartcase"
            | "mouse"
            | "ligatures"
    )
}

fn canonical(name: &str) -> &str {
    match name {
        "ts" => "tabstop",
        "sw" => "shiftwidth",
        "sts" => "softtabstop",
        "et" => "expandtab",
        "so" => "scrolloff",
        "nu" => "number",
        "rnu" => "relativenumber",
        "cul" => "cursorline",
        "hls" => "hlsearch",
        "ic" => "ignorecase",
        "scs" => "smartcase",
        n => n,
    }
}
