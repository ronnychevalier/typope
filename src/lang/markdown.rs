use std::iter::FlatMap;

use btree_range_map::RangeSet;

use tree_sitter::Tree;

use tree_sitter_md::MarkdownTree;

use crate::tree::PreorderTraversal;

use super::{Language, LintableNode, Mode, Parsed};

/// Parser for Markdown that helps to ignore text in code span
struct ParsedMarkdown {
    tree: MarkdownTree,
    tree_sitter_types: &'static [&'static str],
}

impl ParsedMarkdown {
    pub fn new(text: impl AsRef<[u8]>) -> anyhow::Result<Self> {
        let mut parser = tree_sitter_md::MarkdownParser::default();
        let Some(tree) = parser.parse(text.as_ref(), None) else {
            anyhow::bail!("Invalid language");
        };

        Ok(Self {
            tree,
            tree_sitter_types: &["inline"],
        })
    }
}

impl Parsed for ParsedMarkdown {
    fn lintable_nodes<'t>(&'t mut self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
        Box::new(IterMarkdown::new(self))
    }
}

type MarkdownTraversal<'t> = FlatMap<
    core::slice::Iter<'t, Tree>,
    PreorderTraversal<'t>,
    fn(&'t Tree) -> PreorderTraversal<'t>,
>;

pub struct IterMarkdown<'t> {
    traversals: MarkdownTraversal<'t>,
    tree_sitter_types: &'static [&'static str],
    block_quote_ranges: RangeSet<usize>,
}

impl<'t> IterMarkdown<'t> {
    fn new(parsed: &'t ParsedMarkdown) -> Self {
        let block_quote_ranges = PreorderTraversal::from(parsed.tree.block_tree())
            .filter_map(|node| {
                if node.kind() != "block_quote" {
                    return None;
                }

                Some(node.byte_range())
            })
            .collect::<RangeSet<_>>();
        let traversals = parsed
            .tree
            .inline_trees()
            .iter()
            .flat_map(PreorderTraversal::from as _);
        Self {
            traversals,
            tree_sitter_types: parsed.tree_sitter_types,
            block_quote_ranges,
        }
    }
}

impl<'t> Iterator for IterMarkdown<'t> {
    type Item = LintableNode<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.traversals.next()?;
            if self.block_quote_ranges.intersects(node.byte_range()) {
                continue;
            }

            let kind = node.kind();
            if node.byte_range().len() <= 3 {
                continue;
            }

            if !self.tree_sitter_types.contains(&kind) {
                continue;
            }

            let node = LintableNode::from(node);
            let node =
                node.ignore_children_ranges(|node| ["code_span", "image"].contains(&node.kind()));

            return Some(node);
        }
    }
}

impl Language {
    /// Creates a language parser for Markdown
    pub fn markdown() -> Self {
        Self {
            name: "markdown",
            detections: &["*.md"],
            parser: Mode::Custom(Box::new(move |text| {
                Ok(Box::new(ParsedMarkdown::new(text)?))
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use crate::lang::LintableString;
    use crate::SharedSource;

    use super::Language;

    #[test]
    fn exists_in_iter() {
        assert!(Language::iter().any(|lang| lang.name() == "markdown"));
    }

    #[test]
    fn find_from_filename() {
        assert_eq!(
            "markdown",
            Language::from_filename(OsStr::new("README.md"))
                .unwrap()
                .name()
        );
    }

    #[test]
    fn lintable_strings() {
        let markdown = r"# Hello
This is a text `with some` code_span in `various` places
```
what about this
```
hello
";
        let markdown = SharedSource::new("file.md", markdown.as_bytes().to_vec());
        let mut parsed = Language::markdown().parse(&markdown).unwrap();
        let strings = parsed.strings(markdown.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 2,
                    value: "Hello".into()
                },
                LintableString {
                    offset: 8,
                    value: "This is a text ".into()
                },
                LintableString {
                    offset: 34,
                    value: " code_span in ".into()
                },
                LintableString {
                    offset: 57,
                    value: " places".into()
                },
                LintableString {
                    offset: 89,
                    value: "hello".into()
                }
            ]
        );
    }

    #[test]
    fn image() {
        let markdown = r"abc ![link](link)";
        let markdown = SharedSource::new("file.md", markdown.as_bytes().to_vec());
        let mut parsed = Language::markdown().parse(&markdown).unwrap();
        let strings = parsed.strings(markdown.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [LintableString {
                offset: 0,
                value: "abc ".into()
            },]
        );
    }

    #[test]
    fn block_quote() {
        let markdown = r"# Block Quotes

> Should not be lintable
> > This line as well
> > And this one

Something else `hmm`
";
        let markdown = SharedSource::new("file.md", markdown.as_bytes().to_vec());
        let mut parsed = Language::markdown().parse(&markdown).unwrap();
        let strings = parsed.strings(markdown.as_ref()).collect::<Vec<_>>();
        assert_eq!(
            strings,
            [
                LintableString {
                    offset: 2,
                    value: "Block Quotes".into()
                },
                LintableString {
                    offset: 81,
                    value: "Something else ".into()
                },
            ]
        );
    }
}
