//! Java parser for lintal, built on tree-sitter-java.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

/// Result of parsing a Java source file.
pub struct ParseResult {
    pub tree: tree_sitter::Tree,
    pub source: Arc<str>,
}

/// Java parser wrapping tree-sitter.
pub struct JavaParser {
    parser: tree_sitter::Parser,
}

/// Return the tree-sitter Java language.
pub fn java_language() -> tree_sitter::Language {
    tree_sitter_java::LANGUAGE.into()
}

/// Return a map from node kind string to one or more kind IDs.
pub fn java_kind_id_map() -> &'static HashMap<&'static str, Vec<u16>> {
    static KIND_ID_MAP: OnceLock<HashMap<&'static str, Vec<u16>>> = OnceLock::new();

    KIND_ID_MAP.get_or_init(|| {
        let language = java_language();
        let mut map: HashMap<&'static str, Vec<u16>> = HashMap::new();
        let kind_count = language.node_kind_count();

        for id in 0..kind_count {
            let id = id as u16;
            if let Some(kind) = language.node_kind_for_id(id) {
                map.entry(kind).or_default().push(id);
            }
        }

        map
    })
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
