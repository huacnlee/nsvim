//! The languages tree-sitter covers, how files map to them, and which node kinds count as
//! functions, classes, parameter lists and comments in each grammar.

use std::path::Path;

use tree_sitter::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    C,
    Cpp,
    Java,
    Ruby,
    Bash,
    Json,
    Lua,
    Markdown,
}

/// The language of a file, by its name, then its extension.
pub fn lang_for(path: Option<&Path>) -> Option<Lang> {
    let path = path?;
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let by_name = match name.as_str() {
        ".bashrc" | ".bash_profile" | ".profile" | ".zshrc" | ".zprofile" | "pkgbuild" => Some(Lang::Bash),
        "gemfile" | "rakefile" | "guardfile" | "vagrantfile" => Some(Lang::Ruby),
        _ => None,
    };
    if by_name.is_some() {
        return by_name;
    }
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    Some(match ext.as_str() {
        "rs" => Lang::Rust,
        "py" | "pyi" | "pyw" => Lang::Python,
        "js" | "mjs" | "cjs" | "jsx" => Lang::JavaScript,
        "ts" | "mts" | "cts" => Lang::TypeScript,
        "tsx" => Lang::Tsx,
        "go" => Lang::Go,
        "c" | "h" => Lang::C,
        "cc" | "cpp" | "cxx" | "c++" | "hpp" | "hh" | "hxx" | "h++" | "ipp" => Lang::Cpp,
        "java" => Lang::Java,
        "rb" | "rake" | "gemspec" => Lang::Ruby,
        "sh" | "bash" | "zsh" | "ksh" => Lang::Bash,
        "json" | "jsonc" => Lang::Json,
        "lua" => Lang::Lua,
        "md" | "markdown" | "mkd" | "mdx" => Lang::Markdown,
        _ => return None,
    })
}

/// The language a Markdown code block names after its fence: `rust`, `py`, `sh`.
pub fn lang_named(name: &str) -> Option<Lang> {
    let name = name.trim().to_ascii_lowercase();
    let ext = match name.as_str() {
        "rust" => "rs",
        "python" => "py",
        "javascript" => "js",
        "typescript" => "ts",
        "golang" => "go",
        "ruby" => "rb",
        "shell" | "console" | "shellsession" => "sh",
        "c++" => "cpp",
        "markdown" => "md",
        other => other,
    };
    lang_for(Some(Path::new(&format!("x.{ext}"))))
}

impl Lang {
    /// The name the status line shows.
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::Python => "python",
            Lang::JavaScript => "javascript",
            Lang::TypeScript => "typescript",
            Lang::Tsx => "tsx",
            Lang::Go => "go",
            Lang::C => "c",
            Lang::Cpp => "cpp",
            Lang::Java => "java",
            Lang::Ruby => "ruby",
            Lang::Bash => "bash",
            Lang::Json => "json",
            Lang::Lua => "lua",
            Lang::Markdown => "markdown",
        }
    }

    pub fn language(self) -> Language {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Lang::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Go => tree_sitter_go::LANGUAGE.into(),
            Lang::C => tree_sitter_c::LANGUAGE.into(),
            Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Lang::Bash => tree_sitter_bash::LANGUAGE.into(),
            Lang::Json => tree_sitter_json::LANGUAGE.into(),
            Lang::Lua => tree_sitter_lua::LANGUAGE.into(),
            // The block grammar. Highlighting parses inline text with `INLINE_LANGUAGE`.
            Lang::Markdown => tree_sitter_md::LANGUAGE.into(),
        }
    }

    /// Node kinds that are functions, methods or closures.
    pub fn functions(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &["function_item", "function_signature_item", "closure_expression"],
            Lang::Python => &["function_definition", "lambda"],
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => &[
                "function_declaration",
                "function_expression",
                "function",
                "generator_function_declaration",
                "generator_function",
                "arrow_function",
                "method_definition",
                "method_signature",
                "abstract_method_signature",
                "function_signature",
            ],
            Lang::Go => &["function_declaration", "method_declaration", "func_literal"],
            Lang::C => &["function_definition"],
            Lang::Cpp => &["function_definition", "lambda_expression"],
            Lang::Java => &["method_declaration", "constructor_declaration", "compact_constructor_declaration", "lambda_expression"],
            Lang::Ruby => &["method", "singleton_method", "lambda"],
            Lang::Bash => &["function_definition"],
            Lang::Json => &[],
            Lang::Lua => &["function_declaration", "function_definition"],
            Lang::Markdown => &[],
        }
    }

    /// Node kinds that are classes, structs, enums, traits, interfaces or impl blocks.
    pub fn classes(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &["struct_item", "enum_item", "union_item", "trait_item", "impl_item"],
            Lang::Python => &["class_definition"],
            Lang::JavaScript => &["class_declaration", "class"],
            Lang::TypeScript | Lang::Tsx => &[
                "class_declaration",
                "class",
                "abstract_class_declaration",
                "interface_declaration",
                "enum_declaration",
                "type_alias_declaration",
            ],
            Lang::Go => &["type_declaration"],
            Lang::C => &["struct_specifier", "union_specifier", "enum_specifier"],
            Lang::Cpp => &["class_specifier", "struct_specifier", "union_specifier", "enum_specifier"],
            Lang::Java => {
                &["class_declaration", "interface_declaration", "enum_declaration", "record_declaration", "annotation_type_declaration"]
            }
            Lang::Ruby => &["class", "module", "singleton_class"],
            Lang::Bash | Lang::Json | Lang::Lua | Lang::Markdown => &[],
        }
    }

    /// Node kinds whose named children are parameters or arguments.
    pub fn lists(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &[
                "parameters",
                "arguments",
                "closure_parameters",
                "type_parameters",
                "type_arguments",
                "tuple_expression",
                "tuple_pattern",
                "tuple_type",
                "tuple_struct_pattern",
            ],
            Lang::Python => &["parameters", "lambda_parameters", "argument_list", "tuple", "tuple_pattern", "type_parameter"],
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => {
                &["formal_parameters", "arguments", "type_parameters", "type_arguments", "array_pattern"]
            }
            Lang::Go => &["parameter_list", "argument_list", "type_parameter_list", "type_arguments"],
            Lang::C => &["parameter_list", "argument_list"],
            Lang::Cpp => &["parameter_list", "argument_list", "template_parameter_list", "template_argument_list"],
            Lang::Java => &["formal_parameters", "argument_list", "type_parameters", "type_arguments", "inferred_parameters"],
            Lang::Ruby => &["method_parameters", "lambda_parameters", "block_parameters", "argument_list"],
            Lang::Bash | Lang::Json | Lang::Markdown => &[],
            Lang::Lua => &["parameters", "arguments"],
        }
    }

    /// Node kinds that are comments.
    pub fn comments(self) -> &'static [&'static str] {
        match self {
            Lang::Rust | Lang::Java => &["line_comment", "block_comment"],
            _ => &["comment"],
        }
    }

    /// Node kinds that sit above a function or class and belong to it when nothing parts them.
    pub fn attachments(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &["attribute_item"],
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => &["decorator"],
            Lang::Java => &["marker_annotation", "annotation"],
            Lang::Cpp => &["attribute_declaration"],
            _ => &[],
        }
    }

    /// Whether comment text documents the item below it. Go, Ruby, Bash and Python take any
    /// comment directly above; the C family takes only doc comments.
    pub fn is_doc(self, text: &str) -> bool {
        let t = text.trim_start();
        match self {
            Lang::Go | Lang::Ruby | Lang::Bash | Lang::Python => true,
            Lang::Lua => t.starts_with("---"),
            Lang::Rust => (t.starts_with("///") && !t.starts_with("////")) || t.starts_with("/**"),
            _ => t.starts_with("/**") || t.starts_with("///"),
        }
    }

    /// The marker a line comment starts with, or `None` for a block comment.
    pub fn line_marker(self, text: &str) -> Option<&'static str> {
        let t = text.trim_start();
        let markers: &[&'static str] = match self {
            Lang::Python | Lang::Ruby | Lang::Bash => &["#"],
            Lang::Lua => &["---", "--"],
            _ => &["///", "//!", "//"],
        };
        if matches!(self, Lang::Lua) && t.starts_with("--[") {
            return None;
        }
        markers.iter().copied().find(|m| t.starts_with(m))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_map_by_name_then_extension() {
        assert_eq!(lang_for(Some(Path::new("src/main.rs"))), Some(Lang::Rust));
        assert_eq!(lang_for(Some(Path::new("web/App.TSX"))), Some(Lang::Tsx));
        assert_eq!(lang_for(Some(Path::new("/home/x/.bashrc"))), Some(Lang::Bash));
        assert_eq!(lang_for(Some(Path::new("Gemfile"))), Some(Lang::Ruby));
        assert_eq!(lang_for(Some(Path::new("README.md"))), Some(Lang::Markdown));
        assert_eq!((lang_named("rust"), lang_named("sh"), lang_named("toml")), (Some(Lang::Rust), Some(Lang::Bash), None));
        assert_eq!(lang_for(Some(Path::new("notes.txt"))), None);
        assert_eq!(lang_for(None), None);
    }

    #[test]
    fn every_grammar_loads() {
        for l in [
            Lang::Rust,
            Lang::Python,
            Lang::JavaScript,
            Lang::TypeScript,
            Lang::Tsx,
            Lang::Go,
            Lang::C,
            Lang::Cpp,
            Lang::Java,
            Lang::Ruby,
            Lang::Bash,
            Lang::Json,
            Lang::Lua,
            Lang::Markdown,
        ] {
            let mut p = tree_sitter::Parser::new();
            p.set_language(&l.language()).unwrap_or_else(|e| panic!("{l:?}: {e}"));
        }
    }
}
