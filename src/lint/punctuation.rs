//! Typographical mistakes related to punctuation.
//!
//! Here is a list of typos it can find:
//! - [A space *before* a punctuation mark](`TypoSpaceBeforePunctuationMarks`)
use std::ops::Range;

use miette::{Diagnostic, SourceSpan};

use thiserror::Error;

use winnow::ascii::{alphanumeric1, digit1};
use winnow::combinator::{alt, delimited, not, preceded, repeat, repeat_till, terminated};
use winnow::error::InputError;
use winnow::token::{none_of, one_of, take};
use winnow::{Located, PResult, Parser};

use super::SharedSource;
use super::{Rule, Typo};

/// A space *before* a punctuation mark has been detected.
///
/// In English and German typography, one must not put a space before a colon (`:`),
/// a semicolon (`;`), a question mark (`?`), an exclamation mark (`!`),
/// or an interrobang (`‽`).
///
/// # Examples
///
/// Here is a list of mistakes that trigger this rule:
/// - `Oh no !`, should be `Oh no!`
/// - `here is a list of things :`, should be `here is a list of things:`
#[derive(Error, Debug, Diagnostic)]
#[error("In English typography there is no space before a punctuation mark")]
#[diagnostic(code("typope::space-before-punctuation-mark"), url(docsrs))]
pub struct TypoSpaceBeforePunctuationMarks {
    #[source_code]
    src: Option<SharedSource>,

    #[label("Invalid space here")]
    span: SourceSpan,

    #[help]
    help: String,
}

impl TypoSpaceBeforePunctuationMarks {
    fn new(span: impl Into<SourceSpan>, punctuation_mark: char) -> Self {
        Self {
            src: None,
            span: span.into(),
            help: format!("remove the space before `{punctuation_mark}`"),
        }
    }
}

impl Typo for TypoSpaceBeforePunctuationMarks {
    fn span(&self) -> SourceSpan {
        self.span
    }

    fn with_source(&mut self, src: SharedSource, offset: usize) {
        self.src = Some(src);
        self.span = (self.span.offset() + offset, self.span.len()).into();
    }
}

/// A rule that detects typographical mistakes related to punctuation.
///
/// Currently, it can only find and generate the following typo: [`TypoSpaceBeforePunctuationMarks`].
pub struct Punctuation;

impl Rule for Punctuation {
    #[allow(clippy::type_complexity)]
    fn check(&self, bytes: &[u8]) -> Vec<Box<dyn Typo>> {
        fn space_before_colon<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ', '>']), ' '.with_span(), ':').parse_next(input)?;

            // Handles cases when we have an emoji like `:fire:` or `:)`.
            // In such cases, we should not mark them as a typo.
            not(none_of([' '])).parse_next(input)?;

            Ok((':', range))
        }

        fn space_before_exclamation_mark<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ', '&', '=', '>', '|']), ' '.with_span(), '!')
                    .parse_next(input)?;

            // Do not mark such a string `x != y` as a typo
            not('=').parse_next(input)?;
            not('(').parse_next(input)?;
            // Do not mark strings like ` !Send` as a typo: it has a meaning in Rust
            not(alt(("Send", "Sync"))).parse_next(input)?;
            // A string might contain some kind of shell script like `[ ! -e /some/file ]`
            // See `man test` for the possible options.
            not(preceded(
                " -",
                alt((
                    one_of('b'..='h'),
                    one_of([
                        'G', 'k', 'L', 'N', 'O', 'p', 'r', 's', 'S', 't', 'u', 'w', 'x',
                    ]),
                )),
            ))
            .parse_next(input)?;
            // Can be found in code generating C macros (e.g., `#elif !defined(condition)`)
            not("defined(").parse_next(input)?;

            Ok(('!', range))
        }

        fn space_before_question_mark<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ']), ' '.with_span(), '?').parse_next(input)?;

            // Do not mark strings like ` ?Sized` as a typo: it has a meaning in Rust
            not("Sized").parse_next(input)?;
            // Can be found in a text that gives an example of the parameters to use in a URL (e.g., `add ?param=2&param2=40 to the URL`)
            not(terminated(alphanumeric1, '=')).parse_next(input)?;
            // Can be found in SQL queries (e.g., `SELECT a FROM b WHERE c = ?1 AND d = ?2`)
            // See <https://www.sqlite.org/c3ref/bind_blob.html>
            not(digit1).parse_next(input)?;

            Ok(('?', range))
        }

        fn space_before_char<'s, const C: char>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) = terminated(' '.with_span(), C).parse_next(input)?;

            Ok((C, range))
        }

        fn space_before_punctuation<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            alt((
                space_before_colon,
                space_before_exclamation_mark,
                space_before_question_mark,
                space_before_char::<'‽'>,
                space_before_char::<'⸘'>,
            ))
            .parse_next(input)
        }

        fn locate_space_before_punctuation<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<Box<dyn Typo>, InputError<Located<&'s [u8]>>> {
            let (_, (punctuation_mark, range)): (Vec<u8>, (char, Range<usize>)) = repeat_till(
                1..,
                take::<_, _, InputError<_>>(1usize),
                space_before_punctuation,
            )
            .parse_next(input)?;

            // We only mark the space that is invalid not the rest
            let span = (range.start, 1);

            Ok(Box::new(TypoSpaceBeforePunctuationMarks::new(
                span,
                punctuation_mark,
            )))
        }

        repeat(0.., locate_space_before_punctuation)
            .parse_next(&mut Located::new(bytes))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::{Rule, SharedSource};

    use super::Punctuation;

    #[test]
    fn empty() {
        assert!(Punctuation.check(br"").is_empty());
    }

    #[test]
    fn space_after_colon() {
        let typos = Punctuation.check(br"test: foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon() {
        let mut typos = Punctuation.check(br"test : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_question_mark() {
        let mut typos = Punctuation.check(br"footest ? foobar ?fooooo");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (16, 1).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark() {
        let mut typos = Punctuation.check(br"footest ! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark_repeated() {
        let mut typos = Punctuation.check(br"footest !!!! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_neq() {
        assert!(Punctuation.check(br"maybe 0 != 1?").is_empty());
    }

    #[test]
    fn typo_before_end_of_line() {
        let mut typos = Punctuation.check(br"footest !");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());

        let mut typos = Punctuation.check(br"footest ?");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());

        let mut typos = Punctuation.check(br"footest :");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn multiple_typos() {
        let mut typos = Punctuation.check(br"footest ! barfoobar : oh no ?");

        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (27, 1).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (19, 1).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());

        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon_multiple_spaces() {
        let typos = Punctuation.check(br"test     : foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_rust_sized() {
        let typos = Punctuation.check(br"test: ?Sized foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn emoji() {
        assert!(Punctuation.check(br":waving_hand:").is_empty());
        assert!(Punctuation.check(br"footest :fire: bar").is_empty());
        assert!(Punctuation.check(br"foobar :)").is_empty());
        assert!(Punctuation.check(br":D").is_empty());
        assert!(Punctuation.check(br" :> ").is_empty());
        assert!(Punctuation.check(br"foo :'( bar").is_empty());
    }

    #[test]
    fn typo_source() {
        let source = r#""test : foobar""#;
        let mut typos = Punctuation.check(source.trim_matches('"').as_bytes());
        let mut typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        let source = SharedSource::new("fake.rs", source.to_owned().into_bytes());
        typo.with_source(source, 1);
        assert_eq!(typo.span(), (5, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn interrobang() {
        assert!(Punctuation.check(r"test‽".as_bytes()).is_empty());
        assert!(Punctuation.check(br"test?!").is_empty());
        assert!(Punctuation.check(br"test!?").is_empty());
        assert!(Punctuation.check(r"test⸘".as_bytes()).is_empty());

        let mut typos = Punctuation.check(r"test ‽".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());

        let mut typos = Punctuation.check(r"test ?! abc ⸘".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (11, 1).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn looks_like_shell() {
        assert!(Punctuation
            .check(br"[ ! -e /run/dbus ] || mount -t tmpfs none /run/dbus")
            .is_empty());
    }

    #[test]
    fn looks_like_c_macro_generated() {
        assert!(Punctuation
            .check(br"#  elif !defined(missing_arch_template)")
            .is_empty());
    }

    #[test]
    fn looks_like_url_parameter() {
        assert!(Punctuation
            .check(br"Add ?var=1&var2=44 to the URL")
            .is_empty());
    }

    #[test]
    fn sqlite_prepared_statement() {
        assert!(Punctuation
            .check(br"SELECT a FROM b WHERE c = ?1 AND d = ?2")
            .is_empty());
    }

    #[test]
    fn fn_return() {
        assert!(Punctuation.check(br"fn() -> !").is_empty());
    }

    #[test]
    fn condition() {
        assert!(Punctuation
            .check(br"a & !b & !c | !z  or !(y | w)")
            .is_empty());
    }
}
