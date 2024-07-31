use super::Language;

impl Language {
    /// Creates a language parser for C
    pub fn c() -> Self {
        Self {
            name: "c",
            language: tree_sitter_c::language(),
            extensions: &["c", "h"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }
}
