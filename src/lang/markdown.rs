use std::iter::FlatMap;

use tree_sitter::Tree;

use tree_sitter_md::MarkdownTree;

use crate::tree::PreorderTraversal;

use super::{Lang, LintableNode, Parsed};

const TREE_SITTER_TYPES: &[&str] = &["inline"];

pub fn lang() -> Lang {
    Lang {
        language: tree_sitter_md::language(),
        extensions: &["md"],
        tree_sitter_types: TREE_SITTER_TYPES,
        parser: Some(Box::new(move |text| {
            Ok(Box::new(ParsedMarkdown::new(text)?))
        })),
    }
}

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
            tree_sitter_types: TREE_SITTER_TYPES,
        })
    }
}

impl Parsed for ParsedMarkdown {
    fn iter<'t>(&'t self) -> Box<dyn Iterator<Item = LintableNode<'t>> + 't> {
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
}

impl<'t> IterMarkdown<'t> {
    fn new(parsed: &'t ParsedMarkdown) -> Self {
        let traversals = parsed
            .tree
            .inline_trees()
            .iter()
            .flat_map(PreorderTraversal::from as _);
        Self {
            traversals,
            tree_sitter_types: parsed.tree_sitter_types,
        }
    }
}

impl<'t> Iterator for IterMarkdown<'t> {
    type Item = LintableNode<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = self.traversals.next().map(LintableNode::from)?;
            let kind = node.kind();
            if node.byte_range().len() <= 3 {
                continue;
            }

            if !self.tree_sitter_types.contains(&kind) {
                continue;
            }
            let node = node.ignore_children_ranges(|node| node.kind() == "code_span");

            return Some(node);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::lang::Parsed;

    use super::ParsedMarkdown;

    #[test]
    fn lintable_bytes() {
        let markdown = r#"# Hello
This is a text `with some` code_span in `various` places
```
what about this
```
hello
"#;
        let parsed = ParsedMarkdown::new(markdown).unwrap();
        let mut iter = parsed.iter();
        let node = iter.next().unwrap();
        assert_eq!(
            node.lintable_bytes(markdown.as_bytes()).collect::<Vec<_>>(),
            [&b"Hello"[..]]
        );

        let node = iter.next().unwrap();
        assert_eq!(
            node.lintable_bytes(markdown.as_bytes()).collect::<Vec<_>>(),
            [
                &b"This is a text "[..],
                &b" code_span in "[..],
                &b" places"[..],
            ]
        );

        let node = iter.next().unwrap();
        assert_eq!(
            node.lintable_bytes(markdown.as_bytes()).collect::<Vec<_>>(),
            [&b"hello"[..]]
        );

        assert!(iter.next().is_none());
    }
}
