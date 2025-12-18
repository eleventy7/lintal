//! NoWhitespaceBefore rule implementation.
//!
//! Checks that there is no whitespace before specific tokens.
//! Checkstyle equivalent: NoWhitespaceBefore

use std::collections::HashSet;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;

use crate::rules::whitespace::common::{diag_preceded, whitespace_range_before};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by NoWhitespaceBefore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoWhitespaceBeforeToken {
    Comma,
    Semi,
    PostInc,
    PostDec,
    Ellipsis,
    LabeledStat,
    Dot,
    MethodRef,
    GenericStart,
    GenericEnd,
}

impl NoWhitespaceBeforeToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "COMMA" => Some(Self::Comma),
            "SEMI" => Some(Self::Semi),
            "POST_INC" => Some(Self::PostInc),
            "POST_DEC" => Some(Self::PostDec),
            "ELLIPSIS" => Some(Self::Ellipsis),
            "LABELED_STAT" => Some(Self::LabeledStat),
            "DOT" => Some(Self::Dot),
            "METHOD_REF" => Some(Self::MethodRef),
            "GENERIC_START" => Some(Self::GenericStart),
            "GENERIC_END" => Some(Self::GenericEnd),
            _ => None,
        }
    }
}

/// Configuration for NoWhitespaceBefore rule.
#[derive(Debug, Clone)]
pub struct NoWhitespaceBefore {
    /// Which tokens to check.
    pub tokens: HashSet<NoWhitespaceBeforeToken>,
    /// Allow line breaks before token.
    pub allow_line_breaks: bool,
}

impl Default for NoWhitespaceBefore {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(NoWhitespaceBeforeToken::Comma);
        tokens.insert(NoWhitespaceBeforeToken::Semi);
        tokens.insert(NoWhitespaceBeforeToken::PostInc);
        tokens.insert(NoWhitespaceBeforeToken::PostDec);
        tokens.insert(NoWhitespaceBeforeToken::Ellipsis);
        tokens.insert(NoWhitespaceBeforeToken::LabeledStat);
        Self {
            tokens,
            allow_line_breaks: false,
        }
    }
}

impl FromConfig for NoWhitespaceBefore {
    const MODULE_NAME: &'static str = "NoWhitespaceBefore";

    fn from_config(properties: &Properties) -> Self {
        let tokens_str = properties.get("tokens").copied().unwrap_or("");
        let tokens: HashSet<_> = if tokens_str.is_empty() {
            Self::default().tokens
        } else {
            tokens_str
                .split(',')
                .filter_map(NoWhitespaceBeforeToken::from_str)
                .collect()
        };

        let allow_line_breaks = properties
            .get("allowLineBreaks")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);

        Self {
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
            allow_line_breaks,
        }
    }
}

impl Rule for NoWhitespaceBefore {
    fn name(&self) -> &'static str {
        "NoWhitespaceBefore"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            // Comma: a, b, c
            "," if self.tokens.contains(&NoWhitespaceBeforeToken::Comma) => {
                if let Some(ws_range) = self.check_whitespace_before(ctx, node) {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Semicolon: statement;
            ";" if self.tokens.contains(&NoWhitespaceBeforeToken::Semi) => {
                // Skip semicolons in empty for loop initializers or conditions
                if !is_in_empty_for_initializer_or_condition(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Post increment: i++
            "++" if self.tokens.contains(&NoWhitespaceBeforeToken::PostInc) => {
                if is_post_inc(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Post decrement: i--
            "--" if self.tokens.contains(&NoWhitespaceBeforeToken::PostDec) => {
                if is_post_dec(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Ellipsis: String... args
            "..." if self.tokens.contains(&NoWhitespaceBeforeToken::Ellipsis) => {
                if let Some(ws_range) = self.check_whitespace_before_ellipsis(ctx, node) {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Labeled statement: label:
            ":" if self.tokens.contains(&NoWhitespaceBeforeToken::LabeledStat) => {
                if is_labeled_statement_colon(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Dot: obj.field
            "." if self.tokens.contains(&NoWhitespaceBeforeToken::Dot) => {
                if let Some(ws_range) = self.check_whitespace_before(ctx, node) {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Method reference: String::new
            "::" if self.tokens.contains(&NoWhitespaceBeforeToken::MethodRef) => {
                if let Some(ws_range) = self.check_whitespace_before(ctx, node) {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Generic start: List<String>
            "<" if self.tokens.contains(&NoWhitespaceBeforeToken::GenericStart) => {
                if is_generic_start(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            // Generic end: List<String>
            ">" if self.tokens.contains(&NoWhitespaceBeforeToken::GenericEnd) => {
                if is_generic_end(node)
                    && let Some(ws_range) = self.check_whitespace_before(ctx, node)
                {
                    diagnostics.push(diag_preceded(node, ws_range));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

impl NoWhitespaceBefore {
    /// Check if there's unwanted whitespace before an ellipsis token.
    /// Special handling for ellipsis because tree-sitter's positioning might be affected
    /// by type annotations. We check the source text directly like checkstyle does.
    /// Returns the range of whitespace if found.
    fn check_whitespace_before_ellipsis(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
    ) -> Option<lintal_text_size::TextRange> {
        let source = ctx.source();
        let ellipsis_pos = node.range().start();

        // Check directly before the ellipsis in source text
        // This is similar to how checkstyle checks: it looks at the source line/column
        if let Some(ws_range) = whitespace_range_before(source, ellipsis_pos)
            && self.should_report_whitespace(source, ws_range)
        {
            return Some(ws_range);
        }

        None
    }

    /// Check if there's unwanted whitespace before a token.
    /// Returns the range of whitespace if found.
    fn check_whitespace_before(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
    ) -> Option<lintal_text_size::TextRange> {
        let before_pos = node.range().start();
        let source = ctx.source();

        // Check for whitespace before the token
        if let Some(ws_range) = whitespace_range_before(source, before_pos)
            && self.should_report_whitespace(source, ws_range)
        {
            return Some(ws_range);
        }
        None
    }

    /// Check if whitespace should be reported based on allow_line_breaks setting.
    fn should_report_whitespace(
        &self,
        source: &str,
        ws_range: lintal_text_size::TextRange,
    ) -> bool {
        // Check if token is at start of file
        if ws_range.start() == TextSize::new(0) {
            // If allowLineBreaks is true, this is OK
            // If allowLineBreaks is false, this is a violation
            return !self.allow_line_breaks;
        }

        // If we allow line breaks, check if whitespace is only newline(s) at start of line
        if self.allow_line_breaks {
            let ws_text = &source[usize::from(ws_range.start())..usize::from(ws_range.end())];

            // Check if the whitespace contains a newline
            if ws_text.contains('\n') || ws_text.contains('\r') {
                // Check if everything before the token on this line is whitespace
                // Find the position of the last newline before the token
                let before_ws = &source[..usize::from(ws_range.start())];
                if let Some(last_nl_pos) = before_ws.rfind('\n') {
                    // Everything between the newline and the token should be whitespace
                    let line_start = last_nl_pos + 1;
                    let token_pos = ws_range.end(); // The token starts after the whitespace
                    let line_before_token = &source[line_start..usize::from(token_pos)];
                    if line_before_token.chars().all(|c| c.is_whitespace()) {
                        return false; // Token is at start of line with only whitespace before
                    }
                }
            }
        }

        true // Report the whitespace
    }
}

/// Check if node is a post-increment operator (not pre-increment).
fn is_post_inc(node: &CstNode) -> bool {
    // Post-increment appears in update_expression
    node.parent()
        .is_some_and(|p| p.kind() == "update_expression" && is_postfix_update(&p, node))
}

/// Check if node is a post-decrement operator (not pre-decrement).
fn is_post_dec(node: &CstNode) -> bool {
    // Post-decrement appears in update_expression
    node.parent()
        .is_some_and(|p| p.kind() == "update_expression" && is_postfix_update(&p, node))
}

/// Check if the update expression is postfix (operator after operand).
fn is_postfix_update(update_expr: &CstNode, operator: &CstNode) -> bool {
    // In postfix form, the operand comes before the operator
    // Find the operand (identifier, field access, array access, etc.)
    for child in update_expr.children() {
        if child.range() != operator.range()
            && (child.kind() == "identifier"
                || child.kind() == "field_access"
                || child.kind() == "array_access"
                || child.kind() == "parenthesized_expression")
        {
            // If operand starts before operator, it's postfix
            return child.range().start() < operator.range().start();
        }
    }
    false
}

/// Check if colon is part of a labeled statement (not ternary, case, etc.).
fn is_labeled_statement_colon(node: &CstNode) -> bool {
    // Labeled statement colon is a direct child of labeled_statement
    node.parent()
        .is_some_and(|p| p.kind() == "labeled_statement")
}

/// Check if < is a generic type parameter start (not less-than comparison).
fn is_generic_start(node: &CstNode) -> bool {
    // Generic type parameters appear in type_arguments or type_parameters
    node.parent().is_some_and(|p| {
        p.kind() == "type_arguments"
            || p.kind() == "type_parameters"
            || p.kind() == "diamond_operator"
    })
}

/// Check if > is a generic type parameter end (not greater-than comparison).
fn is_generic_end(node: &CstNode) -> bool {
    // Generic type parameters appear in type_arguments or type_parameters
    node.parent().is_some_and(|p| {
        p.kind() == "type_arguments"
            || p.kind() == "type_parameters"
            || p.kind() == "diamond_operator"
    })
}

/// Check if semicolon is in empty for loop initializer or condition.
/// For example: for (; i < 10; i++) or for (int i = 0; ; i++)
fn is_in_empty_for_initializer_or_condition(semi: &CstNode) -> bool {
    // Walk up to find if we're in a for_statement
    let mut current = semi.parent();
    while let Some(node) = current {
        if node.kind() == "for_statement" {
            // Check if the semicolon is one of the special for-loop semicolons
            // In Java grammar, for loop has: for ( [init] ; [condition] ; [update] )
            // We need to check if semi is the first or second semicolon in an empty slot

            // Get all semicolons in the for statement
            let semis: Vec<_> = node.children().filter(|c| c.kind() == ";").collect();

            if semis.len() >= 2 {
                let is_first_semi = semis[0].range() == semi.range();
                let is_second_semi = semis[1].range() == semi.range();

                if is_first_semi {
                    // Check if init is empty: look for content between ( and first ;
                    if let Some(lparen) = node.children().find(|c| c.kind() == "(") {
                        let between = node.children().filter(|c| {
                            c.range().start() > lparen.range().end()
                                && c.range().end() < semi.range().start()
                                && !c.kind().is_empty()
                                && c.kind() != "("
                                && c.kind() != ";"
                        });
                        return between.count() == 0;
                    }
                } else if is_second_semi {
                    // Check if condition is empty: look for content between first ; and second ;
                    let first_semi = &semis[0];
                    let between = node.children().filter(|c| {
                        c.range().start() > first_semi.range().end()
                            && c.range().end() < semi.range().start()
                            && !c.kind().is_empty()
                            && c.kind() != ";"
                    });
                    return between.count() == 0;
                }
            }
            break;
        }
        current = node.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &NoWhitespaceBefore::default())
    }

    fn check_source_with_config(source: &str, rule: &NoWhitespaceBefore) -> Vec<Diagnostic> {
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
    fn test_comma_with_space() {
        let diagnostics = check_source("class Foo { void m(int a , int b) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(",") && d.kind.body.contains("preceded")),
            "Should detect comma with space before: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_comma_without_space() {
        let diagnostics = check_source("class Foo { void m(int a, int b) {} }");
        let comma_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(","))
            .collect();
        assert!(
            comma_violations.is_empty(),
            "Should not flag comma without space before"
        );
    }

    #[test]
    fn test_semicolon_with_space() {
        let diagnostics = check_source("class Foo { void m() { int x = 1 ; } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(";") && d.kind.body.contains("preceded")),
            "Should detect semicolon with space before: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_semicolon_without_space() {
        let diagnostics = check_source("class Foo { void m() { int x = 1; } }");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag semicolon without space before"
        );
    }

    #[test]
    fn test_post_increment_with_space() {
        let diagnostics = check_source("class Foo { void m() { int i = 0; i ++; } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("++") && d.kind.body.contains("preceded")),
            "Should detect post-increment with space before: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_post_increment_without_space() {
        let diagnostics = check_source("class Foo { void m() { int i = 0; i++; } }");
        let inc_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("++"))
            .collect();
        assert!(
            inc_violations.is_empty(),
            "Should not flag post-increment without space before"
        );
    }

    #[test]
    fn test_pre_increment_not_flagged() {
        let diagnostics = check_source("class Foo { void m() { int i = 0; ++i; } }");
        let inc_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("++"))
            .collect();
        assert!(inc_violations.is_empty(), "Should not flag pre-increment");
    }

    #[test]
    fn test_empty_for_loop() {
        let mut config = NoWhitespaceBefore::default();
        config.tokens.clear();
        config.tokens.insert(NoWhitespaceBeforeToken::Semi);
        config.allow_line_breaks = true;

        let diagnostics =
            check_source_with_config("class Foo { void m() { for (; ; ) {} } }", &config);
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(
            semi_violations.is_empty(),
            "Should not flag semicolons in empty for loop: {:?}",
            semi_violations
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { void m(int a , int b) { int x = 1 ; } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }
}
