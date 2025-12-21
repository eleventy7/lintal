//! OperatorWrap rule implementation.
//!
//! Checks that operators are on the correct line when expressions span multiple lines.
//!
//! Checkstyle equivalent: OperatorWrapCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
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

    fn check(&self, _ctx: &CheckContext, _node: &CstNode) -> Vec<Diagnostic> {
        // TODO: Implement
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
        let rule = OperatorWrap { option: WrapOption::Eol };

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
        assert_eq!(diagnostics.len(), 1, "Operator at end of line should be violation with nl option");
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
        assert!(diagnostics.is_empty(), "Operator on new line should be OK with nl option");
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
        assert_eq!(diagnostics.len(), 1, "Operator on new line should be violation with eol option");
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
        assert!(diagnostics.is_empty(), "Operator at end of line should be OK with eol option");
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
        assert!(diagnostics.is_empty(), "Same line expression should not cause violation");
    }
}
