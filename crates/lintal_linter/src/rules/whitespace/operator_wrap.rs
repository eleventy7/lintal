//! OperatorWrap rule implementation.
//!
//! Checks that operators are on the correct line when expressions span multiple lines.
//!
//! Checkstyle equivalent: OperatorWrapCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: operator should be on a new line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnNewLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnNewLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on a new line.", self.operator)
    }
}

/// Violation: operator should be on the previous line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnPrevLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnPrevLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on the previous line.", self.operator)
    }
}

/// Option for where operators should be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapOption {
    /// Operator should be on a new line (default).
    #[default]
    Nl,
    /// Operator should be at end of line.
    Eol,
}

/// Configuration for OperatorWrap rule.
#[derive(Debug, Clone)]
pub struct OperatorWrap {
    pub option: WrapOption,
}

const RELEVANT_KINDS: &[&str] = &["binary_expression"];

impl Default for OperatorWrap {
    fn default() -> Self {
        Self {
            option: WrapOption::Nl,
        }
    }
}

impl FromConfig for OperatorWrap {
    const MODULE_NAME: &'static str = "OperatorWrap";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match *v {
                "eol" | "EOL" => WrapOption::Eol,
                _ => WrapOption::Nl,
            })
            .unwrap_or_default();

        Self { option }
    }
}

impl Rule for OperatorWrap {
    fn name(&self) -> &'static str {
        "OperatorWrap"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Check binary expressions
        if node.kind() != "binary_expression" {
            return vec![];
        }

        let ts_node = node.inner();
        let source_code = ctx.source_code();

        // Get the operator (middle child)
        let mut cursor = ts_node.walk();
        let all_children: Vec<_> = ts_node.children(&mut cursor).collect();

        // Filter out comments and whitespace (extra nodes)
        let children: Vec<_> = all_children.iter().filter(|n| !n.is_extra()).collect();

        if children.len() < 3 {
            return vec![];
        }

        let left = children[0];
        let operator = children[1];
        let right = children[2];

        // Check if expression spans multiple lines
        let left_end = lintal_text_size::TextSize::from(left.end_byte() as u32);

        // Skip comments to find the actual start of the right operand
        let actual_right_start = {
            let mut right_cursor = right.walk();
            let first_non_extra = right
                .children(&mut right_cursor)
                .find(|child| !child.is_extra());

            if let Some(child) = first_non_extra {
                child.start_byte()
            } else {
                right.start_byte()
            }
        };

        let right_start = lintal_text_size::TextSize::from(actual_right_start as u32);
        let op_start = lintal_text_size::TextSize::from(operator.start_byte() as u32);

        let left_end_line = source_code.line_column(left_end).line.get();
        let right_start_line = source_code.line_column(right_start).line.get();
        let op_line = source_code.line_column(op_start).line.get();

        // Only check if expression spans multiple lines
        if left_end_line == right_start_line {
            return vec![];
        }

        let op_text = operator.utf8_text(ctx.source().as_bytes()).unwrap_or("");
        let op_range = lintal_text_size::TextRange::new(
            op_start,
            lintal_text_size::TextSize::from(operator.end_byte() as u32),
        );

        match self.option {
            WrapOption::Nl => {
                // Operator should be on new line (same line as right operand)
                if op_line == left_end_line && op_line != right_start_line {
                    return vec![Diagnostic::new(
                        OperatorShouldBeOnNewLine {
                            operator: op_text.to_string(),
                        },
                        op_range,
                    )];
                }
            }
            WrapOption::Eol => {
                // Operator should be at end of line (same line as left operand)
                if op_line == right_start_line && op_line != left_end_line {
                    return vec![Diagnostic::new(
                        OperatorShouldBeOnPrevLine {
                            operator: op_text.to_string(),
                        },
                        op_range,
                    )];
                }
            }
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source_nl(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap::default(); // nl option

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_eol(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap {
            option: WrapOption::Eol,
        };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_nl_operator_at_end_of_line_violation() {
        // With nl option, operator at end of line is a violation
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Operator at end of line should be violation with nl option"
        );
    }

    #[test]
    fn test_nl_operator_on_new_line_ok() {
        // With nl option, operator on new line is OK
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics.is_empty(),
            "Operator on new line should be OK with nl option"
        );
    }

    #[test]
    fn test_eol_operator_on_new_line_violation() {
        // With eol option, operator on new line is a violation
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Operator on new line should be violation with eol option"
        );
    }

    #[test]
    fn test_eol_operator_at_end_of_line_ok() {
        // With eol option, operator at end of line is OK
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert!(
            diagnostics.is_empty(),
            "Operator at end of line should be OK with eol option"
        );
    }

    #[test]
    fn test_same_line_no_violation() {
        // No violation if everything is on same line
        let source = r#"
class Test {
    void method() {
        int x = 1 + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics.is_empty(),
            "Same line expression should not cause violation"
        );
    }
}
