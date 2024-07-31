use super::Language;

impl Language {
    /// Creates a language parser for TOML
    pub fn toml() -> Self {
        Self {
            name: "toml",
            language: tree_sitter_toml_ng::language(),
            extensions: &["toml"],
            tree_sitter_types: &["string"],
            parser: None,
        }
    }
}
