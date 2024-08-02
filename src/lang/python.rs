use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Python
    pub fn python() -> Self {
        Self {
            name: "python",
            language: tree_sitter_python::language(),
            extensions: &["py"],
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
        assert!(Language::iter().any(|lang| lang.name() == "python"));
    }

    #[test]
    fn find_from_extensions() {
        for ext in Language::python().extensions() {
            assert_eq!(
                "python",
                Language::from_extension(OsStr::new(ext)).unwrap().name()
            );
        }
    }

    #[test]
    fn lintable_strings() {
        let python = r#"
s = "abcd"
d = 1234 # Hehe "foobar"
s = "abcd" "efgh"

# Comments
def f():
    return 'ijkl'
"#;
        let python = SharedSource::new("file.py", python.as_bytes().to_vec());
        let mut parsed = Language::python().parse(&python).unwrap();
        let strings = parsed.strings(python.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 6,
                    value: "abcd".into()
                },
                LintableString {
                    offset: 42,
                    value: "abcd".into()
                },
                LintableString {
                    offset: 49,
                    value: "efgh".into()
                },
                LintableString {
                    offset: 88,
                    value: "ijkl".into()
                }
            ]
        );
    }
}
