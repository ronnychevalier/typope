use super::Language;

impl Language {
    /// Creates a language parser for C++
    pub fn cpp() -> Self {
        Self {
            name: "cpp",
            language: tree_sitter_cpp::language(),
            extensions: &["cpp", "cc", "hpp", "hh"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }
}
