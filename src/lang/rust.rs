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

#[cfg(test)]
mod tests {
    use super::Language;

    #[test]
    fn lintable_strings() {
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }

        static STR: &str = "hello";

        fn f() { let a = ["a", "b", "c"];}
        fn fb() { let a = ["aaaa", "bbbb", "cccc"];}
        "#;

        let parsed = Language::rust().parse(rust).unwrap();
        let strings = parsed.strings(rust.as_bytes()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                r"failed to do something for the following reason : foobar foo",
                "hello",
                "aaaa",
                "bbbb",
                "cccc"
            ]
        );
    }
}
