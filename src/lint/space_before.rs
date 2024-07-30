use std::ops::Range;

use miette::{Diagnostic, SourceSpan};

use thiserror::Error;

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
#[diagnostic(code("orthotypos::space-before-punctuation-mark"), url(docsrs))]
pub struct TypoSpaceBeforePunctuationMarks {
    #[source_code]
    src: Option<SharedSource>,

    #[label("Invalid space here")]
    span: SourceSpan,

    #[help]
    help: String,
}

impl TypoSpaceBeforePunctuationMarks {
    pub fn new(span: impl Into<SourceSpan>, punctuation_mark: char) -> Self {
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

/// A rule that detects when a space is present *before* a punctuation mark.
///
/// See [`TypoSpaceBeforePunctuationMarks`] for more details.
pub struct SpaceBeforePunctuationMarks;

impl Rule for SpaceBeforePunctuationMarks {
    fn check(&self, s: &[u8]) -> Vec<Box<dyn Typo>> {
        fn space_before_colon<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ']), ' '.with_span(), ':').parse_next(input)?;

            // Handles cases when we have an emoji like `:fire:` or `:)`.
            // In such cases, we should not mark them as a typo.
            // not(terminated(alpha1, ':')).parse_next(input)?;
            not(none_of([' '])).parse_next(input)?;

            Ok((':', range))
        }

        fn space_before_exclamation_mark<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ']), ' '.with_span(), '!').parse_next(input)?;

            // Do not mark such a string `x != y` as a typo
            not('=').parse_next(input)?;
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

            Ok(('!', range))
        }

        fn space_before_question_mark<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<(char, Range<usize>), InputError<Located<&'s [u8]>>> {
            let (_space, range) =
                delimited(none_of([' ']), ' '.with_span(), '?').parse_next(input)?;

            // Do not mark strings like ` ?Sized` as a typo: it has a meaning in Rust
            not("Sized").parse_next(input)?;

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
            .parse_next(&mut Located::new(s))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::{Rule, SharedSource};

    use super::SpaceBeforePunctuationMarks;

    #[test]
    fn empty() {
        assert!(SpaceBeforePunctuationMarks.check(br"").is_empty());
    }

    #[test]
    fn space_after_colon() {
        let typos = SpaceBeforePunctuationMarks.check(br"test: foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"test : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_question_mark() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"footest ? foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"footest ! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark_repeated() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"footest !!!! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_neq() {
        assert!(SpaceBeforePunctuationMarks
            .check(br"maybe 0 != 1?")
            .is_empty());
    }

    #[test]
    fn typo_before_end_of_line() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"footest !");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks.check(br"footest ?");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks.check(br"footest :");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn multiple_typos() {
        let mut typos = SpaceBeforePunctuationMarks.check(br"footest ! barfoobar : oh no ?");

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
        let typos = SpaceBeforePunctuationMarks.check(br"test     : foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_rust_sized() {
        let typos = SpaceBeforePunctuationMarks.check(br"test: ?Sized foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn emoji() {
        assert!(SpaceBeforePunctuationMarks
            .check(br":waving_hand:")
            .is_empty());
        assert!(SpaceBeforePunctuationMarks
            .check(br"footest :fire: bar")
            .is_empty());
        assert!(SpaceBeforePunctuationMarks.check(br"foobar :)").is_empty());
        assert!(SpaceBeforePunctuationMarks.check(br":D").is_empty());
        assert!(SpaceBeforePunctuationMarks.check(br" :> ").is_empty());
        assert!(SpaceBeforePunctuationMarks
            .check(br"foo :'( bar")
            .is_empty());
    }

    #[test]
    fn typo_source() {
        let source = r#""test : foobar""#;
        let mut typos = SpaceBeforePunctuationMarks.check(source.trim_matches('"').as_bytes());
        let mut typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        let source = SharedSource::new("fake.rs", source.to_owned().into_bytes());
        typo.with_source(source, 1);
        assert_eq!(typo.span(), (5, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn interrobang() {
        assert!(SpaceBeforePunctuationMarks
            .check(r"test‽".as_bytes())
            .is_empty());
        assert!(SpaceBeforePunctuationMarks.check(br"test?!").is_empty());
        assert!(SpaceBeforePunctuationMarks.check(br"test!?").is_empty());
        assert!(SpaceBeforePunctuationMarks
            .check(r"test⸘".as_bytes())
            .is_empty());

        let mut typos = SpaceBeforePunctuationMarks.check(r"test ‽".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks.check(r"test ?! abc ⸘".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (11, 1).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 1).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn looks_like_shell() {
        assert!(SpaceBeforePunctuationMarks
            .check(br"[ ! -e /run/dbus ] || mount -t tmpfs none /run/dbus")
            .is_empty());
    }
}
