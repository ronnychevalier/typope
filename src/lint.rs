use std::sync::Arc;

use miette::{MietteError, NamedSource, SourceCode, SourceSpan, SpanContents};

use tree_sitter::{Language, Parser, Tree};

mod space_before;

pub use self::space_before::SpaceBeforePunctuationMarks;

use crate::tree::PreorderTraversal;

pub trait Lint {
    fn check(&self, s: &[u8]) -> Vec<Box<dyn Typo>>;
}

pub trait Typo: miette::Diagnostic + std::error::Error + Sync + Send {
    fn span(&self) -> SourceSpan;
    fn with_source(&mut self, src: SharedSource, offset: usize);
}

static VALID_KINDS: &[&str] = &[
    "inline",
    // "line_comment",
    // "block_comment",
    // "inner_doc_comment_marker",
    // "outer_doc_comment_marker",
    "string_content",
    "string",
    "interpreted_string_literal",
    "string_scalar",
    "double_quote_scalar",
];

pub struct Linter {
    tree: Tree,
    source: SharedSource,
    rules: Vec<Box<dyn Lint>>,
}

impl Linter {
    pub fn new(
        language: &Language,
        source_content: impl Into<Vec<u8>>,
        source_name: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let mut parser = Parser::new();
        parser.set_language(language)?;
        let source_content = source_content.into();
        let Some(tree) = parser.parse(&source_content, None) else {
            anyhow::bail!("Invalid language");
        };
        let source = SharedSource::new(source_name, source_content);

        let rules = vec![Box::new(SpaceBeforePunctuationMarks) as Box<dyn Lint>];

        Ok(Self {
            tree,
            source,
            rules,
        })
    }

    /// Returns an iterator over the typos found in the source
    ///
    /// # Example
    ///
    /// ```
    /// # use orthotypos::lint::Linter;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let linter = Linter::new(&tree_sitter_rust::language(), b"...", "file.rs")?;
    /// for typo in &linter {
    ///     let typo: miette::Report = typo.into();
    ///     eprintln!("{typo:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }
}

impl<'t> IntoIterator for &'t Linter {
    type Item = Box<dyn Typo>;

    type IntoIter = Iter<'t>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'t> {
    traversal: PreorderTraversal<'t>,
    source: SharedSource,
    typos: Option<Box<dyn Iterator<Item = Box<dyn Typo>>>>,
    rules: &'t [Box<dyn Lint>],
}

impl<'t> Iter<'t> {
    fn new(linter: &'t Linter) -> Self {
        Self {
            traversal: PreorderTraversal::from(linter.tree.walk()),
            source: linter.source.clone(),
            typos: None,
            rules: &linter.rules,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Box<dyn Typo>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(typos) = &mut self.typos {
                if let Some(typo) = typos.next() {
                    return Some(typo);
                }

                self.typos = None;
            }

            let node = self.traversal.next()?;
            let kind = node.kind();
            if !VALID_KINDS.contains(&kind) {
                continue;
            }
            if node.byte_range().len() <= 3 {
                continue;
            }

            // Specific case for Rust to avoid linting raw strings (`r"this is a raw string"`) and creating false positives.
            // TODO: should be handled differently
            if kind == "string_content"
                && node
                    .parent()
                    .map(|parent| parent.kind() == "raw_string_literal")
                    .unwrap_or(false)
            {
                continue;
            }
            let Some(string) = self.source.inner().get(node.byte_range()) else {
                continue;
            };

            let offset = node.start_byte();
            let source = self.source.clone();
            let typos = self
                .rules
                .iter()
                .flat_map(|rule| rule.check(string))
                .collect::<Vec<_>>();
            self.typos = Some(Box::new(typos.into_iter().map(move |mut typo| {
                typo.with_source(source.clone(), offset);
                typo
            })));
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedSource(Arc<NamedSource<Vec<u8>>>);

impl std::ops::Deref for SharedSource {
    type Target = NamedSource<Vec<u8>>;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl SharedSource {
    pub fn new(name: impl AsRef<str>, bytes: Vec<u8>) -> Self {
        Self(Arc::new(NamedSource::new(name, bytes)))
    }
}

impl SourceCode for SharedSource {
    fn read_span<'a>(
        &'a self,
        span: &miette::SourceSpan,
        context_lines_before: usize,
        context_lines_after: usize,
    ) -> Result<Box<dyn SpanContents<'a> + 'a>, MietteError> {
        self.0
            .read_span(span, context_lines_before, context_lines_after)
    }
}

impl std::error::Error for Box<dyn Typo> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        (**self).source()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl miette::Diagnostic for Box<dyn Typo> {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        (**self).code()
    }

    fn severity(&self) -> Option<miette::Severity> {
        (**self).severity()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        (**self).help()
    }

    fn url<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        (**self).url()
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        (**self).source_code()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        (**self).labels()
    }

    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn miette::Diagnostic> + 'a>> {
        (**self).related()
    }

    fn diagnostic_source(&self) -> Option<&dyn miette::Diagnostic> {
        (**self).diagnostic_source()
    }
}

#[cfg(test)]
mod tests {
    use super::Linter;

    #[test]
    fn typo_rust_string() {
        let language = tree_sitter_rust::language();
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }
        "#;
        let linter = Linter::new(&language, rust.as_bytes().to_vec(), "file.rs").unwrap();

        let mut typos = linter.iter().collect::<Vec<_>>();
        assert_eq!(typos.len(), 1);
        let typo = typos.pop().unwrap();
        assert_eq!(
            format!("{}", typo.code().unwrap()),
            "orthotypos::space-before-punctuation-mark"
        );
        assert_eq!(typo.span(), (141, 2).into());
    }

    #[test]
    fn typo_rust_rawstring() {
        let language = tree_sitter_rust::language();
        let rust = r#"
        fn regex() -> &str {
            r"a ?regex.that ?match ?something ?"
        }
        "#;
        let linter = Linter::new(&language, rust.as_bytes().to_vec(), "file.rs").unwrap();

        let typos = linter.iter().collect::<Vec<_>>();
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_rust_doctest() {
        let language = tree_sitter_rust::language();
        let rust = r#"
        /// Doc comment
        ///
        /// The example below should not trigger a warning since this is code:
        /// ```
        /// let english_words : u8 = 1;
        /// ```
        fn func() -> bool {
            true
        }
        "#;
        let linter = Linter::new(&language, rust.as_bytes().to_vec(), "file.rs").unwrap();

        let typos = linter.iter().collect::<Vec<_>>();
        assert!(typos.is_empty());
    }
}
