use super::Language;

impl Language {
    /// Creates a language parser for Rust
    pub fn rust() -> Self {
        Self {
            name: "rust",
            language: tree_sitter_rust::language(),
            extensions: &["rs"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }
}
