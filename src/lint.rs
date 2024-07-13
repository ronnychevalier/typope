pub trait Lint {
    type Typo: miette::Diagnostic;

    fn check(s: &str) -> Vec<Self::Typo>;
}

mod space_before;

pub use space_before::SpaceBeforePunctuationMarks;
