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
    Toml,
    Yaml,
    Html,
    Css,
    Xml,
    Make,
    Dockerfile,
    Zig,
    Php,
}

/// The language of a file, by its name, then its extension.
pub fn lang_for(path: Option<&Path>) -> Option<Lang> {
    let path = path?;
    let name = path.file_name()?.to_string_lossy().to_ascii_lowercase();
    let by_name = match name.as_str() {
        ".bashrc" | ".bash_profile" | ".profile" | ".zshrc" | ".zprofile" | "pkgbuild" => Some(Lang::Bash),
        "gemfile" | "rakefile" | "guardfile" | "vagrantfile" => Some(Lang::Ruby),
        "cargo.lock" | "uv.lock" | "poetry.lock" | "pdm.lock" | "pipfile" => Some(Lang::Toml),
        "makefile" | "gnumakefile" | "bsdmakefile" => Some(Lang::Make),
        // `Dockerfile.dev`, `Containerfile.prod`. The web has no Dockerfile grammar.
        n if cfg!(not(target_arch = "wasm32"))
            && (n == "dockerfile" || n == "containerfile" || n.starts_with("dockerfile.") || n.starts_with("containerfile.")) =>
        {
            Some(Lang::Dockerfile)
        }
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
        "lua" if cfg!(not(target_arch = "wasm32")) => Lang::Lua,
        "md" | "markdown" | "mkd" | "mdx" => Lang::Markdown,
        "toml" => Lang::Toml,
        "yaml" | "yml" => Lang::Yaml,
        "html" | "htm" | "xhtml" => Lang::Html,
        "css" => Lang::Css,
        "xml" | "svg" | "xsd" | "xsl" | "xslt" | "plist" | "csproj" | "fsproj" | "vcxproj" | "props" | "xaml" => Lang::Xml,
        "mk" | "mak" => Lang::Make,
        "dockerfile" | "containerfile" if cfg!(not(target_arch = "wasm32")) => Lang::Dockerfile,
        "zig" | "zon" => Lang::Zig,
        // The web has no PHP grammar.
        "php" | "phtml" if cfg!(not(target_arch = "wasm32")) => Lang::Php,
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
        "make" | "makefile" => "mk",
        "docker" => "dockerfile",
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
            Lang::Toml => "toml",
            Lang::Yaml => "yaml",
            Lang::Html => "html",
            Lang::Css => "css",
            Lang::Xml => "xml",
            Lang::Make => "makefile",
            Lang::Dockerfile => "dockerfile",
            Lang::Zig => "zig",
            Lang::Php => "php",
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
            #[cfg(not(target_arch = "wasm32"))]
            Lang::C => tree_sitter_c::LANGUAGE.into(),
            // The web has no C grammar. The C++ grammar reads C too.
            #[cfg(target_arch = "wasm32")]
            Lang::C => tree_sitter_cpp::LANGUAGE.into(),
            Lang::Cpp => tree_sitter_cpp::LANGUAGE.into(),
            Lang::Java => tree_sitter_java::LANGUAGE.into(),
            Lang::Ruby => tree_sitter_ruby::LANGUAGE.into(),
            Lang::Bash => tree_sitter_bash::LANGUAGE.into(),
            Lang::Json => tree_sitter_json::LANGUAGE.into(),
            #[cfg(not(target_arch = "wasm32"))]
            Lang::Lua => tree_sitter_lua::LANGUAGE.into(),
            // `lang_for` gives no Lua on the web.
            #[cfg(target_arch = "wasm32")]
            Lang::Lua => tree_sitter_json::LANGUAGE.into(),
            // The block grammar. Highlighting parses inline text with `INLINE_LANGUAGE`.
            Lang::Markdown => tree_sitter_md::LANGUAGE.into(),
            Lang::Toml => tree_sitter_toml_ng::LANGUAGE.into(),
            Lang::Yaml => tree_sitter_yaml::LANGUAGE.into(),
            Lang::Html => tree_sitter_html::LANGUAGE.into(),
            Lang::Css => tree_sitter_css::LANGUAGE.into(),
            Lang::Xml => tree_sitter_xml::LANGUAGE_XML.into(),
            Lang::Make => tree_sitter_make::LANGUAGE.into(),
            #[cfg(not(target_arch = "wasm32"))]
            Lang::Dockerfile => tree_sitter_containerfile::LANGUAGE.into(),
            // `lang_for` gives no Dockerfile on the web.
            #[cfg(target_arch = "wasm32")]
            Lang::Dockerfile => tree_sitter_json::LANGUAGE.into(),
            Lang::Zig => tree_sitter_zig::LANGUAGE.into(),
            #[cfg(not(target_arch = "wasm32"))]
            Lang::Php => tree_sitter_php::LANGUAGE_PHP.into(),
            // `lang_for` gives no PHP on the web.
            #[cfg(target_arch = "wasm32")]
            Lang::Php => tree_sitter_json::LANGUAGE.into(),
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
            Lang::Zig => &["function_declaration"],
            Lang::Php => &["function_definition", "method_declaration", "anonymous_function", "arrow_function"],
            _ => &[],
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
            Lang::Zig => &["struct_declaration", "enum_declaration", "union_declaration"],
            Lang::Php => &["class_declaration", "interface_declaration", "trait_declaration", "enum_declaration"],
            _ => &[],
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
            Lang::Lua | Lang::Zig => &["parameters", "arguments"],
            Lang::Php => &["formal_parameters", "arguments"],
            _ => &[],
        }
    }

    /// Node kinds that are comments.
    pub fn comments(self) -> &'static [&'static str] {
        match self {
            Lang::Rust | Lang::Java => &["line_comment", "block_comment"],
            Lang::Css => &["comment", "js_comment"],
            Lang::Xml => &["Comment"],
            _ => &["comment"],
        }
    }

    /// Node kinds that sit above a function or class and belong to it when nothing parts them.
    pub fn attachments(self) -> &'static [&'static str] {
        match self {
            Lang::Rust => &["attribute_item"],
            Lang::JavaScript | Lang::TypeScript | Lang::Tsx => &["decorator"],
            Lang::Java => &["marker_annotation", "annotation"],
            Lang::Php => &["attribute_list"],
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
            Lang::Python | Lang::Ruby | Lang::Bash | Lang::Toml | Lang::Yaml | Lang::Make | Lang::Dockerfile => &["#"],
            // HTML, XML and CSS have only block comments.
            Lang::Html | Lang::Xml | Lang::Css => &[],
            Lang::Lua => &["---", "--"],
            Lang::Php => &["///", "//", "#"],
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
        assert_eq!((lang_named("rust"), lang_named("sh"), lang_named("toml")), (Some(Lang::Rust), Some(Lang::Bash), Some(Lang::Toml)));
        assert_eq!(
            (lang_named("yml"), lang_named("make"), lang_named("docker")),
            (Some(Lang::Yaml), Some(Lang::Make), Some(Lang::Dockerfile))
        );
        for (file, lang) in [
            ("Cargo.toml", Lang::Toml),
            ("Cargo.lock", Lang::Toml),
            ("uv.lock", Lang::Toml),
            ("poetry.lock", Lang::Toml),
            ("Pipfile", Lang::Toml),
            (".github/ci.yml", Lang::Yaml),
            ("compose.YAML", Lang::Yaml),
            ("index.html", Lang::Html),
            ("site.css", Lang::Css),
            ("icon.svg", Lang::Xml),
            ("pom.xml", Lang::Xml),
            ("Makefile", Lang::Make),
            ("GNUmakefile", Lang::Make),
            ("rules.mk", Lang::Make),
            ("Dockerfile", Lang::Dockerfile),
            ("Dockerfile.dev", Lang::Dockerfile),
            ("Containerfile", Lang::Dockerfile),
            ("build.zig", Lang::Zig),
            ("build.zig.zon", Lang::Zig),
            ("index.php", Lang::Php),
        ] {
            assert_eq!(lang_for(Some(Path::new(file))), Some(lang), "{file}");
        }
        assert_eq!(lang_for(Some(Path::new("notes.txt"))), None);
        assert_eq!(lang_for(Some(Path::new("Cargo.lock.bak"))), None);
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
            Lang::Toml,
            Lang::Yaml,
            Lang::Html,
            Lang::Css,
            Lang::Xml,
            Lang::Make,
            Lang::Dockerfile,
            Lang::Zig,
            Lang::Php,
        ] {
            let mut p = tree_sitter::Parser::new();
            p.set_language(&l.language()).unwrap_or_else(|e| panic!("{l:?}: {e}"));
        }
    }
}
