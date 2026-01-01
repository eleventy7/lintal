//! MultipleVariableDeclarations rule implementation.
//!
//! Checks that each variable is declared in its own statement and on its own line.
//!
//! Checkstyle equivalent: MultipleVariableDeclarationsCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: comma-separated variables in single declaration.
#[derive(Debug, Clone)]
pub struct MultipleInStatementViolation;

impl Violation for MultipleInStatementViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "Each variable declaration must be in its own statement.".to_string()
    }
}

/// Violation: multiple declarations on same line.
#[derive(Debug, Clone)]
pub struct MultipleOnLineViolation;

impl Violation for MultipleOnLineViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Only one variable definition per line allowed.".to_string()
    }
}

/// Configuration for MultipleVariableDeclarations rule.
#[derive(Debug, Clone, Default)]
pub struct MultipleVariableDeclarations;

const RELEVANT_KINDS: &[&str] = &[
    "local_variable_declaration",
    "field_declaration",
    "block",
    "class_body",
];

impl FromConfig for MultipleVariableDeclarations {
    const MODULE_NAME: &'static str = "MultipleVariableDeclarations";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for MultipleVariableDeclarations {
    fn name(&self) -> &'static str {
        "MultipleVariableDeclarations"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check for comma-separated variables in a single declaration
        if node.kind() == "local_variable_declaration" || node.kind() == "field_declaration" {
            // Skip if inside for-loop initializer
            if Self::is_in_for_init(node) {
                return vec![];
            }

            diagnostics.extend(self.check_comma_separated(ctx, node));
        }

        // Check for multiple declarations on same line (handled at block level)
        if node.kind() == "block" || node.kind() == "class_body" {
            diagnostics.extend(self.check_same_line_declarations(ctx, node));
        }

        diagnostics
    }
}

impl MultipleVariableDeclarations {
    /// Check if node is inside a for-loop initializer.
    fn is_in_for_init(node: &CstNode) -> bool {
        let mut current = node.inner().parent();
        while let Some(parent) = current {
            if parent.kind() == "for_statement" {
                // Check if we're in the init part (first child after 'for' and '(')
                let mut cursor = parent.walk();
                for child in parent.children(&mut cursor) {
                    if child.kind() == "local_variable_declaration" {
                        return child.id() == node.inner().id();
                    }
                    if child.kind() == ";" {
                        break;
                    }
                }
            }
            current = parent.parent();
        }
        false
    }

    /// Check for comma-separated variables in a declaration.
    fn check_comma_separated(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let mut cursor = ts_node.walk();

        let mut declarator_count = 0;
        let mut first_declarator_range = None;

        for child in ts_node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                declarator_count += 1;
                if first_declarator_range.is_none() {
                    first_declarator_range = Some(lintal_text_size::TextRange::new(
                        lintal_text_size::TextSize::from(child.start_byte() as u32),
                        lintal_text_size::TextSize::from(child.end_byte() as u32),
                    ));
                }
            }
        }

        if declarator_count > 1
            && let Some(range) = first_declarator_range
        {
            // TODO: Create fix that splits declarations
            return vec![Diagnostic::new(MultipleInStatementViolation, range)];
        }

        vec![]
    }

    /// Check for multiple declarations on the same line.
    fn check_same_line_declarations(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let source_code = ctx.source_code();
        let ts_node = node.inner();
        let mut cursor = ts_node.walk();

        let mut diagnostics = vec![];
        let mut prev_decl_line: Option<usize> = None;

        for child in ts_node.children(&mut cursor) {
            let kind = child.kind();
            if kind != "local_variable_declaration" && kind != "field_declaration" {
                continue;
            }

            let start_pos = lintal_text_size::TextSize::from(child.start_byte() as u32);
            let current_line = source_code.line_column(start_pos).line.get();

            if let Some(prev_line) = prev_decl_line
                && current_line == prev_line
            {
                let range = lintal_text_size::TextRange::new(
                    start_pos,
                    lintal_text_size::TextSize::from(child.end_byte() as u32),
                );

                // Create fix: insert newline before this declaration
                let source = ctx.source();
                let indent = Self::get_indentation(source, child.start_byte());
                let fix_start = Self::find_prev_semicolon_end(source, child.start_byte());
                let fix_range = lintal_text_size::TextRange::new(
                    lintal_text_size::TextSize::from(fix_start as u32),
                    start_pos,
                );

                diagnostics.push(Diagnostic::new(MultipleOnLineViolation, range).with_fix(
                    Fix::safe_edit(Edit::range_replacement(format!("\n{}", indent), fix_range)),
                ));
            }

            prev_decl_line = Some(current_line);
        }

        diagnostics
    }

    fn get_indentation(source: &str, pos: usize) -> String {
        let line_start = source[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let line = &source[line_start..pos];
        let indent_len = line.len() - line.trim_start().len();
        line[..indent_len].to_string()
    }

    fn find_prev_semicolon_end(source: &str, pos: usize) -> usize {
        let before = &source[..pos];
        if let Some(semi_pos) = before.rfind(';') {
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
        let rule = MultipleVariableDeclarations;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_comma_separated_violation() {
        let source = r#"
class Test {
    int i, j;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for comma-separated variables"
        );
        assert!(diagnostics[0].kind.body.contains("own statement"));
    }

    #[test]
    fn test_same_line_violation() {
        let source = r#"
class Test {
    int i; int j;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for same-line declarations"
        );
        assert!(diagnostics[0].kind.body.contains("per line"));
    }

    #[test]
    fn test_separate_lines_ok() {
        let source = r#"
class Test {
    int i;
    int j;
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Separate lines should not cause violations"
        );
    }

    #[test]
    fn test_for_loop_ok() {
        let source = r#"
class Test {
    void method() {
        for (int i = 0, j = 0; i < 10; i++, j++) {}
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "For loop initializers should not cause violations"
        );
    }
}
