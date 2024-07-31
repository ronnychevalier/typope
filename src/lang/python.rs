use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Python
    pub fn python() -> Self {
        Self {
            name: "python",
            language: tree_sitter_python::language(),
            extensions: &["py"],
            parser: Mode::Generic {
                tree_sitter_types: &["string", "concatenated_string"],
            },
        }
    }
}
