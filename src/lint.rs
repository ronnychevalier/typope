use std::path::Path;

use miette::{SourceCode, SourceSpan};

pub mod punctuation;

use self::punctuation::Punctuation;

use crate::lang::{Language, LintableString, Parsed};
use crate::SharedSource;

/// Type that represents a rule that checks for typos
pub trait Rule {
    /// Returns the typos found by applying this rule to an array of bytes
    fn check(&self, bytes: &[u8]) -> Vec<Box<dyn Typo>>;
}

/// Type that represents a typo found
pub trait Typo: miette::Diagnostic + std::error::Error + Sync + Send {
    /// Span that identify where the typo is located
    fn span(&self) -> SourceSpan;

    /// Specify within which source the typo has been found
    fn with_source(&mut self, src: SharedSource, offset: usize);
}

/// Detects typos in a file
pub struct Linter {
    parsed: Box<dyn Parsed>,
    source: SharedSource,
    rules: Vec<Box<dyn Rule>>,
    ignore_re: Vec<regex::Regex>,
}

impl Linter {
    /// Builds a linter that checks for typos in the file at the given path
    pub fn from_path(source: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        let path = source.as_ref();
        let extension = path.extension().unwrap_or_default();
        let Some(language) = Language::from_extension(extension) else {
            // TODO: parse the file as a text file without tree-sitter
            return Ok(None);
        };

        let source_content = std::fs::read(path)?;
        let linter = Self::new(language, source_content, path.to_string_lossy())?;

        Ok(Some(linter))
    }

    fn new(
        lang: &Language,
        source_content: impl Into<Vec<u8>>,
        source_name: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let source_content = source_content.into();
        let source = SharedSource::new(source_name, source_content);
        let parsed = lang.parse(&source)?;

        let rules = vec![Box::new(Punctuation) as Box<dyn Rule>];

        Ok(Self {
            parsed,
            source,
            rules,
            ignore_re: Vec::new(),
        })
    }

    /// Extends the list of regexes that prevents some strings from being checked
    pub fn extend_ignore_re(&mut self, ignore_re: &[regex::Regex]) {
        self.ignore_re.extend_from_slice(ignore_re);
    }

    /// Returns an iterator over the typos found in the source
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use typope::lint::Linter;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let Some(mut linter) = Linter::from_path("file.rs")? else { return Ok(()); };
    /// for typo in linter.iter() {
    ///     let typo: miette::Report = typo.into();
    ///     eprintln!("{typo:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn iter(&mut self) -> Iter<'_> {
        Iter::new(self)
    }
}

/// Iterator over the typos found in a file
pub struct Iter<'t> {
    strings: Box<dyn Iterator<Item = LintableString> + 't>,
    source: SharedSource,
    typos: Vec<Box<dyn Typo>>,
    rules: &'t [Box<dyn Rule>],
    ignore_re: &'t [regex::Regex],
}

impl<'t> Iter<'t> {
    fn new(linter: &'t mut Linter) -> Self {
        Self {
            strings: linter.parsed.strings(linter.source.as_ref()),
            source: linter.source.clone(),
            typos: vec![],
            rules: &linter.rules,
            ignore_re: &linter.ignore_re,
        }
    }
}

impl Iterator for Iter<'_> {
    type Item = Box<dyn Typo>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(typo) = self.typos.pop() {
                return Some(typo);
            }

            let string = self.strings.next()?;

            let offset = string.offset();
            let ignored = self.ignore_re.iter().any(|re| re.is_match(string.as_str()));
            if ignored {
                continue;
            }

            let source = self.source.clone();
            let typos = self
                .rules
                .iter()
                .flat_map(move |rule| rule.check(string.as_str().as_bytes()))
                .map(move |mut typo| {
                    typo.with_source(source.clone(), offset);
                    typo
                });

            self.typos.extend(typos);
        }
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
    use crate::lint::Language;

    use super::Linter;

    #[test]
    fn from_path_unknown_extension() {
        assert!(Linter::from_path("file.with_unknown_extension")
            .unwrap()
            .is_none());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_string() {
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }
        "#;
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let mut typos = linter.iter().collect::<Vec<_>>();
        assert_eq!(typos.len(), 1);
        let typo = typos.pop().unwrap();
        assert_eq!(
            format!("{}", typo.code().unwrap()),
            "typope::space-before-punctuation-mark"
        );
        assert_eq!(typo.span(), (141, 1).into());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_into_report() {
        use miette::LabeledSpan;

        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }
        "#;
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let mut typos = linter.iter().collect::<Vec<_>>();
        assert_eq!(typos.len(), 1);
        let typo = typos.pop().unwrap();
        let report: miette::Report = typo.into();
        assert_eq!(
            format!("{}", report.code().unwrap()),
            "typope::space-before-punctuation-mark"
        );
        assert_eq!(
            report.labels().unwrap().collect::<Vec<_>>(),
            [LabeledSpan::new(Some("Invalid space here".into()), 141, 1)]
        );
        assert_eq!(
            report.help().unwrap().to_string(),
            "remove the space before `:`"
        );
        assert!(report
            .url()
            .unwrap()
            .to_string()
            .starts_with("https://docs.rs"));
        assert!(report.source_code().is_some());
        assert!(report.diagnostic_source().is_none());
        assert_eq!(report.severity(), None);
        assert!(report.related().is_none());
        assert!(report.source().is_none());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_from_path() {
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }
        "#;
        let file = tempfile::Builder::new().suffix(".rs").tempfile().unwrap();
        std::fs::write(file.path(), rust).unwrap();
        let mut linter = Linter::from_path(file.path()).unwrap().unwrap();

        let mut typos = linter.iter().collect::<Vec<_>>();
        assert_eq!(typos.len(), 1);
        let typo = typos.pop().unwrap();
        assert_eq!(
            format!("{}", typo.code().unwrap()),
            "typope::space-before-punctuation-mark"
        );
        assert_eq!(typo.span(), (141, 1).into());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_string_ignored() {
        let rust = r#"
        /// Doc comment
        fn func() -> anyhow::Result<()> {
            anyhow::bail!("failed to do something for the following reason : foobar foo");
        }
        "#;
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();
        linter.extend_ignore_re(&[regex::Regex::new(r"foobar foo").unwrap()]);

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_rawstring() {
        let rust = r#"
        fn regex() -> &str {
            r"a ?regex.that ?match ?something ?"
        }
        "#;
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn typo_rust_doctest() {
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
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }

    #[cfg(feature = "lang-markdown")]
    #[test]
    fn typo_markdown_inline() {
        let markdown = r#"# Hello
Hello mate `this should not trigger the rule : foobar` abc
        "#;
        let mut linter = Linter::new(
            &Language::markdown(),
            markdown.as_bytes().to_vec(),
            "file.md",
        )
        .unwrap();

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }
}
