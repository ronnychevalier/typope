use tree_sitter::{Node, Tree, TreeCursor};

/// Traverse a tree with a depth-first search from left to right
pub struct PreorderTraversal<'a> {
    cursor: TreeCursor<'a>,
    finished: bool,
}

impl<'a> Iterator for PreorderTraversal<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        let node = self.cursor.node();

        if self.cursor.goto_first_child() || self.cursor.goto_next_sibling() {
            return Some(node);
        }

        loop {
            if !self.cursor.goto_parent() {
                self.finished = true;
                break;
            }

            if self.cursor.goto_next_sibling() {
                break;
            }
        }

        Some(node)
    }
}

impl<'a> From<TreeCursor<'a>> for PreorderTraversal<'a> {
    fn from(cursor: TreeCursor<'a>) -> Self {
        Self {
            cursor,
            finished: false,
        }
    }
}

impl<'a> From<&'a Tree> for PreorderTraversal<'a> {
    fn from(tree: &'a Tree) -> Self {
        Self::from(tree.walk())
    }
}
