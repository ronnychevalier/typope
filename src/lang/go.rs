use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Go
    pub fn go() -> Self {
        Self {
            name: "go",
            detections: &["*.go"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_go::LANGUAGE),
                tree_sitter_types: &["interpreted_string_literal"],
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
        assert!(Language::iter().any(|lang| lang.name() == "go"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "go",
            Language::from_filename(OsStr::new("file.go"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let go = r#"
package main

import "fmt"

func f() string {
    return "abcdef"
}

func main() {
    s := "foobar"
    fmt.Println("Hello world!", s, f())
}
"#;
        let go = SharedSource::new("file.go", go.as_bytes().to_vec());
        let mut parsed = Language::go().parse(&go).unwrap();
        let strings = parsed.strings(go.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 22,
                    value: r#""fmt""#.into()
                },
                LintableString {
                    offset: 58,
                    value: r#""abcdef""#.into()
                },
                LintableString {
                    offset: 93,
                    value: r#""foobar""#.into()
                },
                LintableString {
                    offset: 118,
                    value: r#""Hello world!""#.into()
                }
            ]
        );
    }
}
