//! WhitespaceAfter rule implementation.
//!
//! Checks that a token is followed by whitespace.
//! Checkstyle equivalent: WhitespaceAfter

use std::collections::HashSet;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::rules::whitespace::common::{diag_not_followed, has_whitespace_after};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by WhitespaceAfter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhitespaceAfterToken {
    Comma,
    Semi,
    Typecast,
    LiteralIf,
    LiteralElse,
    LiteralWhile,
    LiteralDo,
    LiteralFor,
    DoWhile,
}

impl WhitespaceAfterToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "COMMA" => Some(Self::Comma),
            "SEMI" => Some(Self::Semi),
            "TYPECAST" => Some(Self::Typecast),
            "LITERAL_IF" => Some(Self::LiteralIf),
            "LITERAL_ELSE" => Some(Self::LiteralElse),
            "LITERAL_WHILE" => Some(Self::LiteralWhile),
            "LITERAL_DO" => Some(Self::LiteralDo),
            "LITERAL_FOR" => Some(Self::LiteralFor),
            "DO_WHILE" => Some(Self::DoWhile),
            _ => None,
        }
    }
}

/// Configuration for WhitespaceAfter rule.
#[derive(Debug, Clone)]
pub struct WhitespaceAfter {
    /// Which tokens to check.
    pub tokens: HashSet<WhitespaceAfterToken>,
}

const RELEVANT_KINDS: &[&str] = &[
    ",",
    ";",
    "cast_expression",
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "enhanced_for_statement",
];

impl Default for WhitespaceAfter {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(WhitespaceAfterToken::Comma);
        tokens.insert(WhitespaceAfterToken::Semi);
        Self { tokens }
    }
}

impl FromConfig for WhitespaceAfter {
    const MODULE_NAME: &'static str = "WhitespaceAfter";

    fn from_config(properties: &Properties) -> Self {
        let tokens_str = properties.get("tokens").copied().unwrap_or("COMMA, SEMI");
        let tokens: HashSet<_> = tokens_str
            .split(',')
            .filter_map(WhitespaceAfterToken::from_str)
            .collect();

        Self {
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
        }
    }
}

impl Rule for WhitespaceAfter {
    fn name(&self) -> &'static str {
        "WhitespaceAfter"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            // Comma: array initializers, parameter lists, etc.
            "," if self.tokens.contains(&WhitespaceAfterToken::Comma) => {
                if !is_followed_by_whitespace_or_valid(ctx, node) {
                    diagnostics.push(diag_not_followed(node));
                }
            }

            // Semicolon: statement terminator, for loop parts
            ";" if self.tokens.contains(&WhitespaceAfterToken::Semi) => {
                // Skip semicolons at end of line or end of for loop
                if !is_semicolon_exempt(ctx, node)
                    && !has_whitespace_after(ctx.source(), node.range().end())
                {
                    diagnostics.push(diag_not_followed(node));
                }
            }

            // Cast expression: (Type) value
            "cast_expression" if self.tokens.contains(&WhitespaceAfterToken::Typecast) => {
                // Find the closing paren of the typecast
                if let Some(rparen) = node.children().find(|c| c.kind() == ")")
                    && !has_whitespace_after(ctx.source(), rparen.range().end())
                {
                    diagnostics.push(diag_not_followed(&rparen));
                }
            }

            // if keyword
            "if_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralIf) => {
                if let Some(kw) = find_keyword(node, "if")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
            }

            // else keyword
            "if_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralElse) => {
                if let Some(kw) = find_keyword(node, "else")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
            }

            // while keyword (in while statement)
            "while_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralWhile) => {
                if let Some(kw) = find_keyword(node, "while")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
            }

            // do keyword
            "do_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralDo) => {
                if let Some(kw) = find_keyword(node, "do")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
                // Also check while in do-while (DO_WHILE token)
                if self.tokens.contains(&WhitespaceAfterToken::DoWhile)
                    && let Some(kw) = find_keyword(node, "while")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
            }

            // for keyword
            "for_statement" | "enhanced_for_statement"
                if self.tokens.contains(&WhitespaceAfterToken::LiteralFor) =>
            {
                if let Some(kw) = find_keyword(node, "for")
                    && !has_whitespace_after(ctx.source(), kw.range().end())
                {
                    diagnostics.push(diag_not_followed(&kw));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

/// Find a keyword in node's children.
fn find_keyword<'a>(node: &CstNode<'a>, keyword: &str) -> Option<CstNode<'a>> {
    node.children().find(|c| c.kind() == keyword)
}

/// Check if comma is followed by whitespace or valid non-whitespace (like closing bracket).
fn is_followed_by_whitespace_or_valid(ctx: &CheckContext, node: &CstNode) -> bool {
    let after_pos = node.range().end();
    let source = ctx.source();

    if let Some(c) = source[usize::from(after_pos)..].chars().next() {
        // Whitespace is always OK
        if c.is_whitespace() {
            return true;
        }
        // Closing brackets/parens are OK after comma (empty trailing)
        if matches!(c, ')' | ']' | '}') {
            return true;
        }
    }
    false
}

/// Check if semicolon is exempt from whitespace-after check.
fn is_semicolon_exempt(ctx: &CheckContext, node: &CstNode) -> bool {
    let after_pos = node.range().end();
    let source = ctx.source();

    // Check what follows
    match source[usize::from(after_pos)..].chars().next() {
        // End of file is OK
        None => true,
        Some(c) => {
            // End of line is OK
            if c == '\n' || c == '\r' {
                return true;
            }
            // Closing paren is OK (for loop)
            if c == ')' {
                return true;
            }
            // Another semicolon is OK (empty for parts)
            if c == ';' {
                return true;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &WhitespaceAfter::default())
    }

    fn check_source_with_config(source: &str, rule: &WhitespaceAfter) -> Vec<Diagnostic> {
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
    fn test_comma_without_space() {
        let diagnostics = check_source("class Foo { int[] a = {1,2}; }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(",") && d.kind.body.contains("not followed")),
            "Should detect comma without space: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_comma_with_space() {
        let diagnostics = check_source("class Foo { int[] a = {1, 2}; }");
        let comma_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(","))
            .collect();
        assert!(
            comma_violations.is_empty(),
            "Should not flag comma with space"
        );
    }

    #[test]
    fn test_semicolon_end_of_line() {
        let diagnostics = check_source("class Foo { int x = 1;\n}");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag semicolon at EOL"
        );
    }

    #[test]
    fn test_semicolon_end_of_file() {
        // Package declaration with semicolon at EOF (no trailing newline)
        let diagnostics = check_source("package foo;");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag semicolon at EOF: {:?}",
            semi_violations
        );
    }

    #[test]
    fn test_for_loop_semicolon() {
        let diagnostics = check_source("class Foo { void m() { for (int i = 0;i < 10; i++) {} } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(";") && d.kind.body.contains("not followed")),
            "Should detect semicolon without space in for loop"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { int[] a = {1,2,3}; }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }
}
