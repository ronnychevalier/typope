use super::{Language, Mode};

impl Language {
    /// Creates a language parser for C
    pub fn c() -> Self {
        Self {
            name: "c",
            language: tree_sitter_c::language(),
            extensions: &["c", "h"],
            parser: Mode::Generic {
                tree_sitter_types: &["string_content"],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lang::LintableString;
    use crate::SharedSource;

    use super::Language;

    #[test]
    fn lintable_strings() {
        let c = r#"
#include <stdio.h>

#define MACRO "not handled by the parser yet"

const char *f() {
    return "abcdef" MACRO;
}

int main() {
    char *s = "foobar";

    printf("Hello world! %s", s);
    return 0;
}
"#;
        let c = SharedSource::new("file.c", c.as_bytes().to_vec());
        let mut parsed = Language::c().parse(&c).unwrap();
        let strings = parsed.strings(c.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 98,
                    value: "abcdef".into()
                },
                LintableString {
                    offset: 144,
                    value: "foobar".into()
                },
                LintableString {
                    offset: 166,
                    value: "Hello world! %s".into()
                }
            ]
        );
    }
}
