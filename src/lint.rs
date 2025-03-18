use std::io::Write;
use std::path::{Path, PathBuf};

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

/// The kind of action to perform to fix the lint suggestion
pub enum Fix {
    /// Unclear how to fix the typo, nothing is done
    Unknown,

    /// Removes some characters
    Remove { span: SourceSpan },
}

pub struct TypoFixer {
    path: PathBuf,
    buffer: Vec<u8>,
    offset: isize,
}

impl TypoFixer {
    pub fn new(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let buffer = std::fs::read(path)?;

        Ok(Self {
            path: path.into(),
            buffer,
            offset: 0,
        })
    }

    pub fn fix(&mut self, typo: &dyn Typo) -> anyhow::Result<()> {
        self.offset += typo
            .fix()
            .apply_with_offset(&mut self.buffer, self.offset)?;

        Ok(())
    }
}

impl Drop for TypoFixer {
    fn drop(&mut self) {
        let write_changes = || -> anyhow::Result<()> {
            let mut file = if let Some(parent) = self.path.parent() {
                tempfile::NamedTempFile::new_in(parent)?
            } else {
                tempfile::NamedTempFile::new()?
            };
            file.write_all(&self.buffer)?;
            file.persist(&self.path)?;

            Ok(())
        };

        let _ = write_changes();
    }
}

impl Fix {
    /// Applies the action on the given buffer.
    ///
    /// Returns the offset that needs to be given to this function for
    /// the following calls.
    fn apply_with_offset(&self, buffer: &mut Vec<u8>, offset: isize) -> anyhow::Result<isize> {
        match self {
            Self::Unknown => Ok(0),
            Self::Remove { span } => {
                let typo_offset: isize = span.offset().try_into()?;
                let start: usize = (offset + typo_offset).try_into()?;
                let end = start + span.len();

                buffer.splice(start..end, []);

                Ok(-span.len().try_into()?)
            }
        }
    }
}

/// Type that represents a typo found
pub trait Typo: miette::Diagnostic + std::error::Error + Sync + Send {
    /// Span that identify where the typo is located
    fn span(&self) -> SourceSpan;

    /// Specify within which source the typo has been found
    fn with_source(&mut self, src: SharedSource, offset: usize);

    /// Returns the action to perform to fix the typo
    fn fix(&self) -> Fix {
        Fix::Unknown
    }
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
        let filename = path.file_name().unwrap_or_default();
        let Some(language) = Language::from_filename(filename) else {
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

    /// Returns an iterator over the strings that can be linted in the source
    pub fn strings(&mut self) -> impl Iterator<Item = String> + '_ {
        self.parsed.strings(self.source.as_ref()).map(Into::into)
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

    use super::{Fix, Linter};

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
        let rust = r"
        /// Doc comment
        ///
        /// The example below should not trigger a warning since this is code:
        /// ```
        /// let english_words : u8 = 1;
        /// ```
        fn func() -> bool {
            true
        }
        ";
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }

    #[cfg(feature = "lang-markdown")]
    #[test]
    fn typo_markdown_inline() {
        let markdown = r"# Hello
Hello mate `this should not trigger the rule : foobar` abc
        ";
        let mut linter = Linter::new(
            &Language::markdown(),
            markdown.as_bytes().to_vec(),
            "file.md",
        )
        .unwrap();

        let typos = linter.iter().count();
        assert_eq!(typos, 0);
    }

    #[test]
    fn apply_with_offset() {
        let mut content = b"123456".to_vec();

        let fix = Fix::Remove {
            span: (1, 2).into(),
        };
        fix.apply_with_offset(&mut content, 0).unwrap();
        assert_eq!("1456", String::from_utf8_lossy(&content));

        let fix = Fix::Remove {
            span: (1, 1).into(),
        };
        fix.apply_with_offset(&mut content, 2).unwrap();
        assert_eq!("145", String::from_utf8_lossy(&content));
    }

    #[cfg(feature = "lang-markdown")]
    #[test]
    fn typo_markdown_apply_multiple_fixes() {
        use crate::lint::TypoFixer;

        let markdown = r#"This should trigger the rule : foobar This one ! Annnnnd here ??? And !!! what about ! or this! and this ?? "another test with ! in it, but nothing else""#;
        let markdown_fixed = r#"This should trigger the rule: foobar This one! Annnnnd here??? And!!! what about! or this! and this?? "another test with! in it, but nothing else""#;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("file.md");
        std::fs::write(&file_path, markdown.as_bytes()).unwrap();

        let mut linter = Linter::from_path(&file_path).unwrap().unwrap();

        let typos = linter.iter().collect::<Vec<_>>();
        assert_eq!(typos.len(), 7);

        let mut fixer = TypoFixer::new(&file_path).unwrap();
        for typo in typos.into_iter().rev() {
            fixer.fix(typo.as_ref()).unwrap();
        }

        drop(fixer);

        assert_eq!(markdown_fixed, std::fs::read_to_string(file_path).unwrap());
    }

    #[cfg(feature = "lang-rust")]
    #[test]
    fn strings() {
        let rust = r#"
        /// Docstring
        fn regex() -> &str {
            let _unused = String::from("abcd");
            "something"
        }
        "#;
        let mut linter = Linter::new(&Language::rust(), rust, "file.rs").unwrap();

        let strings = linter.strings().collect::<Vec<_>>();
        assert_eq!(strings, &["abcd", "something"]);
    }
}
