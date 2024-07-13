use std::cell::OnceCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::Metadata;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use miette::{MietteError, NamedSource, SourceCode, SpanContents};

use tree_sitter::{Language, Node, Parser, TreeCursor};

use crate::lint::{Lint, SpaceBeforePunctuationMarks};

mod lint;

fn for_each_node<'a>(mut c: TreeCursor<'a>, mut callback: impl FnMut(Node<'a>)) {
    loop {
        callback(c.node());

        if c.goto_first_child() {
            continue;
        }

        if c.goto_next_sibling() {
            continue;
        }

        loop {
            if !c.goto_parent() {
                return;
            }

            if c.goto_next_sibling() {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedSource(Arc<NamedSource<Vec<u8>>>);

impl SharedSource {
    pub fn new(name: impl AsRef<str>, bytes: impl AsRef<[u8]>) -> Self {
        Self(Arc::new(NamedSource::new(name, bytes.as_ref().to_owned())))
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

struct Lazy<T> {
    cell: OnceLock<T>,
    init: fn() -> T,
}

impl<T> Lazy<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            cell: OnceLock::new(),
            init,
        }
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &'_ T {
        self.cell.get_or_init(self.init)
    }
}

static EXTENSION_LANGUAGE: Lazy<HashMap<&'static OsStr, Language>> = Lazy::new(|| {
    let mut map = HashMap::new();

    #[cfg(feature = "lang-rust")]
    map.insert(OsStr::new("rs"), tree_sitter_rust::language());
    #[cfg(feature = "lang-cpp")]
    map.insert(OsStr::new("cpp"), tree_sitter_cpp::language());
    #[cfg(feature = "lang-c")]
    map.insert(OsStr::new("c"), tree_sitter_c::language());
    #[cfg(feature = "lang-go")]
    map.insert(OsStr::new("go"), tree_sitter_go::language());
    #[cfg(feature = "lang-python")]
    map.insert(OsStr::new("py"), tree_sitter_python::language());
    #[cfg(feature = "lang-toml")]
    map.insert(OsStr::new("toml"), tree_sitter_toml_ng::language());
    #[cfg(feature = "lang-yaml")]
    map.insert(OsStr::new("yml"), tree_sitter_yaml::language());
    #[cfg(feature = "lang-json")]
    map.insert(OsStr::new("json"), tree_sitter_json::language());

    map
});

fn main() -> anyhow::Result<()> {
    let valid_kinds = [
        "line_comment",
        "string_content",
        "string",
        "interpreted_string_literal",
        "string_scalar",
        "double_quote_scalar",
    ];
    let mut parser = Parser::new();

    for file in ignore::Walk::new(".")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .metadata()
                .as_ref()
                .map(Metadata::is_file)
                .unwrap_or(false)
        })
    {
        let extension = file.path().extension().unwrap_or_default();
        let Some(language) = EXTENSION_LANGUAGE.get(extension) else {
            continue;
        };
        parser.set_language(language)?;

        let source_content = std::fs::read(file.path())?;
        let Some(tree) = parser.parse(&source_content, None) else {
            continue;
        };

        let source = OnceCell::new();

        for_each_node(tree.walk(), |node| {
            let kind = node.kind();
            if !valid_kinds.contains(&kind) {
                return;
            }
            if node.byte_range().len() <= 3 {
                return;
            }
            let Some(string) = source_content.get(node.byte_range()) else {
                return;
            };

            let string = String::from_utf8_lossy(string);
            for typo in SpaceBeforePunctuationMarks::check(&string) {
                let source = source.get_or_init(|| {
                    SharedSource::new(file.path().to_string_lossy(), &source_content)
                });
                let typo = typo.with_source(source.clone(), node.start_byte());
                let typo: miette::Report = typo.into();
                eprintln!("{typo:?}");
            }
        });
    }

    Ok(())
}
