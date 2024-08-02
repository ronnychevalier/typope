use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Rust
    pub fn rust() -> Self {
        Self {
            name: "rust",
            language: tree_sitter_rust::language(),
            extensions: &["rs"],
            parser: Mode::Query("(string_literal (string_content) @strings)+".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use crate::lang::LintableString;
    use crate::SharedSource;

    use super::Language;

    #[test]
    fn exists_in_iter() {
        assert!(Language::iter().any(|lang| lang.name() == "rust"));
    }

    #[test]
    fn find_from_extensions() {
        for ext in Language::rust().extensions() {
            assert_eq!(
                "rust",
                Language::from_extension(OsStr::new(ext)).unwrap().name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }

        static STR: &str = "hello";
        static RSTR: &str = r"raw raw";
        fn f() { let a = ["a", "b", "c"];}
        fn fb() { let a = ["aaaa", "bbbb", "cccc"];}
        "#;

        let rust = SharedSource::new("file.rs", rust.as_bytes().to_vec());
        let mut parsed = Language::rust().parse(&rust).unwrap();
        let strings = parsed.strings(rust.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 94,
                    value: r"failed to do something for the following reason : foobar foo".into()
                },
                LintableString {
                    offset: 197,
                    value: "hello".into()
                },
                LintableString {
                    offset: 316,
                    value: "aaaa".into()
                },
                LintableString {
                    offset: 324,
                    value: "bbbb".into()
                },
                LintableString {
                    offset: 332,
                    value: "cccc".into()
                }
            ]
        );
    }
}
