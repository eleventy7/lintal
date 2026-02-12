//! InnerAssignment rule implementation.
//!
//! Checks for assignments in subexpressions (e.g. `if (a = b)`).
//! Assignments should only appear as standalone statements.
//!
//! Checkstyle equivalent: InnerAssignmentCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: inner assignment found.
#[derive(Debug, Clone)]
pub struct InnerAssignmentViolation;

impl Violation for InnerAssignmentViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Inner assignments should be avoided.".to_string()
    }
}

/// Configuration for InnerAssignment rule.
#[derive(Debug, Clone, Default)]
pub struct InnerAssignment;

const RELEVANT_KINDS: &[&str] = &["assignment_expression"];

impl FromConfig for InnerAssignment {
    const MODULE_NAME: &'static str = "InnerAssignment";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for InnerAssignment {
    fn name(&self) -> &'static str {
        "InnerAssignment"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "assignment_expression" {
            return vec![];
        }

        // Walk up from the assignment to see if it's a standalone statement
        let Some(parent) = node.parent() else {
            return vec![];
        };

        // Direct child of expression_statement → standalone assignment, no violation
        if parent.kind() == "expression_statement" {
            return vec![];
        }

        // Check if inside for-loop init or update positions
        if self.is_in_for_init_or_update(node) {
            return vec![];
        }

        // Check if inside resource_specification (try-with-resources)
        if self.is_in_resource_specification(node) {
            return vec![];
        }

        // Check if inside while/do-while condition (common idiom: while ((b = read()) != -1))
        if self.is_in_while_condition(node) {
            return vec![];
        }

        vec![Diagnostic::new(InnerAssignmentViolation, node.range())]
    }
}

impl InnerAssignment {
    /// Check if the assignment is in a for-loop initializer or update.
    fn is_in_for_init_or_update(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "for_statement" => return true,
                // Stop at statement boundaries
                "method_declaration"
                | "constructor_declaration"
                | "class_declaration"
                | "block" => {
                    return false;
                }
                _ => {}
            }
            current = parent.parent();
        }
        false
    }

    /// Check if the assignment is inside a while/do-while condition.
    /// This is a common idiom (e.g. `while ((b = is.read()) != -1)`) that checkstyle allows.
    fn is_in_while_condition(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "while_statement" | "do_statement" => return true,
                // Stop at statement/block boundaries — if we hit a block, the assignment
                // is in the body, not the condition
                "block"
                | "expression_statement"
                | "method_declaration"
                | "constructor_declaration"
                | "class_declaration" => {
                    return false;
                }
                _ => {}
            }
            current = parent.parent();
        }
        false
    }

    /// Check if the assignment is inside a resource_specification (try-with-resources).
    fn is_in_resource_specification(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "resource_specification" | "resource" => return true,
                // Stop at statement boundaries
                "method_declaration"
                | "constructor_declaration"
                | "class_declaration"
                | "block" => {
                    return false;
                }
                _ => {}
            }
            current = parent.parent();
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = InnerAssignment;
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push(loc.line.get());
            }
        }
        violations
    }

    #[test]
    fn test_standalone_assignment_no_violation() {
        let source = r#"
class Test {
    void method() {
        int a;
        a = 5;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_assignment_in_condition() {
        let source = r#"
class Test {
    void method() {
        int a;
        if ((a = getValue()) != 0) {}
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], 5);
    }

    #[test]
    fn test_for_loop_init_no_violation() {
        let source = r#"
class Test {
    void method() {
        int i;
        for (i = 0; i < 10; i++) {}
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_assignment_in_return() {
        let source = r#"
class Test {
    int method() {
        int a;
        return a = 5;
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_chained_assignment_inner_violation() {
        let source = r#"
class Test {
    void method() {
        int a, b;
        a = b = 5;
    }
}
"#;
        // b = 5 is an inner assignment (inside a = ...)
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }
}
