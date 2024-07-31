use super::{Language, Mode};

impl Language {
    /// Creates a language parser for TOML
    pub fn toml() -> Self {
        Self {
            name: "toml",
            language: tree_sitter_toml_ng::language(),
            extensions: &["toml"],
            parser: Mode::Generic {
                tree_sitter_types: &["string"],
            },
        }
    }
}
