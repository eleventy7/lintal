//! Java parser for jruff, built on tree-sitter-java.

use std::sync::Arc;

/// Result of parsing a Java source file.
pub struct ParseResult {
    pub tree: tree_sitter::Tree,
    pub source: Arc<str>,
}

/// Java parser wrapping tree-sitter.
pub struct JavaParser {
    parser: tree_sitter::Parser,
}

impl JavaParser {
    /// Create a new Java parser.
    pub fn new() -> Self {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_java::LANGUAGE.into())
            .expect("Failed to load Java grammar");
        Self { parser }
    }

    /// Parse Java source code into a syntax tree.
    pub fn parse(&mut self, source: &str) -> Option<ParseResult> {
        let tree = self.parser.parse(source, None)?;
        Some(ParseResult {
            tree,
            source: source.into(),
        })
    }

    /// Parse with an existing tree for incremental parsing.
    pub fn parse_with_old_tree(
        &mut self,
        source: &str,
        old_tree: &tree_sitter::Tree,
    ) -> Option<ParseResult> {
        let tree = self.parser.parse(source, Some(old_tree))?;
        Some(ParseResult {
            tree,
            source: source.into(),
        })
    }
}

impl Default for JavaParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_class() {
        let mut parser = JavaParser::new();
        let source = r#"
public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;
        let result = parser.parse(source).expect("Failed to parse");
        assert_eq!(result.tree.root_node().kind(), "program");
    }

    #[test]
    fn test_parse_record() {
        let mut parser = JavaParser::new();
        let source = "public record Point(int x, int y) {}";
        let result = parser.parse(source).expect("Failed to parse");
        assert_eq!(result.tree.root_node().kind(), "program");
    }
}
