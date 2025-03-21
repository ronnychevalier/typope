//! Parsers to find strings in various source code files
use std::collections::HashSet;
use std::ffi::OsStr;
use std::sync::Arc;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use tree_sitter::{Node, Parser, Query, QueryCursor, Tree};

use crate::lock::LazyLock;
use crate::tree::PreorderTraversal;
use crate::SharedSource;

#[cfg(feature = "lang-c")]
mod c;
mod cargo_toml;
#[cfg(feature = "lang-cpp")]
mod cpp;
#[cfg(feature = "lang-go")]
mod go;
#[cfg(feature = "lang-javascript")]
mod javascript;
#[cfg(feature = "lang-json")]
mod json;
#[cfg(feature = "lang-kotlin")]
mod kotlin;
#[cfg(feature = "lang-markdown")]
mod markdown;
#[cfg(feature = "lang-python")]
mod python;
#[cfg(feature = "lang-rust")]
mod rust;
#[cfg(feature = "lang-toml")]
mod toml;
#[cfg(feature = "lang-typescript")]
mod typescript;
#[cfg(feature = "lang-yaml")]
mod yaml;

struct Mapping {
    glob_set: GlobSet,
    glob_to_lang: Vec<Arc<Language>>,
    languages: Vec<Arc<Language>>,
}

impl Mapping {
    pub fn build() -> Self {
        let mut languages = Vec::new();
        let mut glob_set = GlobSetBuilder::new();
        let mut glob_to_lang = Vec::new();

        macro_rules! lang {
            ($lang:ident) => {
                let lang = Arc::new(Language::$lang());
                for glob in lang.detections() {
                    let Ok(glob) = GlobBuilder::new(glob).literal_separator(true).build() else {
                        continue;
                    };
                    glob_set.add(glob);
                    glob_to_lang.push(Arc::clone(&lang));
                }
                languages.push(lang);
            };
            ($lang:ident, $feature: literal) => {
                #[cfg(feature = $feature)]
                {
                    lang!($lang);
                }
            };
        }

        lang!(rust, "lang-rust");
        lang!(c, "lang-c");
        lang!(cpp, "lang-cpp");
        lang!(kotlin, "lang-kotlin");
        lang!(go, "lang-go");
        lang!(python, "lang-python");
        lang!(toml, "lang-toml");
        lang!(yaml, "lang-yaml");
        lang!(json, "lang-json");
        lang!(javascript, "lang-javascript");
        lang!(typescript, "lang-typescript");
        lang!(markdown, "lang-markdown");
        // Takes precedence over the generic toml parser, so it needs to be last in the insertion order
        lang!(cargo_toml);

        let glob_set = glob_set.build().unwrap_or_default();

        Self {
            glob_set,
            glob_to_lang,
            languages,
        }
    }

    pub fn find_from_filename(&self, filename: &OsStr) -> Option<&Language> {
        let matches = self.glob_set.matches(filename);
        let i = matches.last()?;

        self.glob_to_lang.get(*i).map(AsRef::as_ref)
    }
}

static MAPPING: LazyLock<Mapping> = LazyLock::new(Mapping::build);

type CustomParser = Box<dyn Fn(&[u8]) -> anyhow::Result<Box<dyn Parsed>> + Send + Sync>;

/// Defines how to parse this language to find relevant strings
enum Mode {
    /// Parse the language using a generic parser that iterates over strings with a given node type
    Generic {
        language: tree_sitter::Language,
        tree_sitter_types: &'static [&'static str],
    },

    /// Parse the language using a custom parser
    Custom(CustomParser),

    /// Parse the language using a query
    Query {
        language: tree_sitter::Language,
        query: String,
        ignore_captures: Option<&'static [&'static str]>,
    },
}

/// Parser for a language to find strings based on its grammar
pub struct Language {
    name: &'static str,
    detections: &'static [&'static str],
    parser: Mode,
}

impl Language {
    /// Finds the language to parse based on a file name
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use typope::lang::Language;
    /// assert!(Language::from_filename(OsStr::new("file.rs")).is_some());
    /// ```
    pub fn from_filename(filename: &OsStr) -> Option<&Self> {
        MAPPING.find_from_filename(filename)
    }

    /// Returns an array of glob patterns of files supported by this language
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use typope::lang::Language;
    /// let rust = Language::from_filename(OsStr::new("file.rs")).unwrap();
    /// assert_eq!(rust.detections(), &["*.rs"]);
    /// ```
    pub fn detections(&self) -> &'static [&'static str] {
        self.detections
    }

    /// Returns the name of the language
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use typope::lang::Language;
    /// let rust = Language::from_filename(OsStr::new("file.rs")).unwrap();
    /// assert_eq!(rust.name(), "rust");
    /// ```
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Returns an iterator over the supported languages
    pub fn iter() -> impl Iterator<Item = &'static Self> {
        MAPPING.languages.iter().map(AsRef::as_ref)
    }

    /// Parses the content of a file
    pub fn parse(&self, source: &SharedSource) -> anyhow::Result<Box<dyn Parsed>> {
        match &self.parser {
            Mode::Generic {
                language,
                tree_sitter_types,
            } => {
                let mut parser = Parser::new();
                parser.set_language(language)?;
                let Some(tree) = parser.parse(source, None) else {
                    anyhow::bail!("Invalid language");
                };

                Ok(Box::new(ParsedGeneric {
                    tree,
                    tree_sitter_types,
                }))
            }
            Mode::Custom(parser) => Ok(parser(source.as_ref())?),
            Mode::Query {
                language,
                query,
                ignore_captures,
            } => {
                let mut parser: Parser = Parser::new();
                parser.set_language(language)?;
                let Some(tree) = parser.parse(source.as_ref(), None) else {
                    anyhow::bail!("Invalid language");
                };
                let query = Query::new(language, query)?;

                Ok(Box::new(ParsedQuery {
                    tree,
                    query,
                    ignore_captures: *ignore_captures,
                    source: source.clone(),
                    cursor: QueryCursor::new(),
                    ignored_nodes: HashSet::new(),
                }))
            }
        }
    }
}

struct ParsedQuery {
    tree: Tree,
    query: Query,
    cursor: QueryCursor,
    source: SharedSource,
    ignore_captures: Option<&'static [&'static str]>,
    ignored_nodes: HashSet<usize>,
}

impl Parsed for ParsedQuery {
    fn lintable_nodes<'t>(&'t mut self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
        let nodes = self
            .cursor
            .matches(&self.query, self.tree.root_node(), self.source.as_ref())
            .flat_map(|m| m.captures.iter())
            .filter_map(|capture| {
                if capture.node.byte_range().len() <= 3 {
                    return None;
                }

                if let Some(ignore_captures) = self.ignore_captures {
                    if self.ignored_nodes.contains(&capture.node.id()) {
                        return None;
                    }
                    let name = self.query.capture_names().get(capture.index as usize)?;
                    if ignore_captures.contains(name) {
                        self.ignored_nodes.insert(capture.node.id());
                        return None;
                    }
                }

                Some(LintableNode::from(capture.node))
            });
        Box::new(nodes)
    }
}

/// A string that can be checked with its offset within its source
#[derive(PartialEq, Eq, Debug)]
pub struct LintableString {
    offset: usize,
    value: String,
}

impl LintableString {
    /// Returns the string that can be checked for typos
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Offset of the string within its source
    pub fn offset(&self) -> usize {
        self.offset
    }
}

impl From<LintableString> for String {
    fn from(lintable: LintableString) -> Self {
        lintable.value
    }
}

impl From<&::toml::Spanned<String>> for LintableString {
    fn from(spanned: &::toml::Spanned<String>) -> Self {
        Self {
            offset: spanned.span().start + 1,
            value: spanned.get_ref().clone(),
        }
    }
}

/// Wrapper around a [`Node`] to make it easier to ignore ranges of bytes based on some children
pub struct LintableNode<'t> {
    node: Node<'t>,
    ignore_nodes: Vec<Node<'t>>,
}

impl<'t> LintableNode<'t> {
    /// Selects the children ranges that are ignored
    pub fn ignore_children_ranges(mut self, f: impl Fn(&Node<'t>) -> bool) -> Self {
        let mut cursor = self.node.walk();
        self.ignore_nodes = self.node.children(&mut cursor).filter(f).collect();

        self
    }

    /// Node's type
    pub fn kind(&self) -> &'static str {
        self.node.kind()
    }

    fn lintable_ranges(&self) -> impl Iterator<Item = std::ops::Range<usize>> + '_ {
        let mut current_range_start = self.node.byte_range().start;
        let mut iter = self.ignore_nodes.iter();
        let mut ended = false;
        std::iter::from_fn(move || {
            if ended {
                return None;
            }

            if let Some(ignore_node) = iter.next() {
                let start = ignore_node.start_byte();
                let end = ignore_node.end_byte();
                let range = current_range_start..start;
                current_range_start = end;
                Some(range)
            } else {
                ended = true;
                if (current_range_start..self.node.byte_range().end).is_empty() {
                    None
                } else {
                    Some(current_range_start..self.node.byte_range().end)
                }
            }
        })
    }

    /// Returns an iterator over the strings of the node that have not been ignored
    pub fn lintable_strings<'a, 'b>(
        &'a self,
        bytes: &'b [u8],
    ) -> impl Iterator<Item = LintableString> + 'a
    where
        'b: 'a,
    {
        self.lintable_ranges().filter_map(move |range| {
            let offset = range.start;
            let bytes = bytes.get(range)?;
            let string = String::from_utf8_lossy(bytes).into_owned();

            Some(LintableString {
                offset,
                value: string,
            })
        })
    }

    /// Byte range of source code that this node represents
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.node.byte_range()
    }
}

impl<'t> From<Node<'t>> for LintableNode<'t> {
    fn from(node: Node<'t>) -> Self {
        Self {
            node,
            ignore_nodes: Vec::new(),
        }
    }
}

/// Type that represents a file that has been parsed
pub trait Parsed {
    /// Returns an iterator over the lintable nodes based on the language grammar
    fn lintable_nodes<'t>(&'t mut self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't>;

    /// Returns an iterator over the strings found in the source based on the language grammar
    fn strings<'t>(
        &'t mut self,
        source: &'t [u8],
    ) -> Box<dyn Iterator<Item = LintableString> + 't> {
        Box::new(
            self.lintable_nodes()
                .flat_map(|node| node.lintable_strings(source).collect::<Vec<_>>()),
        )
    }
}

struct ParsedGeneric {
    tree: Tree,
    tree_sitter_types: &'static [&'static str],
}

impl Parsed for ParsedGeneric {
    fn lintable_nodes<'t>(&'t mut self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
        Box::new(
            PreorderTraversal::from(self.tree.walk()).filter_map(|node| {
                if node.byte_range().len() <= 3 {
                    return None;
                }

                if !self.tree_sitter_types.contains(&node.kind()) {
                    return None;
                }

                Some(LintableNode::from(node))
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::Language;

    #[test]
    fn unknown_file_type() {
        assert!(
            Language::from_filename(OsStr::new("file.withextensionthatdoesnotexist")).is_none()
        );
    }
}
