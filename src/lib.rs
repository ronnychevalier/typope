use std::sync::Arc;

use miette::{MietteError, NamedSource, SourceCode, SpanContents};

pub mod config;
pub mod lang;
pub mod lint;
mod tree;

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

impl AsRef<[u8]> for SharedSource {
    fn as_ref(&self) -> &[u8] {
        self.0.inner()
    }
}
