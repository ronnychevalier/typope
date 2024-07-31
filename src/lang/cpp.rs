use super::{Language, Mode};

impl Language {
    /// Creates a language parser for C++
    pub fn cpp() -> Self {
        Self {
            name: "cpp",
            language: tree_sitter_cpp::language(),
            extensions: &["cpp", "cc", "hpp", "hh"],
            parser: Mode::Generic {
                tree_sitter_types: &["string_content"],
            },
        }
    }
}
