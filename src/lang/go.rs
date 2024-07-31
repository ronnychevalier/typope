use super::Language;

impl Language {
    /// Creates a language parser for Go
    pub fn go() -> Self {
        Self {
            name: "go",
            language: tree_sitter_go::language(),
            extensions: &["go"],
            tree_sitter_types: &["interpreted_string_literal"],
            parser: None,
        }
    }
}