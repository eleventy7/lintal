//! Typed CST node wrappers for Java syntax trees.
//!
//! Provides strongly-typed access to tree-sitter nodes while preserving
//! source positions needed for fixes.

use lintal_text_size::{TextRange, TextSize};
use tree_sitter::Node;

/// Convert a tree-sitter node range to a TextRange.
pub fn node_range(node: &Node) -> TextRange {
    let start = TextSize::new(node.start_byte() as u32);
    let end = TextSize::new(node.end_byte() as u32);
    TextRange::new(start, end)
}

/// A token in the Java source (leaf node).
#[derive(Debug, Clone, Copy)]
pub struct Token<'a> {
    node: Node<'a>,
    source: &'a str,
}

impl<'a> Token<'a> {
    pub fn new(node: Node<'a>, source: &'a str) -> Self {
        Self { node, source }
    }

    pub fn text(&self) -> &'a str {
        self.node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    pub fn range(&self) -> TextRange {
        node_range(&self.node)
    }

    pub fn kind(&self) -> &'static str {
        self.node.kind()
    }
}

/// Wrapper for traversing CST nodes.
#[derive(Debug, Clone, Copy)]
pub struct CstNode<'a> {
    node: Node<'a>,
    source: &'a str,
}

impl<'a> CstNode<'a> {
    pub fn new(node: Node<'a>, source: &'a str) -> Self {
        Self { node, source }
    }

    pub fn kind(&self) -> &'static str {
        self.node.kind()
    }

    pub fn kind_id(&self) -> u16 {
        self.node.kind_id()
    }

    pub fn range(&self) -> TextRange {
        node_range(&self.node)
    }

    pub fn text(&self) -> &'a str {
        self.node.utf8_text(self.source.as_bytes()).unwrap_or("")
    }

    pub fn parent(&self) -> Option<CstNode<'a>> {
        self.node.parent().map(|n| CstNode::new(n, self.source))
    }

    pub fn children(&self) -> impl Iterator<Item = CstNode<'a>> + 'a {
        let source = self.source;
        let count = self.node.child_count();
        let node = self.node;
        (0..count).map(move |i| {
            let child = node.child(i as u32).unwrap();
            CstNode::new(child, source)
        })
    }

    pub fn child_by_field_name(&self, name: &str) -> Option<CstNode<'a>> {
        self.node
            .child_by_field_name(name)
            .map(|n| CstNode::new(n, self.source))
    }

    pub fn named_children(&self) -> impl Iterator<Item = CstNode<'a>> + 'a {
        self.children().filter(|c| c.node.is_named())
    }

    pub fn next_named_sibling(&self) -> Option<CstNode<'a>> {
        self.node
            .next_named_sibling()
            .map(|n| CstNode::new(n, self.source))
    }

    /// Get the raw tree-sitter node.
    pub fn inner(&self) -> Node<'a> {
        self.node
    }
}

/// Iterator for walking all nodes in a tree (pre-order traversal).
pub struct TreeWalker<'a> {
    cursor: tree_sitter::TreeCursor<'a>,
    source: &'a str,
    done: bool,
}

impl<'a> TreeWalker<'a> {
    pub fn new(root: Node<'a>, source: &'a str) -> Self {
        Self {
            cursor: root.walk(),
            source,
            done: false,
        }
    }
}

impl<'a> Iterator for TreeWalker<'a> {
    type Item = CstNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let node = CstNode::new(self.cursor.node(), self.source);

        // Try to go to first child
        if self.cursor.goto_first_child() {
            return Some(node);
        }

        // Try to go to next sibling
        if self.cursor.goto_next_sibling() {
            return Some(node);
        }

        // Go up until we can go to next sibling or reach root
        loop {
            if !self.cursor.goto_parent() {
                self.done = true;
                return Some(node);
            }
            if self.cursor.goto_next_sibling() {
                return Some(node);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_parser::JavaParser;

    #[test]
    fn test_cst_node_traversal() {
        let mut parser = JavaParser::new();
        let source = "class Foo { int x; }";
        let result = parser.parse(source).unwrap();
        let root = CstNode::new(result.tree.root_node(), source);

        assert_eq!(root.kind(), "program");
        let named_count = root.named_children().count();
        assert!(
            named_count > 0,
            "Expected named children, got {}",
            named_count
        );
    }

    #[test]
    fn test_tree_walker() {
        let mut parser = JavaParser::new();
        let source = "class Foo {}";
        let result = parser.parse(source).unwrap();

        let walker = TreeWalker::new(result.tree.root_node(), source);
        let nodes: Vec<_> = walker.collect();

        assert!(!nodes.is_empty());
        assert_eq!(nodes[0].kind(), "program");
    }
}
