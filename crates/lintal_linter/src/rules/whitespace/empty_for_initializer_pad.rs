//! EmptyForInitializerPad rule implementation.
//!
//! Checks the padding of an empty for initializer; that is whether a white
//! space is required at an empty for initializer, or such white space is
//! forbidden. No check occurs if there is a line wrap at the initializer.
//! Checkstyle equivalent: EmptyForInitializerPad

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;

use crate::rules::whitespace::common::{
    diag_not_preceded, diag_preceded, has_whitespace_before, whitespace_range_before,
};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration option for EmptyForInitializerPad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadOption {
    /// No space before semicolon: `for (;`
    NoSpace,
    /// Space before semicolon: `for ( ;`
    Space,
}

impl PadOption {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_uppercase().as_str() {
            "NOSPACE" => Some(Self::NoSpace),
            "SPACE" => Some(Self::Space),
            _ => None,
        }
    }
}

/// Configuration for EmptyForInitializerPad rule.
#[derive(Debug, Clone)]
pub struct EmptyForInitializerPad {
    /// Padding option: nospace (default) or space.
    pub option: PadOption,
}

impl Default for EmptyForInitializerPad {
    fn default() -> Self {
        Self {
            option: PadOption::NoSpace,
        }
    }
}

impl FromConfig for EmptyForInitializerPad {
    const MODULE_NAME: &'static str = "EmptyForInitializerPad";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .and_then(|s| PadOption::from_str(s))
            .unwrap_or(PadOption::NoSpace);

        Self { option }
    }
}

impl Rule for EmptyForInitializerPad {
    fn name(&self) -> &'static str {
        "EmptyForInitializerPad"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // We only check for_statement nodes
        if node.kind() != "for_statement" {
            return diagnostics;
        }

        // Find the first semicolon in the for statement
        // This is the semicolon after the initializer section
        let semicolons: Vec<_> = node.children().filter(|c| c.kind() == ";").collect();

        if semicolons.is_empty() {
            return diagnostics;
        }

        let first_semi = &semicolons[0];

        // Check if the initializer is empty
        // The initializer is empty if there's nothing between '(' and the first ';'
        // except whitespace
        if !is_empty_initializer(node, first_semi) {
            return diagnostics;
        }

        // Check the whitespace before the semicolon
        let source = ctx.source();
        let semi_pos = first_semi.range().start();

        // Don't check if semicolon is at the beginning of a line (line wrap case)
        // Checkstyle logic: skip if column is 0, or if everything before the semicolon
        // on the same line is whitespace (meaning it's at start of line after indent)
        if is_at_line_start(source, semi_pos) {
            return diagnostics;
        }

        let has_ws = has_whitespace_before(source, semi_pos);

        match self.option {
            PadOption::NoSpace => {
                // Option is nospace, report if there's whitespace before semicolon
                if has_ws
                    && let Some(ws_range) = whitespace_range_before(source, semi_pos)
                    && usize::from(ws_range.start()) > 0
                {
                    // Make sure we're not at the start of file (which counts as whitespace)
                    diagnostics.push(diag_preceded(first_semi, ws_range));
                }
            }
            PadOption::Space => {
                // Option is space, report if there's NO whitespace before semicolon
                if !has_ws {
                    diagnostics.push(diag_not_preceded(first_semi));
                }
            }
        }

        diagnostics
    }
}

/// Check if a position is at the start of a line (possibly after whitespace/indent).
/// This matches checkstyle's logic: skip checking if the token is at the beginning
/// of a line, even if there's indentation before it.
fn is_at_line_start(source: &str, pos: TextSize) -> bool {
    let pos_usize = usize::from(pos);

    // Find the start of the current line
    let line_start = source[..pos_usize].rfind('\n').map(|p| p + 1).unwrap_or(0);

    // Check if everything between line start and position is whitespace
    source[line_start..pos_usize]
        .chars()
        .all(|c| c.is_whitespace())
}

/// Check if the for loop has an empty initializer.
/// The initializer is empty if there's nothing between '(' and the first ';'
/// except whitespace and comments.
fn is_empty_initializer(for_stmt: &CstNode, first_semi: &CstNode) -> bool {
    // Find the opening paren
    let lparen = for_stmt.children().find(|c| c.kind() == "(");
    if lparen.is_none() {
        return false;
    }
    let lparen = lparen.unwrap();

    // Check if there are any meaningful nodes between '(' and first ';'
    // In tree-sitter, if the initializer is empty, the first semicolon comes right after '('
    // If not empty, there will be a local_variable_declaration or expression node
    for child in for_stmt.children() {
        // Skip the lparen and semicolon themselves
        if child.range() == lparen.range() || child.range() == first_semi.range() {
            continue;
        }

        // Only consider children that are between lparen and semicolon
        if child.range().start() >= lparen.range().end()
            && child.range().end() <= first_semi.range().start()
        {
            let kind = child.kind();

            // If we find a local_variable_declaration or any expression, it's not empty
            if !kind.is_empty() && kind != "line_comment" && kind != "block_comment" {
                return false;
            }
        }
    }

    // No initializer nodes found - it's empty
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &EmptyForInitializerPad::default())
    }

    fn check_source_with_config(source: &str, rule: &EmptyForInitializerPad) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_empty_initializer_with_space_nospace_option() {
        // Default is nospace, so space before semicolon is a violation
        let diagnostics =
            check_source("class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(";") && d.kind.body.contains("preceded")),
            "Should detect space before semicolon with nospace option: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_initializer_no_space_nospace_option() {
        // Default is nospace, no space before semicolon is OK
        let diagnostics =
            check_source("class Foo { void m() { int i = 0; for (; i < 1; i++) {} } }");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag no space with nospace option"
        );
    }

    #[test]
    fn test_empty_initializer_no_space_space_option() {
        // Space option, no space before semicolon is a violation
        let rule = EmptyForInitializerPad {
            option: PadOption::Space,
        };
        let diagnostics = check_source_with_config(
            "class Foo { void m() { int i = 0; for (; i < 1; i++) {} } }",
            &rule,
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(";") && d.kind.body.contains("not preceded")),
            "Should detect no space before semicolon with space option: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_initializer_with_space_space_option() {
        // Space option, space before semicolon is OK
        let rule = EmptyForInitializerPad {
            option: PadOption::Space,
        };
        let diagnostics = check_source_with_config(
            "class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }",
            &rule,
        );
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag space with space option"
        );
    }

    #[test]
    fn test_non_empty_initializer_not_checked() {
        // For loops with non-empty initializers should not be checked
        let diagnostics = check_source("class Foo { void m() { for (int i = 0; i < 1; i++) {} } }");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not check for loops with non-empty initializer"
        );
    }

    #[test]
    fn test_line_wrap_not_checked() {
        // Line wraps before semicolon should not be checked
        let diagnostics = check_source("class Foo { void m() { for (\n; ; ) {} } }");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not check when semicolon is on new line"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics =
            check_source("class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }
}
