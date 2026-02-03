use super::{Language, Mode};

impl Language {
    /// Creates a language parser for Python
    pub fn python() -> Self {
        Self {
            name: "python",
            detections: &["*.py"],
            parser: Mode::Query {
                language: tree_sitter::Language::new(tree_sitter_python::LANGUAGE),
                query: "; Module docstring
((module . (expression_statement (string (string_content) @docstrings))))

; Class docstring
((class_definition
  body: (block . (expression_statement  (string (string_content) @docstrings)))))

; Function/method docstring
((function_definition
  body: (block . (expression_statement (string (string_content) @docstrings)))))

; Attribute docstring
(((expression_statement (assignment)) . (expression_statement  (string (string_content) @docstrings))))

(string_content)+ @strings"
                    .into(),
                ignore_captures: Some(&["docstrings"]),
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
        assert!(Language::iter().any(|lang| lang.name() == "python"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "python",
            Language::from_filename(OsStr::new("file.py"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let python = r#"
"""
Module docstring foobar
"""
s = "abcd"
d = 1234 # Hehe "foobar"
s = "abcd" "efgh"

# Comments
def f():
    """
    Function docstring
    """

    return 'ijkl'
"#;
        let python = SharedSource::new("file.py", python.as_bytes().to_vec());
        let mut parsed = Language::python().parse(&python).unwrap();
        let strings = parsed.strings(python.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 38,
                    value: "abcd".into()
                },
                LintableString {
                    offset: 74,
                    value: "abcd".into()
                },
                LintableString {
                    offset: 81,
                    value: "efgh".into()
                },
                LintableString {
                    offset: 160,
                    value: "ijkl".into()
                }
            ]
        );
    }
}
