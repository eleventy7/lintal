//! SimplifyBooleanExpression rule implementation.
//!
//! Checks for boolean expressions that can be simplified.
//!
//! Checkstyle equivalent: SimplifyBooleanExpressionCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: boolean expression can be simplified.
#[derive(Debug, Clone)]
pub struct SimplifyBooleanExpressionViolation {
    pub suggestion: String,
}

impl Violation for SimplifyBooleanExpressionViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!(
            "Expression can be simplified, such as use '{}'.",
            self.suggestion
        )
    }
}

/// Violation: expression is always true or false.
#[derive(Debug, Clone)]
pub struct AlwaysTrueOrFalseViolation {
    pub value: bool,
}

impl Violation for AlwaysTrueOrFalseViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Expression is always {}.",
            if self.value { "true" } else { "false" }
        )
    }
}

/// Configuration for SimplifyBooleanExpression rule.
#[derive(Debug, Clone, Default)]
pub struct SimplifyBooleanExpression;

const RELEVANT_KINDS: &[&str] = &[
    "binary_expression",
    "unary_expression",
    "ternary_expression",
];

impl FromConfig for SimplifyBooleanExpression {
    const MODULE_NAME: &'static str = "SimplifyBooleanExpression";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for SimplifyBooleanExpression {
    fn name(&self) -> &'static str {
        "SimplifyBooleanExpression"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "binary_expression" => self.check_binary_expression(ctx, node),
            "unary_expression" => self.check_unary_expression(ctx, node),
            "ternary_expression" => self.check_ternary_expression(ctx, node),
            _ => vec![],
        }
    }
}

impl SimplifyBooleanExpression {
    /// Check binary expressions like `a == true`, `a != false`, etc.
    fn check_binary_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();

        // Get operator
        let Some(operator) = ts_node.child_by_field_name("operator") else {
            return vec![];
        };
        let op_text = &ctx.source()[operator.start_byte()..operator.end_byte()];

        // Get left and right operands
        let Some(left) = ts_node.child_by_field_name("left") else {
            return vec![];
        };
        let Some(right) = ts_node.child_by_field_name("right") else {
            return vec![];
        };

        let left_text = &ctx.source()[left.start_byte()..left.end_byte()];
        let right_text = &ctx.source()[right.start_byte()..right.end_byte()];

        let left_is_true = left.kind() == "true";
        let left_is_false = left.kind() == "false";
        let right_is_true = right.kind() == "true";
        let right_is_false = right.kind() == "false";

        let range = node.range();

        match op_text {
            "==" => {
                // a == true -> a
                // true == a -> a
                if right_is_true {
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: left_text.to_string(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            left_text.to_string(),
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                if left_is_true {
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: right_text.to_string(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            right_text.to_string(),
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                // a == false -> !a
                // false == a -> !a
                if right_is_false {
                    let suggestion = format!("!{}", left_text);
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: suggestion.clone(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            suggestion,
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                if left_is_false {
                    let suggestion = format!("!{}", right_text);
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: suggestion.clone(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            suggestion,
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
            }
            "!=" => {
                // a != true -> !a
                // true != a -> !a
                if right_is_true {
                    let suggestion = format!("!{}", left_text);
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: suggestion.clone(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            suggestion,
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                if left_is_true {
                    let suggestion = format!("!{}", right_text);
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: suggestion.clone(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            suggestion,
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                // a != false -> a
                // false != a -> a
                if right_is_false {
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: left_text.to_string(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            left_text.to_string(),
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
                if left_is_false {
                    return vec![
                        Diagnostic::new(
                            SimplifyBooleanExpressionViolation {
                                suggestion: right_text.to_string(),
                            },
                            range,
                        )
                        .with_fix(Fix::unsafe_edit(Edit::replacement(
                            right_text.to_string(),
                            range.start(),
                            range.end(),
                        ))),
                    ];
                }
            }
            "||" => {
                // a || true -> always true (warn only, no fix)
                if right_is_true || left_is_true {
                    return vec![Diagnostic::new(
                        AlwaysTrueOrFalseViolation { value: true },
                        range,
                    )];
                }
            }
            "&&" => {
                // a && false -> always false (warn only, no fix)
                if right_is_false || left_is_false {
                    return vec![Diagnostic::new(
                        AlwaysTrueOrFalseViolation { value: false },
                        range,
                    )];
                }
            }
            _ => {}
        }

        vec![]
    }

    /// Check unary expressions like `!true`, `!false`.
    fn check_unary_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();

        // Get operator (first child)
        let Some(operator) = ts_node.child_by_field_name("operator") else {
            return vec![];
        };
        let op_text = &ctx.source()[operator.start_byte()..operator.end_byte()];

        if op_text != "!" {
            return vec![];
        }

        // Get operand
        let Some(operand) = ts_node.child_by_field_name("operand") else {
            return vec![];
        };

        let range = node.range();

        // !true -> false
        if operand.kind() == "true" {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: "false".to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    "false".to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // !false -> true
        if operand.kind() == "false" {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: "true".to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    "true".to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        vec![]
    }

    /// Check ternary expressions like `cond ? true : false`, `true ? a : b`, etc.
    fn check_ternary_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();

        // Collect the three expression parts (skip ? and : tokens)
        let mut cursor = ts_node.walk();
        let parts: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|c| c.kind() != "?" && c.kind() != ":")
            .collect();

        if parts.len() != 3 {
            return vec![];
        }

        let condition = &parts[0];
        let consequence = &parts[1];
        let alternative = &parts[2];

        let condition_text = &ctx.source()[condition.start_byte()..condition.end_byte()];
        let consequence_text = &ctx.source()[consequence.start_byte()..consequence.end_byte()];
        let alternative_text = &ctx.source()[alternative.start_byte()..alternative.end_byte()];

        let condition_is_true = condition.kind() == "true";
        let condition_is_false = condition.kind() == "false";
        let consequence_is_true = consequence.kind() == "true";
        let consequence_is_false = consequence.kind() == "false";
        let alternative_is_true = alternative.kind() == "true";
        let alternative_is_false = alternative.kind() == "false";

        let range = node.range();

        // true ? a : b -> a
        if condition_is_true {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: consequence_text.to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    consequence_text.to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // false ? a : b -> b
        if condition_is_false {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: alternative_text.to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    alternative_text.to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // cond ? true : false -> cond
        if consequence_is_true && alternative_is_false {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: condition_text.to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    condition_text.to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // cond ? false : true -> !cond
        if consequence_is_false && alternative_is_true {
            let suggestion = format!("!{}", condition_text);
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: suggestion.clone(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    suggestion,
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // cond ? true : true -> true (same value both branches)
        if consequence_is_true && alternative_is_true {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: "true".to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    "true".to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        // cond ? false : false -> false (same value both branches)
        if consequence_is_false && alternative_is_false {
            return vec![
                Diagnostic::new(
                    SimplifyBooleanExpressionViolation {
                        suggestion: "false".to_string(),
                    },
                    range,
                )
                .with_fix(Fix::unsafe_edit(Edit::replacement(
                    "false".to_string(),
                    range.start(),
                    range.end(),
                ))),
            ];
        }

        vec![]
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
        let rule = SimplifyBooleanExpression;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_equals_true() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b == true) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_equals_false() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b == false) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_not_equals_true() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b != true) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_not_equals_false() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b != false) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_not_true() {
        let source = r#"
class Test {
    void method() {
        boolean x = !true;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_not_false() {
        let source = r#"
class Test {
    void method() {
        boolean x = !false;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_or_true() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b || true) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_and_false() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (b && false) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_normal_expressions_no_violation() {
        let source = r#"
class Test {
    void method(boolean a, boolean b) {
        if (a == b) {}
        if (a != b) {}
        if (a && b) {}
        if (a || b) {}
        boolean x = !a;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Normal expressions should not be violations"
        );
    }

    #[test]
    fn test_true_equals_a() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (true == b) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_false_equals_a() {
        let source = r#"
class Test {
    void method(boolean b) {
        if (false == b) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }
}
