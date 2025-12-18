//! WhitespaceAround rule implementation.
//!
//! Checks that operators are surrounded by whitespace.

use jruff_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use jruff_java_cst::CstNode;
use jruff_text_size::TextSize;

use crate::{CheckContext, Rule};

/// Configuration for WhitespaceAround rule.
#[derive(Debug, Clone)]
pub struct WhitespaceAround {
    pub allow_empty_lambdas: bool,
    pub allow_empty_methods: bool,
    pub allow_empty_constructors: bool,
}

impl Default for WhitespaceAround {
    fn default() -> Self {
        Self {
            allow_empty_lambdas: false,
            allow_empty_methods: false,
            allow_empty_constructors: false,
        }
    }
}

/// Violation for missing whitespace before a token.
#[derive(Debug, Clone)]
pub struct MissingWhitespaceBefore {
    pub token: String,
}

impl Violation for MissingWhitespaceBefore {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Missing whitespace before `{}`", self.token)
    }
}

/// Violation for missing whitespace after a token.
#[derive(Debug, Clone)]
pub struct MissingWhitespaceAfter {
    pub token: String,
}

impl Violation for MissingWhitespaceAfter {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Missing whitespace after `{}`", self.token)
    }
}

impl Rule for WhitespaceAround {
    fn name(&self) -> &'static str {
        "WhitespaceAround"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check binary expressions for operator whitespace
        if node.kind() == "binary_expression" {
            if let Some(op) = find_operator(node) {
                let op_range = op.range();
                let op_text = op.text();

                // Check whitespace before operator
                let before_pos = op_range.start();
                if before_pos > TextSize::new(0) {
                    let char_before = ctx
                        .source()
                        .get(usize::from(before_pos) - 1..usize::from(before_pos))
                        .unwrap_or("");
                    if !char_before.chars().next().map_or(false, |c| c.is_whitespace()) {
                        diagnostics.push(
                            Diagnostic::new(
                                MissingWhitespaceBefore {
                                    token: op_text.to_string(),
                                },
                                op_range,
                            )
                            .with_fix(Fix::safe_edit(Edit::insertion(
                                " ".to_string(),
                                before_pos,
                            ))),
                        );
                    }
                }

                // Check whitespace after operator
                let after_pos = op_range.end();
                let char_after = ctx
                    .source()
                    .get(usize::from(after_pos)..usize::from(after_pos) + 1)
                    .unwrap_or("");
                if !char_after.chars().next().map_or(false, |c| c.is_whitespace()) {
                    diagnostics.push(
                        Diagnostic::new(
                            MissingWhitespaceAfter {
                                token: op_text.to_string(),
                            },
                            op_range,
                        )
                        .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), after_pos))),
                    );
                }
            }
        }

        diagnostics
    }
}

/// Find the operator node in a binary expression.
fn find_operator<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
    // In tree-sitter-java, binary_expression has structure:
    // (binary_expression left: ... operator right: ...)
    // The operator is typically the second child
    for child in node.children() {
        let kind = child.kind();
        // Check for operator tokens
        if matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "=="
                | "!="
                | "<"
                | ">"
                | "<="
                | ">="
                | "&&"
                | "||"
                | "&"
                | "|"
                | "^"
                | "<<"
                | ">>"
                | ">>>"
        ) {
            return Some(child);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use jruff_java_cst::TreeWalker;
    use jruff_java_parser::JavaParser;

    #[test]
    fn test_whitespace_around_detects_missing() {
        let mut parser = JavaParser::new();
        let source = "class Foo { int x = 1+2; }";
        let result = parser.parse(source).unwrap();

        let rule = WhitespaceAround::default();
        let ctx = CheckContext::new(source);

        let mut all_diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            all_diagnostics.extend(rule.check(&ctx, &node));
        }

        // Should detect missing whitespace around +
        assert!(!all_diagnostics.is_empty());
    }
}
