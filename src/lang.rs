use std::collections::HashMap;
use std::ffi::OsStr;
use std::ops::Deref;
use std::sync::{Arc, OnceLock};

use tree_sitter::{Node, Parser, Tree};

use crate::tree::PreorderTraversal;

#[cfg(feature = "lang-markdown")]
mod markdown;

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

static EXTENSION_LANGUAGE: Lazy<HashMap<&'static OsStr, Arc<Language>>> = Lazy::new(|| {
    let mut map = HashMap::new();

    macro_rules! lang {
        ($lang:ident, $feature: literal) => {
            #[cfg(feature = $feature)]
            {
                let lang = Arc::new(Language::$lang());
                for extension in lang.extensions() {
                    map.insert(OsStr::new(extension), Arc::clone(&lang));
                }
            }
        };
    }

    lang!(rust, "lang-rust");
    lang!(c, "lang-c");
    lang!(cpp, "lang-cpp");
    lang!(go, "lang-go");
    lang!(python, "lang-python");
    lang!(toml, "lang-toml");
    lang!(yaml, "lang-yaml");
    lang!(json, "lang-json");
    lang!(markdown, "lang-markdown");

    map
});

type CustomParser = Box<dyn Fn(&[u8]) -> anyhow::Result<Box<dyn Parsed>> + Send + Sync>;

/// Parser for a language to find strings based on its grammar
pub struct Language {
    language: tree_sitter::Language,
    extensions: &'static [&'static str],
    tree_sitter_types: &'static [&'static str],
    parser: Option<CustomParser>,
}

impl Language {
    /// Find the language to parse based on a file extension
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use orthotypos::lang::Language;
    /// assert!(Language::from_extension(OsStr::new("rs")).is_some());
    /// ```
    pub fn from_extension(extension: &OsStr) -> Option<Arc<Self>> {
        EXTENSION_LANGUAGE.get(extension).map(Arc::clone)
    }

    /// Returns an array of extensions supported by this language
    ///
    /// # Example
    ///
    /// ```
    /// # use std::ffi::OsStr;
    /// #
    /// # use orthotypos::lang::Language;
    /// let rust = Language::from_extension(OsStr::new("rs")).unwrap();
    /// assert_eq!(rust.extensions(), &["rs"]);
    /// ```
    pub fn extensions(&self) -> &'static [&'static str] {
        self.extensions
    }

    /// Parses the content of a file
    pub fn parse(&self, source_content: impl AsRef<[u8]>) -> anyhow::Result<Box<dyn Parsed>> {
        if let Some(parser) = &self.parser {
            Ok(parser(source_content.as_ref())?)
        } else {
            let mut parser = Parser::new();
            parser.set_language(&self.language)?;
            let Some(tree) = parser.parse(source_content, None) else {
                anyhow::bail!("Invalid language");
            };

            Ok(Box::new(ParsedGeneric {
                tree,
                tree_sitter_types: self.tree_sitter_types,
            }))
        }
    }

    /// Creates a language parser for Rust
    #[cfg(feature = "lang-rust")]
    pub fn rust() -> Self {
        Self {
            language: tree_sitter_rust::language(),
            extensions: &["rs"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }

    /// Creates a language parser for C++
    #[cfg(feature = "lang-cpp")]
    pub fn cpp() -> Self {
        Self {
            language: tree_sitter_cpp::language(),
            extensions: &["cpp", "cc", "hpp", "hh"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }

    /// Creates a language parser for C
    #[cfg(feature = "lang-c")]
    pub fn c() -> Self {
        Self {
            language: tree_sitter_c::language(),
            extensions: &["c", "h"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }

    /// Creates a language parser for Go
    #[cfg(feature = "lang-go")]
    pub fn go() -> Self {
        Self {
            language: tree_sitter_go::language(),
            extensions: &["go"],
            tree_sitter_types: &["interpreted_string_literal"],
            parser: None,
        }
    }

    /// Creates a language parser for Python
    #[cfg(feature = "lang-python")]
    pub fn python() -> Self {
        Self {
            language: tree_sitter_python::language(),
            extensions: &["py"],
            tree_sitter_types: &["string", "concatenated_string"],
            parser: None,
        }
    }

    /// Creates a language parser for TOML
    #[cfg(feature = "lang-toml")]
    pub fn toml() -> Self {
        Self {
            language: tree_sitter_toml_ng::language(),
            extensions: &["toml"],
            tree_sitter_types: &["string"],
            parser: None,
        }
    }

    /// Creates a language parser for YAML
    #[cfg(feature = "lang-yaml")]
    pub fn yaml() -> Self {
        Self {
            language: tree_sitter_yaml::language(),
            extensions: &["yml", "yaml"],
            tree_sitter_types: &["string_scalar"],
            parser: None,
        }
    }

    /// Creates a language parser for JSON
    #[cfg(feature = "lang-json")]
    pub fn json() -> Self {
        Self {
            language: tree_sitter_json::language(),
            extensions: &["json"],
            tree_sitter_types: &["string_content"],
            parser: None,
        }
    }

    /// Creates a language parser for Markdown
    #[cfg(feature = "lang-markdown")]
    pub fn markdown() -> Self {
        markdown::lang()
    }
}

/// Wrapper around a [Node] to make it easier to ignore ranges of bytes based on some children
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
                Some(current_range_start..self.node.byte_range().end)
            }
        })
    }

    /// Returns an iterator over the bytes of the node that have not been ignored
    pub fn lintable_bytes<'a, 'b>(&'a self, bytes: &'b [u8]) -> impl Iterator<Item = &'b [u8]> + 'a
    where
        'b: 'a,
    {
        self.lintable_ranges()
            .filter_map(move |range| bytes.get(range))
    }

    /// Byte offset where this node start
    pub fn start_byte(&self) -> usize {
        self.node.start_byte()
    }

    /// Byte range of source code that this node represents
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        self.node.byte_range()
    }

    /// Node's immediate parent
    pub fn parent(&self) -> Option<Node<'t>> {
        self.node.parent()
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

pub struct ParsedGeneric {
    tree: Tree,
    tree_sitter_types: &'static [&'static str],
}

impl Parsed for ParsedGeneric {
    fn iter<'t>(&'t self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
        Box::new(Iter::new(self))
    }
}

pub trait Parsed {
    fn iter<'t>(&'t self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't>;
}

pub struct Iter<'t> {
    traversal: PreorderTraversal<'t>,
    tree_sitter_types: &'static [&'static str],
}

impl<'t> Iter<'t> {
    fn new(parsed: &'t ParsedGeneric) -> Self {
        Self {
            traversal: PreorderTraversal::from(parsed.tree.walk()),
            tree_sitter_types: parsed.tree_sitter_types,
        }
    }
}

impl<'t> Iterator for Iter<'t> {
    type Item = LintableNode<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.traversal.next().map(LintableNode::from)?;
            let kind = node.kind();
            if node.byte_range().len() <= 3 {
                continue;
            }

            if !self.tree_sitter_types.contains(&kind) {
                continue;
            }

            return Some(node);
        }
    }
}
