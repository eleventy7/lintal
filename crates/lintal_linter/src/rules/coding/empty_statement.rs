//! EmptyStatement rule implementation.
//!
//! Detects empty statements (lone semicolons) that are usually a mistake.
//!
//! Checkstyle equivalent: EmptyStatementCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: empty statement detected.
#[derive(Debug, Clone)]
pub struct EmptyStatementViolation;

impl Violation for EmptyStatementViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "Empty statement.".to_string()
    }
}

/// Configuration for EmptyStatement rule.
#[derive(Debug, Clone, Default)]
pub struct EmptyStatement;

const RELEVANT_KINDS: &[&str] = &[
    "if_statement",
    "while_statement",
    "for_statement",
    "enhanced_for_statement",
    "do_statement",
    ";",
];

impl FromConfig for EmptyStatement {
    const MODULE_NAME: &'static str = "EmptyStatement";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for EmptyStatement {
    fn name(&self) -> &'static str {
        "EmptyStatement"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "if_statement" => self.check_if_statement(ctx, node),
            "while_statement" => self.check_while_statement(ctx, node),
            "for_statement" => self.check_for_statement(ctx, node),
            "enhanced_for_statement" => self.check_enhanced_for_statement(ctx, node),
            "do_statement" => self.check_do_statement(ctx, node),
            ";" => self.check_standalone_semicolon(ctx, node),
            _ => vec![],
        }
    }
}

impl EmptyStatement {
    /// Check if statement for empty body.
    fn check_if_statement(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // In tree-sitter Java, if_statement has:
        // - "condition" field: the condition
        // - "consequence" field: the then body
        // - "alternative" field (optional): the else body
        let mut diagnostics = vec![];

        // Check the consequence (then body)
        if let Some(consequence) = node.child_by_field_name("consequence")
            && consequence.kind() == ";"
        {
            diagnostics.push(self.create_diagnostic(&consequence));
        }

        // Check the alternative (else body) if present
        if let Some(alternative) = node.child_by_field_name("alternative")
            && alternative.kind() == ";"
        {
            diagnostics.push(self.create_diagnostic(&alternative));
        }

        diagnostics
    }

    /// Check while statement for empty body.
    fn check_while_statement(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // while_statement has "body" field
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == ";"
        {
            return vec![self.create_diagnostic(&body)];
        }
        vec![]
    }

    /// Check for statement for empty body.
    fn check_for_statement(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // for_statement has "body" field
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == ";"
        {
            return vec![self.create_diagnostic(&body)];
        }
        vec![]
    }

    /// Check enhanced for statement for empty body.
    fn check_enhanced_for_statement(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // enhanced_for_statement has "body" field
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == ";"
        {
            return vec![self.create_diagnostic(&body)];
        }
        vec![]
    }

    /// Check do statement for empty body.
    fn check_do_statement(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // do_statement has "body" field
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == ";"
        {
            return vec![self.create_diagnostic(&body)];
        }
        vec![]
    }

    /// Check for standalone semicolons in blocks.
    fn check_standalone_semicolon(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only flag semicolons that are direct children of blocks (statement-level)
        // Skip semicolons in:
        // - for loops (init/update)
        // - switch statements (after labels)
        // - empty method/class bodies

        let Some(parent) = node.parent() else {
            return vec![];
        };

        match parent.kind() {
            // Empty statements in blocks are violations
            "block" | "constructor_body" => vec![self.create_diagnostic(node)],
            // Also check program-level (top-level) empty statements
            "program" => vec![self.create_diagnostic(node)],
            // Switch block body can have empty statements
            "switch_block_statement_group" => vec![self.create_diagnostic(node)],
            _ => vec![],
        }
    }

    /// Create a diagnostic for an empty statement.
    fn create_diagnostic(&self, node: &CstNode) -> Diagnostic {
        let range = node.range();

        // Create a fix that removes the empty semicolon
        // This is an unsafe fix because removing it might change behavior
        // (e.g., if (cond); doSomething() -> if (cond) doSomething())
        Diagnostic::new(EmptyStatementViolation, range)
            .with_fix(Fix::unsafe_edit(Edit::deletion(range.start(), range.end())))
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
        let rule = EmptyStatement;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_if_with_empty_body() {
        let source = r#"
class Test {
    void method() {
        if (true);
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1, "Should detect empty if body");
    }

    #[test]
    fn test_if_else_with_empty_bodies() {
        let source = r#"
class Test {
    void method() {
        if (true);
        else;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            2,
            "Should detect empty if and else bodies"
        );
    }

    #[test]
    fn test_while_with_empty_body() {
        let source = r#"
class Test {
    void method() {
        while (condition);
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1, "Should detect empty while body");
    }

    #[test]
    fn test_for_with_empty_body() {
        let source = r#"
class Test {
    void method() {
        for (int i = 0; i < 10; i++);
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1, "Should detect empty for body");
    }

    #[test]
    fn test_enhanced_for_with_empty_body() {
        let source = r#"
class Test {
    void method(int[] arr) {
        for (int x : arr);
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Should detect empty enhanced for body"
        );
    }

    #[test]
    fn test_do_with_empty_body() {
        let source = r#"
class Test {
    void method() {
        do; while (condition);
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1, "Should detect empty do body");
    }

    #[test]
    fn test_standalone_semicolon_in_block() {
        let source = r#"
class Test {
    void method() {
        ;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Should detect standalone semicolon in block"
        );
    }

    #[test]
    fn test_normal_statements_no_violation() {
        let source = r#"
class Test {
    void method() {
        if (true) {
            doSomething();
        }
        while (condition) {
            work();
        }
        for (int i = 0; i < 10; i++) {
            process(i);
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Normal statements should not be violations"
        );
    }

    #[test]
    fn test_for_loop_semicolons_no_violation() {
        // The semicolons inside for loop syntax are not violations
        let source = r#"
class Test {
    void method() {
        for (;;) {
            break;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "For loop syntax semicolons should not be violations"
        );
    }
}
