use super::{Language, Mode};

impl Language {
    /// Creates a language parser for C
    pub fn c() -> Self {
        Self {
            name: "c",
            language: tree_sitter_c::language(),
            extensions: &["c", "h"],
            parser: Mode::Generic {
                tree_sitter_types: &["string_content"],
            },
        }
    }
}
