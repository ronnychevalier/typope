use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Kotlin
    pub fn kotlin() -> Self {
        Self {
            name: "kotlin",
            detections: &["*.kt", "*.kts"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_kotlin_ng::LANGUAGE),
                tree_sitter_types: &["string_content"],
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
        assert!(Language::iter().any(|lang| lang.name() == "kotlin"));
    }

    #[test]
    fn find_from_filenames() {
        for filename in ["file.kt", "file.kts"] {
            assert_eq!(
                "kotlin",
                Language::from_filename(OsStr::new(filename))
                    .unwrap()
                    .name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let kotlin = r#"
package org.kotlinlang.play

fun f(): String {
    return "abcdef"
}

fun main() {
    var s = "foobar"
    println("Hello, World! ($s) ghijkl")
}
"#;
        let kotlin = SharedSource::new("file.kt", kotlin.as_bytes().to_vec());
        let mut parsed = Language::kotlin().parse(&kotlin).unwrap();
        let strings = parsed.strings(kotlin.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 60,
                    value: "abcdef".into()
                },
                LintableString {
                    offset: 97,
                    value: "foobar".into()
                },
                LintableString {
                    offset: 118,
                    value: "Hello, World! (".into()
                },
                LintableString {
                    offset: 134,
                    value: "s) ghijkl".into()
                },
            ]
        );
    }
}
