use super::{Language, Mode};

impl Language {
    /// Creates a language parser for YAML
    pub fn yaml() -> Self {
        Self {
            name: "yaml",
            detections: &["*.yml", "*.yaml"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_yaml::LANGUAGE),
                tree_sitter_types: &["double_quote_scalar"],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use crate::SharedSource;
    use crate::lang::LintableString;

    use super::Language;

    #[test]
    fn exists_in_iter() {
        assert!(Language::iter().any(|lang| lang.name() == "yaml"));
    }

    #[test]
    fn find_from_filenames() {
        for filename in ["file.yml", "file.yaml"] {
            assert_eq!(
                "yaml",
                Language::from_filename(OsStr::new(filename))
                    .unwrap()
                    .name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let yaml = r#"---
name: "foobar"
description: "a description describing : something"
field:
    - a: "abcdef"
      b: "ghijk"
      required: false
    - a: "1234"
      b: "5678"
      required: true
"#;
        let yaml = SharedSource::new("file.yml", yaml.as_bytes().to_vec());
        let mut parsed = Language::yaml().parse(&yaml).unwrap();
        let strings = parsed.strings(yaml.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 10,
                    value: r#""foobar""#.into()
                },
                LintableString {
                    offset: 32,
                    value: r#""a description describing : something""#.into()
                },
                LintableString {
                    offset: 87,
                    value: r#""abcdef""#.into()
                },
                LintableString {
                    offset: 105,
                    value: r#""ghijk""#.into()
                },
                LintableString {
                    offset: 144,
                    value: r#""1234""#.into()
                },
                LintableString {
                    offset: 160,
                    value: r#""5678""#.into()
                }
            ]
        );
    }
}
