//! TypecastParenPad rule implementation.
//!
//! Checks for whitespace padding inside typecast parentheses.
//! Checkstyle equivalent: TypecastParenPad

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::rules::whitespace::common::{
    diag_followed, diag_not_followed, diag_not_preceded, diag_preceded, has_whitespace_after,
    has_whitespace_before, whitespace_range_after, whitespace_range_before,
};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// TypecastParenPad option: space or nospace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypecastParenPadOption {
    Space,
    NoSpace,
}

impl TypecastParenPadOption {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "space" => Self::Space,
            _ => Self::NoSpace,
        }
    }
}

/// Configuration for TypecastParenPad rule.
#[derive(Debug, Clone)]
pub struct TypecastParenPad {
    /// Whether to require space or no space inside parens.
    pub option: TypecastParenPadOption,
}

const RELEVANT_KINDS: &[&str] = &["cast_expression"];

impl Default for TypecastParenPad {
    fn default() -> Self {
        Self {
            option: TypecastParenPadOption::NoSpace,
        }
    }
}

impl FromConfig for TypecastParenPad {
    const MODULE_NAME: &'static str = "TypecastParenPad";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|s| TypecastParenPadOption::from_str(s))
            .unwrap_or(TypecastParenPadOption::NoSpace);

        Self { option }
    }
}

impl Rule for TypecastParenPad {
    fn name(&self) -> &'static str {
        "TypecastParenPad"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check cast_expression nodes
        if node.kind() == "cast_expression" {
            diagnostics.extend(self.check_cast(ctx, node));
        }

        diagnostics
    }
}

impl TypecastParenPad {
    /// Check parens of a cast expression.
    fn check_cast(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the opening and closing parens
        let lparen = node.children().find(|c| c.kind() == "(");
        let rparen = node.children().find(|c| c.kind() == ")");

        if let Some(lparen) = lparen {
            diagnostics.extend(self.check_lparen(ctx, &lparen));
        }

        if let Some(rparen) = rparen {
            diagnostics.extend(self.check_rparen(ctx, &rparen));
        }

        diagnostics
    }

    /// Check whitespace after opening paren.
    fn check_lparen(&self, ctx: &CheckContext, lparen: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let after_pos = lparen.range().end();

        // Check if the next char is a closing paren (empty parens)
        if let Some(next_char) = ctx.source()[usize::from(after_pos)..].chars().next()
            && next_char == ')'
        {
            // Empty parens, don't check
            return diagnostics;
        }

        let has_space = has_whitespace_after(ctx.source(), after_pos);

        match self.option {
            TypecastParenPadOption::NoSpace => {
                if has_space && let Some(ws_range) = whitespace_range_after(ctx.source(), after_pos)
                {
                    diagnostics.push(diag_followed(lparen, ws_range));
                }
            }
            TypecastParenPadOption::Space => {
                if !has_space {
                    diagnostics.push(diag_not_followed(lparen));
                }
            }
        }

        diagnostics
    }

    /// Check whitespace before closing paren.
    fn check_rparen(&self, ctx: &CheckContext, rparen: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let before_pos = rparen.range().start();

        // Check if the previous char is an opening paren (empty parens)
        if before_pos > 0.into()
            && let Some(prev_char) = ctx.source()[..usize::from(before_pos)].chars().last()
            && prev_char == '('
        {
            // Empty parens, don't check
            return diagnostics;
        }

        let has_space = has_whitespace_before(ctx.source(), before_pos);

        match self.option {
            TypecastParenPadOption::NoSpace => {
                if has_space
                    && let Some(ws_range) = whitespace_range_before(ctx.source(), before_pos)
                {
                    diagnostics.push(diag_preceded(rparen, ws_range));
                }
            }
            TypecastParenPadOption::Space => {
                if !has_space {
                    diagnostics.push(diag_not_preceded(rparen));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &TypecastParenPad::default())
    }

    fn check_source_with_config(source: &str, rule: &TypecastParenPad) -> Vec<Diagnostic> {
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
    fn test_nospace_with_space() {
        let diagnostics = check_source("class Foo { void m() { Object o = ( String ) x; } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is followed")),
            "Should detect space after lparen in cast"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("')' is preceded")),
            "Should detect space before rparen in cast"
        );
    }

    #[test]
    fn test_nospace_without_space() {
        let diagnostics = check_source("class Foo { void m() { Object o = (String) x; } }");
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("'('") || d.kind.body.contains("')'"))
            .collect();
        assert!(
            paren_violations.is_empty(),
            "Should not flag parens without space"
        );
    }

    #[test]
    fn test_space_without_space() {
        let rule = TypecastParenPad {
            option: TypecastParenPadOption::Space,
        };
        let diagnostics =
            check_source_with_config("class Foo { void m() { Object o = (String) x; } }", &rule);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is not followed")),
            "Should detect missing space after lparen"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("')' is not preceded")),
            "Should detect missing space before rparen"
        );
    }

    #[test]
    fn test_space_with_space() {
        let rule = TypecastParenPad {
            option: TypecastParenPadOption::Space,
        };
        let diagnostics =
            check_source_with_config("class Foo { void m() { Object o = ( String ) x; } }", &rule);
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("'('") || d.kind.body.contains("')'"))
            .collect();
        assert!(
            paren_violations.is_empty(),
            "Should not flag parens with space when option=space"
        );
    }

    #[test]
    fn test_multiple_casts() {
        let diagnostics =
            check_source("class Foo { void m() { Object o = (String)x; int i = ( int ) o; } }");
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.kind.body.contains("'(' is followed"))
                .count(),
            1,
            "Should detect one lparen with space"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.kind.body.contains("')' is preceded"))
                .count(),
            1,
            "Should detect one rparen with space"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { void m() { Object o = ( String ) x; } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }

    #[test]
    fn test_does_not_affect_method_calls() {
        let diagnostics = check_source("class Foo { void m( int x ) {} }");
        // Should not produce any violations for method parens
        assert!(
            diagnostics.is_empty(),
            "Should not check method definition parens"
        );
    }
}
