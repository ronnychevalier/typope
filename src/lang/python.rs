use super::Language;

impl Language {
    /// Creates a language parser for Python
    pub fn python() -> Self {
        Self {
            name: "python",
            language: tree_sitter_python::language(),
            extensions: &["py"],
            tree_sitter_types: &["string", "concatenated_string"],
            parser: None,
        }
    }
}
