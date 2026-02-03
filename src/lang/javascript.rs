use super::{Language, Mode};

impl Language {
    /// Creates a language parser for JavaScript
    pub fn javascript() -> Self {
        Self {
            name: "javascript",
            detections: &["*.js"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_javascript::LANGUAGE),
                tree_sitter_types: &["string_fragment"],
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
        assert!(Language::iter().any(|lang| lang.name() == "javascript"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "javascript",
            Language::from_filename(OsStr::new("file.js"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let javascript = r#"
var a = "test";
button = document.createElement("button");
button.addEventListener("click", cb);
"#;
        let javascript = SharedSource::new("file.js", javascript.as_bytes().to_vec());
        let mut parsed = Language::javascript().parse(&javascript).unwrap();
        let strings = parsed.strings(javascript.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 10,
                    value: "test".into()
                },
                LintableString {
                    offset: 50,
                    value: "button".into()
                },
                LintableString {
                    offset: 85,
                    value: "click".into()
                }
            ]
        );
    }
}
