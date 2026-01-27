//! StringLiteralEquality rule implementation.
//!
//! Checks for string literal comparisons using == or !=, which should use
//! equals() instead for proper string comparison in Java.
//!
//! Checkstyle equivalent: StringLiteralEqualityCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use tree_sitter::Node;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: string literals should be compared with equals(), not ==.
#[derive(Debug, Clone)]
pub struct StringLiteralEqualityViolation;

impl Violation for StringLiteralEqualityViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "Literal Strings should be compared using equals(), not '=='.".to_string()
    }
}

/// Configuration for StringLiteralEquality rule.
#[derive(Debug, Clone, Default)]
pub struct StringLiteralEquality;

const RELEVANT_KINDS: &[&str] = &["binary_expression"];

impl FromConfig for StringLiteralEquality {
    const MODULE_NAME: &'static str = "StringLiteralEquality";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for StringLiteralEquality {
    fn name(&self) -> &'static str {
        "StringLiteralEquality"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "binary_expression" {
            return vec![];
        }

        let ts_node = node.inner();

        // Get operator
        let Some(operator) = ts_node.child_by_field_name("operator") else {
            return vec![];
        };
        let op_text = &ctx.source()[operator.start_byte()..operator.end_byte()];

        // Only check == and !=
        if op_text != "==" && op_text != "!=" {
            return vec![];
        }

        // Get left and right operands
        let Some(left) = ts_node.child_by_field_name("left") else {
            return vec![];
        };
        let Some(right) = ts_node.child_by_field_name("right") else {
            return vec![];
        };

        // Check if either operand is a string expression (literal or concatenation of strings)
        let left_is_string_expr = Self::is_string_expression(&left, ctx.source());
        let right_is_string_expr = Self::is_string_expression(&right, ctx.source());

        // At least one operand must be a string expression
        if !left_is_string_expr && !right_is_string_expr {
            return vec![];
        }

        // Use operator position for diagnostic (matches checkstyle behavior for multiline expressions)
        let operator_range = CstNode::new(operator, ctx.source()).range();
        // Use full expression range for the fix
        let expr_range = node.range();
        let source = ctx.source();

        let left_text = &source[left.start_byte()..left.end_byte()];
        let right_text = &source[right.start_byte()..right.end_byte()];

        // Create fix only for simple cases (direct string literals)
        let left_is_simple_string = left.kind() == "string_literal";
        let right_is_simple_string = right.kind() == "string_literal";

        let fix = if left_is_simple_string || right_is_simple_string {
            self.create_fix(
                left_text,
                right_text,
                left_is_simple_string,
                op_text == "!=",
            )
        } else {
            None // No fix for complex cases like concatenation
        };

        // Report at operator position, but fix the whole expression
        let diagnostic = Diagnostic::new(StringLiteralEqualityViolation, operator_range);

        match fix {
            Some(replacement) => {
                vec![diagnostic.with_fix(Fix::unsafe_edit(Edit::replacement(
                    replacement,
                    expr_range.start(),
                    expr_range.end(),
                )))]
            }
            None => vec![diagnostic],
        }
    }
}

impl StringLiteralEquality {
    /// Check if an expression is a "string expression" for the purposes of this check.
    ///
    /// A string expression is:
    /// - A string literal
    /// - A binary expression with + operator where at least one side is a string expression
    /// - A parenthesized expression containing a string expression
    ///
    /// NOT a string expression:
    /// - Method invocations (even on string literals)
    /// - Assignment expressions
    /// - Other expression types
    fn is_string_expression(node: &Node, source: &str) -> bool {
        match node.kind() {
            "string_literal" => true,
            "binary_expression" => {
                // Check if this is a string concatenation (+ operator)
                if let Some(operator) = node.child_by_field_name("operator") {
                    let op_text = &source[operator.start_byte()..operator.end_byte()];
                    if op_text == "+" {
                        // Check if either operand is a string expression
                        let left = node.child_by_field_name("left");
                        let right = node.child_by_field_name("right");

                        let left_is_string = left
                            .map(|n| Self::is_string_expression(&n, source))
                            .unwrap_or(false);
                        let right_is_string = right
                            .map(|n| Self::is_string_expression(&n, source))
                            .unwrap_or(false);

                        return left_is_string || right_is_string;
                    }
                }
                false
            }
            "parenthesized_expression" => {
                // Check the inner expression
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() != "(" && child.kind() != ")" {
                        return Self::is_string_expression(&child, source);
                    }
                }
                false
            }
            _ => false,
        }
    }

    /// Create a fix for the string comparison.
    ///
    /// For `str == "literal"` -> `"literal".equals(str)` (null-safe)
    /// For `"literal" == str` -> `"literal".equals(str)`
    /// For `str != "literal"` -> `!"literal".equals(str)`
    fn create_fix(
        &self,
        left_text: &str,
        right_text: &str,
        left_is_string: bool,
        is_not_equals: bool,
    ) -> Option<String> {
        let (literal, other) = if left_is_string {
            (left_text, right_text)
        } else {
            (right_text, left_text)
        };

        let equals_call = format!("{}.equals({})", literal, other);

        if is_not_equals {
            Some(format!("!{}", equals_call))
        } else {
            Some(equals_call)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str) -> Vec<(usize, String)> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = StringLiteralEquality;
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                diagnostics.push((loc.line.get(), d.kind.body.clone()));
            }
        }
        diagnostics
    }

    #[test]
    fn test_string_equals_literal() {
        let source = r#"
class Test {
    void method(String s) {
        if (s == "foo") {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].0, 4);
    }

    #[test]
    fn test_literal_equals_string() {
        let source = r#"
class Test {
    void method(String s) {
        if ("foo" == s) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].0, 4);
    }

    #[test]
    fn test_string_not_equals_literal() {
        let source = r#"
class Test {
    void method(String s) {
        if (s != "foo") {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].0, 4);
    }

    #[test]
    fn test_no_violation_without_string_literal() {
        let source = r#"
class Test {
    void method(String a, String b) {
        if (a == b) {}
        if (a != b) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Comparing two variables should not trigger violation"
        );
    }

    #[test]
    fn test_equals_method_no_violation() {
        let source = r#"
class Test {
    void method(String s) {
        if (s.equals("foo")) {}
        if ("foo".equals(s)) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Using equals() should not trigger violation"
        );
    }

    #[test]
    fn test_fix_generated() {
        let source = r#"
class Test {
    void method(String s) {
        if (s == "foo") {}
    }
}
"#;
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = StringLiteralEquality;

        let mut has_fix = false;
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                if d.fix.is_some() {
                    has_fix = true;
                }
            }
        }
        assert!(has_fix, "Should generate a fix for string literal equality");
    }

    #[test]
    fn test_concatenated_string_literal() {
        let source = r#"
class Test {
    void method(String s) {
        if (s == "a" + "bc") {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Should detect string literals in concatenation"
        );
    }

    #[test]
    fn test_text_block() {
        let source = r#"
class Test {
    void method(String s) {
        if (s == """
                foo""") {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1, "Should detect text blocks");
    }

    #[test]
    fn test_method_call_on_literal_no_violation() {
        let source = r#"
class Test {
    void method() {
        if ("foo".toUpperCase() == "bar".toLowerCase()) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Method calls on string literals should not trigger violation"
        );
    }

    #[test]
    fn test_assignment_expression_no_violation() {
        let source = r#"
class Test {
    void method(String s, String p) {
        if ((s += "asd") != p) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Assignment expressions should not trigger violation"
        );
    }
}
