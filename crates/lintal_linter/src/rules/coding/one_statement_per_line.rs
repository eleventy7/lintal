//! OneStatementPerLine rule implementation.
//!
//! Checks that there is only one statement per line.
//!
//! Checkstyle equivalent: OneStatementPerLineCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: multiple statements on same line.
#[derive(Debug, Clone)]
pub struct OneStatementPerLineViolation;

impl Violation for OneStatementPerLineViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Only one statement per line allowed.".to_string()
    }
}

/// Configuration for OneStatementPerLine rule.
#[derive(Debug, Clone, Default)]
pub struct OneStatementPerLine {
    /// Whether try-with-resources resources count as statements.
    pub treat_try_resources_as_statement: bool,
}

const RELEVANT_KINDS: &[&str] = &["block", "class_body"];

impl FromConfig for OneStatementPerLine {
    const MODULE_NAME: &'static str = "OneStatementPerLine";

    fn from_config(properties: &Properties) -> Self {
        let treat_try_resources_as_statement = properties
            .get("treatTryResourcesAsStatement")
            .map(|v| *v == "true")
            .unwrap_or(false);

        Self {
            treat_try_resources_as_statement,
        }
    }
}

impl Rule for OneStatementPerLine {
    fn name(&self) -> &'static str {
        "OneStatementPerLine"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Check both blocks (method bodies) and class_body (field declarations)
        if node.kind() != "block" && node.kind() != "class_body" {
            return vec![];
        }

        let source = ctx.source();
        let source_code = ctx.source_code();

        let mut diagnostics = vec![];
        let mut prev_statement_line: Option<usize> = None;

        // Iterate through children of the block
        let ts_node = node.inner();
        let mut cursor = ts_node.walk();

        for child in ts_node.children(&mut cursor) {
            // Skip non-statement nodes (braces, comments)
            if !Self::is_statement_node(child.kind()) {
                continue;
            }

            let start_pos = lintal_text_size::TextSize::from(child.start_byte() as u32);
            let current_line = source_code.line_column(start_pos).line.get();

            if let Some(prev_line) = prev_statement_line
                && current_line == prev_line
            {
                // Two statements on same line - violation
                let range = lintal_text_size::TextRange::new(
                    start_pos,
                    lintal_text_size::TextSize::from(child.end_byte() as u32),
                );

                // Calculate fix: insert newline + indentation before this statement
                let indent = Self::get_indentation(source, child.start_byte());
                let fix_start = Self::find_prev_semicolon_end(source, child.start_byte());
                let fix_range = lintal_text_size::TextRange::new(
                    lintal_text_size::TextSize::from(fix_start as u32),
                    start_pos,
                );

                diagnostics.push(
                    Diagnostic::new(OneStatementPerLineViolation, range).with_fix(Fix::safe_edit(
                        Edit::range_replacement(format!("\n{}", indent), fix_range),
                    )),
                );
            }

            prev_statement_line = Some(current_line);
        }

        diagnostics
    }
}

impl OneStatementPerLine {
    /// Check if a node kind represents a statement.
    fn is_statement_node(kind: &str) -> bool {
        matches!(
            kind,
            "local_variable_declaration"
                | "field_declaration"
                | "expression_statement"
                | "if_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "while_statement"
                | "do_statement"
                | "try_statement"
                | "switch_expression"
                | "return_statement"
                | "throw_statement"
                | "break_statement"
                | "continue_statement"
                | "assert_statement"
                | "synchronized_statement"
                | "labeled_statement"
                | "empty_statement"
        )
    }

    /// Get the indentation at a byte position.
    fn get_indentation(source: &str, pos: usize) -> String {
        // Find the start of the line
        let line_start = source[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &source[line_start..pos];

        // Extract leading whitespace
        let indent_len = line.len() - line.trim_start().len();
        line[..indent_len].to_string()
    }

    /// Find the end of the previous semicolon (after any whitespace).
    fn find_prev_semicolon_end(source: &str, pos: usize) -> usize {
        // Look backwards for semicolon
        let before = &source[..pos];
        if let Some(semi_pos) = before.rfind(';') {
            // Return position after semicolon
            semi_pos + 1
        } else {
            pos
        }
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
        let rule = OneStatementPerLine::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_two_statements_same_line() {
        let source = r#"
class Test {
    void method() {
        int a; int b;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for two statements on same line"
        );
    }

    #[test]
    fn test_single_statement_per_line_ok() {
        let source = r#"
class Test {
    void method() {
        int a;
        int b;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Single statements per line should not cause violations"
        );
    }

    #[test]
    fn test_for_loop_header_ok() {
        let source = r#"
class Test {
    void method() {
        for (int i = 0; i < 10; i++) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "For loop header should not cause violations"
        );
    }

    #[test]
    fn test_class_level_fields() {
        let source = r#"
class Test {
    int a; int b;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for class-level fields on same line"
        );
    }
}
