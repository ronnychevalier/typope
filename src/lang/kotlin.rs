use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Kotlin
    pub fn kotlin() -> Self {
        Self {
            name: "kotlin",
            language: tree_sitter_kotlin::language(),
            extensions: &["kt"],
            parser: Mode::Generic {
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
        assert!(Language::iter().any(|lang| lang.name() == "kotlin"));
    }

    #[test]
    fn find_from_extensions() {
        for ext in Language::kotlin().extensions() {
            assert_eq!(
                "kotlin",
                Language::from_extension(OsStr::new(ext)).unwrap().name()
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
                    offset: 135,
                    value: ") ghijkl".into()
                },
            ]
        );
    }
}
