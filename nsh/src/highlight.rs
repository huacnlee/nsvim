//! Language names for files: what the status line shows and which language server a file
//! gets. Syntax colours come from the core (`editor::syntax::highlight`).

use std::path::Path;

/// A language, by the name language servers and the status line use (`rust`, `markdown`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Lang(&'static str);

/// Names a file can map to. Anything else is plain text.
const NAMES: &[&str] = &[
    "rust",
    "python",
    "javascript",
    "typescript",
    "tsx",
    "jsx",
    "go",
    "c",
    "cpp",
    "java",
    "ruby",
    "bash",
    "json",
    "lua",
    "toml",
    "yaml",
    "markdown",
    "html",
    "css",
    "scss",
    "makefile",
    "dockerfile",
    "cmake",
    "sql",
    "zig",
    "swift",
    "kotlin",
    "php",
    "csharp",
    "haskell",
    "ocaml",
    "elixir",
    "erlang",
    "nix",
    "vue",
    "svelte",
    "xml",
    "diff",
    "fish",
    "zsh",
];

impl Lang {
    pub const PLAIN: Lang = Lang("plain");

    pub fn from_name(name: &str) -> Option<Lang> {
        NAMES.iter().find(|n| **n == name).map(|n| Lang(n))
    }

    pub fn name(self) -> &'static str {
        self.0
    }
}

/// The language for a file, by name and extension.
pub fn lang_for(path: Option<&Path>) -> Lang {
    let Some(path) = path else { return Lang::PLAIN };
    let name = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let by_name = match name.as_str() {
        "makefile" | "gnumakefile" => Some("makefile"),
        "dockerfile" | "containerfile" => Some("dockerfile"),
        "cmakelists.txt" => Some("cmake"),
        "cargo.lock" | "uv.lock" | "poetry.lock" => Some("toml"),
        ".bashrc" | ".zshrc" | ".profile" | ".bash_profile" | "pkgbuild" => Some("bash"),
        _ => None,
    };
    if let Some(l) = by_name.and_then(Lang::from_name) {
        return l;
    }
    let ext = path.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let alias = match ext.as_str() {
        "rs" => "rust",
        "py" | "pyi" | "pyw" => "python",
        "js" | "mjs" | "cjs" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "h" => "c",
        "hpp" | "cc" | "cxx" | "hh" => "cpp",
        "rb" | "rake" | "gemspec" => "ruby",
        "sh" => "bash",
        "yml" => "yaml",
        "md" | "markdown" => "markdown",
        "htm" => "html",
        "cs" => "csharp",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "ex" | "exs" => "elixir",
        "erl" => "erlang",
        "kt" | "kts" => "kotlin",
        "conf" | "ini" => "toml",
        "jsonc" => "json",
        other => other,
    };
    Lang::from_name(alias).unwrap_or(Lang::PLAIN)
}

/// The language's name for the status line: `rust`, `markdown`, `text`.
pub fn lang_label(lang: Lang) -> &'static str {
    match lang.name() {
        "plain" => "text",
        n => n,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_pick_languages() {
        assert_eq!(lang_label(lang_for(Some(Path::new("src/main.rs")))), "rust");
        assert_eq!(lang_label(lang_for(Some(Path::new("Cargo.toml")))), "toml");
        assert_eq!(lang_label(lang_for(Some(Path::new("README.md")))), "markdown");
        assert_eq!(lang_label(lang_for(Some(Path::new(".gitignore")))), "text");
    }
}
