use super::{Language, Mode};

impl Language {
    /// Creates a language parser for TypeScript
    pub fn typescript() -> Self {
        Self {
            name: "typescript",
            detections: &["*.ts"],
            parser: Mode::Generic {
                language: tree_sitter::Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
                tree_sitter_types: &["string_fragment"],
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
        assert!(Language::iter().any(|lang| lang.name() == "typescript"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "typescript",
            Language::from_filename(OsStr::new("file.ts"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let typescript = r#"
let helloWorld = "Hello World";
const user = {
  name: "Hayes",
  id: 0,
};
type WindowStates = "open" | "closed" | "minimized";
"#;
        let typescript = SharedSource::new("file.ts", typescript.as_bytes().to_vec());
        let mut parsed = Language::typescript().parse(&typescript).unwrap();
        let strings = parsed.strings(typescript.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 19,
                    value: "Hello World".into()
                },
                LintableString {
                    offset: 57,
                    value: "Hayes".into()
                },
                LintableString {
                    offset: 98,
                    value: "open".into()
                },
                LintableString {
                    offset: 107,
                    value: "closed".into()
                },
                LintableString {
                    offset: 118,
                    value: "minimized".into()
                }
            ]
        );
    }
}
