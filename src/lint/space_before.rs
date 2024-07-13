use std::ops::Range;

use miette::{Diagnostic, SourceSpan};

use thiserror::Error;

use winnow::ascii::space1;
use winnow::combinator::{alt, not, preceded, repeat, repeat_till};
use winnow::error::InputError;
use winnow::token::{none_of, take};
use winnow::{Located, PResult, Parser};

use crate::SharedSource;

use super::Lint;

/// A space *before* a punctuation mark has been detected.
///
/// In English typography, one must not put a space before a colon (`:`),
/// a semicolon (`;`), a question mark (`?`), an exclamation mark (`!`),
/// or an interrobang (`‽`).
///
/// # References
/// -
#[derive(Error, Debug, Diagnostic)]
#[error("In English typography there is no space before a punctuation mark")]
#[diagnostic(code("orthotypos::space-before-punctuation-mark"), url(docsrs))]
pub struct Typo {
    #[source_code]
    src: Option<SharedSource>,

    #[label("Invalid space here")]
    span: SourceSpan,

    #[help]
    help: String,
}

impl Typo {
    pub fn new(range: Range<usize>, punctuation_mark: char) -> Self {
        Self {
            src: None,
            span: range.into(),
            help: format!("remove the space before `{punctuation_mark}`"),
        }
    }

    pub fn with_source(mut self, src: SharedSource, offset: usize) -> Self {
        self.src = Some(src);
        self.span = (self.span.offset() + offset, self.span.len()).into();
        self
    }
}

pub struct SpaceBeforePunctuationMarks;

impl Lint for SpaceBeforePunctuationMarks {
    type Typo = Typo;

    fn check(s: &str) -> Vec<Typo> {
        fn space_before_colon<'s>(
            input: &mut Located<&'s str>,
        ) -> PResult<char, InputError<Located<&'s str>>> {
            let colon = preceded(space1, ':').parse_next(input)?;

            // Handles cases when we have an emoji like `:fire:` or `:)`.
            // In such cases, we should not mark them as a typo.
            // not(terminated(alpha1, ':')).parse_next(input)?;
            not(none_of([' '])).parse_next(input)?;

            Ok(colon)
        }

        fn space_before_punctuation<'s>(
            input: &mut Located<&'s str>,
        ) -> PResult<char, InputError<Located<&'s str>>> {
            let punctuation_mark = alt((
                space_before_colon,
                preceded(space1, alt(('!', '?', '‽', '⸘'))),
            ))
            .parse_next(input)?;

            Ok(punctuation_mark)
        }

        fn locate_space_before_punctuation<'s>(
            input: &mut Located<&'s str>,
        ) -> PResult<Typo, InputError<Located<&'s str>>> {
            let (_, (punctuation_mark, range)): (String, (char, Range<usize>)) = repeat_till(
                1..,
                take::<_, _, InputError<_>>(1usize),
                space_before_punctuation.with_span(),
            )
            .parse_next(input)?;

            Ok(Typo::new(range, punctuation_mark))
        }

        repeat(0.., locate_space_before_punctuation)
            .parse_next(&mut Located::new(s))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::lint::Lint;
    use crate::SharedSource;

    use super::SpaceBeforePunctuationMarks;

    #[test]
    fn empty() {
        assert!(SpaceBeforePunctuationMarks::check("").is_empty());
    }

    #[test]
    fn space_after_colon() {
        let typos = SpaceBeforePunctuationMarks::check("test: foobar");
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon() {
        let mut typos = SpaceBeforePunctuationMarks::check("test : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (4, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_question_mark() {
        let mut typos = SpaceBeforePunctuationMarks::check("footest ? foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark() {
        let mut typos = SpaceBeforePunctuationMarks::check("footest ! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_exclamation_mark_repeated() {
        let mut typos = SpaceBeforePunctuationMarks::check("footest !!!! barfoobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn typo_before_end_of_line() {
        let mut typos = SpaceBeforePunctuationMarks::check("footest !");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("footest ?");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("footest :");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn multiple_typos() {
        let mut typos = SpaceBeforePunctuationMarks::check("footest ! barfoobar : oh no ?");

        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (27, 2).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (19, 2).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (7, 2).into());

        assert!(typos.is_empty());
    }

    #[test]
    fn typo_colon_multiple_spaces() {
        let mut typos = SpaceBeforePunctuationMarks::check("test  : foobar");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (4, 3).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn emoji() {
        assert!(SpaceBeforePunctuationMarks::check(":waving_hand:").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("footest :fire: bar").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("foobar :)").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(":D").is_empty());
        assert!(SpaceBeforePunctuationMarks::check(" :> ").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("foo :'( bar").is_empty());
    }

    #[test]
    fn typo_source() {
        let source = "\"test  : foobar\"";
        let mut typos = SpaceBeforePunctuationMarks::check(source.trim_matches('"'));
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (4, 3).into());
        let source = SharedSource::new("fake.rs", source);
        let typo = typo.with_source(source, 1);
        assert_eq!(typo.span, (5, 3).into());
        assert!(typos.is_empty());
    }

    #[test]
    fn interrobang() {
        assert!(SpaceBeforePunctuationMarks::check("test‽").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("test?!").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("test!?").is_empty());
        assert!(SpaceBeforePunctuationMarks::check("test⸘").is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("test ‽");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (4, 4).into());
        assert!(typos.is_empty());

        let mut typos = SpaceBeforePunctuationMarks::check("test ?! abc ⸘");
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (11, 4).into());
        let typo = typos.pop().unwrap();
        assert_eq!(typo.span, (4, 2).into());
        assert!(typos.is_empty());
    }
}
