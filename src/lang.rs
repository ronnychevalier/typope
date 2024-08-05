use std::collections::HashMap;
use std::ffi::OsStr;
use std::sync::{Arc, LazyLock};

use tree_sitter::{Node, Parser, Query, QueryCursor, Tree};

use crate::tree::PreorderTraversal;
use crate::SharedSource;

#[cfg(feature = "lang-c")]
mod c;
#[cfg(feature = "lang-cpp")]
mod cpp;
#[cfg(feature = "lang-go")]
mod go;
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
#[cfg(feature = "lang-yaml")]
mod yaml;

struct Mapping {
    lang_from_extensions: HashMap<&'static OsStr, Arc<Language>>,
    languages: Vec<Arc<Language>>,
}

impl Mapping {
    pub fn build() -> Self {
        let mut lang_from_extensions = HashMap::new();
        let mut languages = Vec::new();

        macro_rules! lang {
            ($lang:ident, $feature: literal) => {
                #[cfg(feature = $feature)]
                {
                    let lang = Arc::new(Language::$lang());
                    for extension in lang.extensions() {
                        lang_from_extensions.insert(OsStr::new(extension), Arc::clone(&lang));
                    }
                    languages.push(lang);
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
        lang!(markdown, "lang-markdown");

        Self {
            lang_from_extensions,
            languages,
        }
    }

    pub fn find_from_extension(&self, extension: &OsStr) -> Option<&Language> {
        self.lang_from_extensions.get(extension).map(AsRef::as_ref)
    }
}

static MAPPING: LazyLock<Mapping> = LazyLock::new(Mapping::build);

type CustomParser = Box<dyn Fn(&[u8]) -> anyhow::Result<Box<dyn Parsed>> + Send + Sync>;

/// Defines how to parse this language to find relevant strings
enum Mode {
    /// Parse the language using a generic parser that iterates over strings with a given node type
    Generic {
        tree_sitter_types: &'static [&'static str],
    },

    /// Parse the language using a custom parser
    Custom(CustomParser),

    /// Parse the language using a query
    Query(String),
}

/// Parser for a language to find strings based on its grammar
pub struct Language {
    name: &'static str,
    language: tree_sitter::Language,
    extensions: &'static [&'static str],
    parser: Mode,
}

impl Language {
    /// Finds the language to parse based on a file extension
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use typope::lang::Language;
    /// assert!(Language::from_extension(OsStr::new("rs")).is_some());
    /// ```
    pub fn from_extension(extension: &OsStr) -> Option<&Self> {
        MAPPING.find_from_extension(extension)
    }

    /// Returns an array of extensions supported by this language
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use typope::lang::Language;
    /// let rust = Language::from_extension(OsStr::new("rs")).unwrap();
    /// assert_eq!(rust.extensions(), &["rs"]);
    /// ```
    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    /// Returns the name of the language
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
            Mode::Generic { tree_sitter_types } => {
                let mut parser = Parser::new();
                parser.set_language(&self.language)?;
                let Some(tree) = parser.parse(source, None) else {
                    anyhow::bail!("Invalid language");
                };

                Ok(Box::new(ParsedGeneric {
                    tree,
                    tree_sitter_types,
                }))
            }
            Mode::Custom(parser) => Ok(parser(source.as_ref())?),
            Mode::Query(query) => {
                let mut parser = Parser::new();
                parser.set_language(&self.language)?;
                let Some(tree) = parser.parse(source.as_ref(), None) else {
                    anyhow::bail!("Invalid language");
                };
                let query = Query::new(&self.language, query)?;

                Ok(Box::new(ParsedQuery {
                    tree,
                    query,
                    source: source.clone(),
                    cursor: QueryCursor::new(),
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

                Some(LintableNode::from(capture.node))
            });
        Box::new(nodes)
    }
}

/// A string that can be checked with its offset within its source
#[derive(PartialEq, Eq, Debug)]
pub struct LintableString {
    pub(crate) offset: usize,
    pub(crate) value: String,
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
    fn from_extension_invalid() {
        assert!(Language::from_extension(OsStr::new("extension that does not exist")).is_none());
    }
}
