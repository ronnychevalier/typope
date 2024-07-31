use super::Language;

impl Language {
    /// Creates a language parser for YAML
    pub fn yaml() -> Self {
        Self {
            name: "yaml",
            language: tree_sitter_yaml::language(),
            extensions: &["yml", "yaml"],
            tree_sitter_types: &["string_scalar"],
            parser: None,
        }
    }
}
