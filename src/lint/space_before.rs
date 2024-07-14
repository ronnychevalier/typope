use std::ops::Range;

use miette::{Diagnostic, SourceSpan};

use thiserror::Error;

use winnow::ascii::space1;
use winnow::combinator::{alt, not, preceded, repeat, repeat_till};
use winnow::error::InputError;
use winnow::token::{none_of, take};
use winnow::{Located, PResult, Parser};

use super::SharedSource;
use super::{Lint, Typo};

/// A space *before* a punctuation mark has been detected.
///
/// In English typography, one must not put a space before a colon (`:`),
/// a semicolon (`;`), a question mark (`?`), an exclamation mark (`!`),
/// or an interrobang (`‽`).
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
    pub fn new(range: Range<usize>, punctuation_mark: char) -> Self {
        Self {
            src: None,
            span: range.into(),
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

pub struct SpaceBeforePunctuationMarks;

impl Lint for SpaceBeforePunctuationMarks {
    fn check(s: &[u8]) -> Vec<Box<dyn Typo>> {
        fn space_before_colon<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<char, InputError<Located<&'s [u8]>>> {
            let colon = preceded(space1, ':').parse_next(input)?;

            // Handles cases when we have an emoji like `:fire:` or `:)`.
            // In such cases, we should not mark them as a typo.
            // not(terminated(alpha1, ':')).parse_next(input)?;
            not(none_of([' '])).parse_next(input)?;

            Ok(colon)
        }

        fn space_before_exclamation_mark<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<char, InputError<Located<&'s [u8]>>> {
            let exclamation_mark = preceded(space1, '!').parse_next(input)?;

            // Do not mark such a string `x != y` as a typo
            not('=').parse_next(input)?;

            Ok(exclamation_mark)
        }

        fn space_before_punctuation<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<char, InputError<Located<&'s [u8]>>> {
            let punctuation_mark = alt((
                space_before_colon,
                space_before_exclamation_mark,
                preceded(space1, alt(('?', '‽', '⸘'))),
            ))
            .parse_next(input)?;

            Ok(punctuation_mark)
        }

        fn locate_space_before_punctuation<'s>(
            input: &mut Located<&'s [u8]>,
        ) -> PResult<Box<dyn Typo>, InputError<Located<&'s [u8]>>> {
            let (_, (punctuation_mark, range)): (Vec<u8>, (char, Range<usize>)) = repeat_till(
                1..,
                take::<_, _, InputError<_>>(1usize),
                space_before_punctuation.with_span(),
            )
            .parse_next(input)?;

            Ok(Box::new(TypoSpaceBeforePunctuationMarks::new(
                range,
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
    use crate::lint::{Lint, SharedSource};

    use super::SpaceBeforePunctuationMarks;

    #[test]
    fn empty() {
        assert!(SpaceBeforePunctuationMarks::check(b"").is_empty());
    }

    #[test]
    fn space_after_colon() {
        let typos = SpaceBeforePunctuationMarks::check(b"test: foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"test : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_question_mark() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"footest ? foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"footest ! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark_repeated() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"footest !!!! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_neq() {
        assert!(SpaceBeforePunctuationMarks::check(b"maybe 0 != 1?").is_empty());
    }

    #[test]
    fn typo_before_end_of_line() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"footest !");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check(b"footest ?");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check(b"footest :");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn multiple_typos() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"footest ! barfoobar : oh no ?");

        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (27, 2).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (19, 2).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (7, 2).into());

        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon_multiple_spaces() {
        let mut typos = SpaceBeforePunctuationMarks::check(b"test  : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 3).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn emoji() {
        assert!(SpaceBeforePunctuationMarks::check(b":waving_hand:").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b"footest :fire: bar").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b"foobar :)").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b":D").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b" :> ").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b"foo :'( bar").is_empty());
    }

    #[test]
    fn typo_source() {
        let source = "\"test  : foobar\"";
        let mut typos = SpaceBeforePunctuationMarks::check(source.trim_matches('"').as_bytes());
        let mut typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 3).into());
        let source = SharedSource::new("fake.rs", source.to_owned().into_bytes());
        typo.with_source(source, 1);
        assert_eq!(typo.span(), (5, 3).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn interrobang() {
        assert!(SpaceBeforePunctuationMarks::check("test‽".as_bytes()).is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b"test?!").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(b"test!?").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("test⸘".as_bytes()).is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("test ‽".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 4).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("test ?! abc ⸘".as_bytes());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (11, 4).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span(), (4, 2).into());
        assert!(typos.is_empty());
    }
}
