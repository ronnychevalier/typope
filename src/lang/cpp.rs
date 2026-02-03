use super::{Language, Mode};

impl Language {
    /// Creates a language parser for C++
    pub fn cpp() -> Self {
        Self {
            name: "cpp",
            detections: &["*.[ch]pp", "*.cc", "*.hh", "*.[ch]xx"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_cpp::LANGUAGE),
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
        assert!(Language::iter().any(|lang| lang.name() == "cpp"));
    }

    #[test]
    fn find_from_filenames() {
        for filename in ["file.cc", "file.cpp", "file.hxx"] {
            assert_eq!(
                "cpp",
                Language::from_filename(OsStr::new(filename))
                    .unwrap()
                    .name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let cpp = r#"
#include <iostream>
#include <string>

#define MACRO "not handled by the parser yet"

std::string f() {
    return std::string("abcdef" MACRO);
}

int main() {
    std::string s("foobar");

    std::cout << "Hello world!" << s << std::endl;
    return 0;
}
"#;
        let cpp = SharedSource::new("file.cpp", cpp.as_bytes().to_vec());
        let mut parsed = Language::cpp().parse(&cpp).unwrap();
        let strings = parsed.strings(cpp.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 129,
                    value: "abcdef".into()
                },
                LintableString {
                    offset: 180,
                    value: "foobar".into()
                },
                LintableString {
                    offset: 209,
                    value: "Hello world!".into()
                }
            ]
        );
    }
}
