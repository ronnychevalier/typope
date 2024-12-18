use super::{Language, Mode};

impl Language {
    /// Creates a language parser for JSON
    pub fn json() -> Self {
        Self {
            name: "json",
            extensions: &["json"],
            parser: Mode::Generic {
                language: tree_sitter_json::language(),
                tree_sitter_types: &["string_content"],
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
        assert!(Language::iter().any(|lang| lang.name() == "json"));
    }

    #[test]
    fn find_from_extensions() {
        for ext in Language::json().extensions() {
            assert_eq!(
                "json",
                Language::from_extension(OsStr::new(ext)).unwrap().name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let json = r#"
{
    "field": "content",
    "another_field": 1234,
    "dict": {
        "boolean": false,
        "array": ["data"]
    }
}
"#;
        let json = SharedSource::new("file.json", json.as_bytes().to_vec());
        let mut parsed = Language::json().parse(&json).unwrap();
        let strings = parsed.strings(json.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 8,
                    value: "field".into()
                },
                LintableString {
                    offset: 17,
                    value: "content".into()
                },
                LintableString {
                    offset: 32,
                    value: "another_field".into()
                },
                LintableString {
                    offset: 59,
                    value: "dict".into()
                },
                LintableString {
                    offset: 77,
                    value: "boolean".into()
                },
                LintableString {
                    offset: 103,
                    value: "array".into()
                },
                LintableString {
                    offset: 113,
                    value: "data".into()
                }
            ]
        );
    }
}
