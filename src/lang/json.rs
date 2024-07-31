use super::Language;

impl Language {
    /// Creates a language parser for JSON
    pub fn json() -> Self {
        Self {
            name: "json",
            language: tree_sitter_json::language(),
            extensions: &["json"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }
}
