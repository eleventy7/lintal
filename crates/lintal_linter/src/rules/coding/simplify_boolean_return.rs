//! SimplifyBooleanReturn rule implementation.
//!
//! Checks for overly complicated boolean return statements.
//!
//! Checkstyle equivalent: SimplifyBooleanReturnCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: conditional logic can be removed.
#[derive(Debug, Clone)]
pub struct SimplifyBooleanReturnViolation;

impl Violation for SimplifyBooleanReturnViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "Conditional logic can be removed.".to_string()
    }
}

/// Configuration for SimplifyBooleanReturn rule.
#[derive(Debug, Clone, Default)]
pub struct SimplifyBooleanReturn;

const RELEVANT_KINDS: &[&str] = &["if_statement"];

impl FromConfig for SimplifyBooleanReturn {
    const MODULE_NAME: &'static str = "SimplifyBooleanReturn";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for SimplifyBooleanReturn {
    fn name(&self) -> &'static str {
        "SimplifyBooleanReturn"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "if_statement" {
            return vec![];
        }

        let ts_node = node.inner();

        // Must have an else clause
        let Some(alternative) = ts_node.child_by_field_name("alternative") else {
            return vec![];
        };

        let Some(consequence) = ts_node.child_by_field_name("consequence") else {
            return vec![];
        };

        // Check if consequence returns a boolean literal
        let Some(then_literal) = Self::get_single_boolean_return(&consequence) else {
            return vec![];
        };

        // Get the actual else body (might be a block or a bare statement)
        let else_body = if alternative.kind() == "return_statement" {
            // Bare return statement without braces
            alternative
        } else if alternative.kind() == "block" {
            alternative
        } else {
            // It's an else clause, get its body child (block or statement)
            let mut cursor = alternative.walk();
            alternative
                .children(&mut cursor)
                .find(|c| c.kind() == "block" || c.kind() == "return_statement")
                .unwrap_or(alternative)
        };

        // Check if alternative returns a boolean literal
        let Some(else_literal) = Self::get_single_boolean_return(&else_body) else {
            return vec![];
        };

        // Both return boolean literals - this is a violation
        if (then_literal && !else_literal) || (!then_literal && else_literal) {
            let range = lintal_text_size::TextRange::new(
                lintal_text_size::TextSize::from(ts_node.start_byte() as u32),
                lintal_text_size::TextSize::from(ts_node.end_byte() as u32),
            );
            return vec![Diagnostic::new(SimplifyBooleanReturnViolation, range)];
        }

        vec![]
    }
}

impl SimplifyBooleanReturn {
    /// Check if a block or statement is a single return statement with a boolean literal.
    /// Returns Some(true) for `return true`, Some(false) for `return false`, None otherwise.
    fn get_single_boolean_return(node: &tree_sitter::Node) -> Option<bool> {
        // Handle bare return statement
        if node.kind() == "return_statement" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "true" => return Some(true),
                    "false" => return Some(false),
                    _ => continue,
                }
            }
            return None;
        }

        // Handle block with return statement
        let mut cursor = node.walk();
        let children: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| !c.is_extra() && c.kind() != "{" && c.kind() != "}")
            .collect();

        // Must have exactly one statement
        if children.len() != 1 {
            return None;
        }

        let stmt = &children[0];
        if stmt.kind() != "return_statement" {
            return None;
        }

        // Get the return value
        let mut stmt_cursor = stmt.walk();
        for child in stmt.children(&mut stmt_cursor) {
            match child.kind() {
                "true" => return Some(true),
                "false" => return Some(false),
                _ => continue,
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = SimplifyBooleanReturn;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_if_true_else_false_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond) {
        if (cond) {
            return true;
        } else {
            return false;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "if true else false should be violation"
        );
    }

    #[test]
    fn test_if_false_else_true_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond) {
        if (cond) {
            return false;
        } else {
            return true;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "if false else true should be violation"
        );
    }

    #[test]
    fn test_no_else_no_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond) {
        if (cond) {
            return true;
        }
        return false;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "No else clause should not be violation"
        );
    }

    #[test]
    fn test_non_literal_no_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond, boolean other) {
        if (cond) {
            return true;
        } else {
            return other;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Non-literal return should not be violation"
        );
    }

    #[test]
    fn test_simple_return_no_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond) {
        return cond;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Simple return should not be violation"
        );
    }

    #[test]
    fn test_bare_return_statements_violation() {
        let source = r#"
class Test {
    boolean method(boolean cond) {
        if (!cond)
            return true;
        else
            return false;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Bare return statements should be violation"
        );
    }
}
