use super::{Language, Mode};

impl Language {
    /// Creates a language parser for TOML
    pub fn toml() -> Self {
        Self {
            name: "toml",
            detections: &["*.toml"],
            parser: Mode::Generic {
                language: tree_sitter_toml_ng::language(),
                tree_sitter_types: &["string"],
            },
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
        assert!(Language::iter().any(|lang| lang.name() == "toml"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "toml",
            Language::from_filename(OsStr::new("file.toml"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let toml = r#"
s = "abcd"
d = 1234 # Hehe "foobar"
s = "efgh"

# Comments
[stuff]
key = 1234
another = "ijkl"

[foo."bar"]
required = true
"#;
        let toml = SharedSource::new("file.toml", toml.as_bytes().to_vec());
        let mut parsed = Language::toml().parse(&toml).unwrap();
        let strings = parsed.strings(toml.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 5,
                    value: r#""abcd""#.into()
                },
                LintableString {
                    offset: 41,
                    value: r#""efgh""#.into()
                },
                LintableString {
                    offset: 89,
                    value: r#""ijkl""#.into()
                }
            ]
        );
    }
}
