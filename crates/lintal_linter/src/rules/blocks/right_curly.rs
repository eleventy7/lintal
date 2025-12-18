//! RightCurly rule implementation.
//!
//! Checks the placement of right curly braces ('}') for code blocks.
//! This is a port of the checkstyle RightCurlyCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::are_on_same_line;

/// Policy for placement of right curly braces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RightCurlyOption {
    /// Right curly should be on same line as next part (else, catch, finally)
    /// or alone if it's the last part.
    #[default]
    Same,
    /// Right curly must always be alone on its line.
    Alone,
    /// Right curly alone on line OR entire block on single line.
    AloneOrSingleline,
}


/// Configuration for RightCurly rule.
#[derive(Debug, Clone)]
pub struct RightCurly {
    pub option: RightCurlyOption,
}

impl Default for RightCurly {
    fn default() -> Self {
        Self {
            option: RightCurlyOption::Same,
        }
    }
}

impl FromConfig for RightCurly {
    const MODULE_NAME: &'static str = "RightCurly";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match v.to_uppercase().as_str() {
                "SAME" => RightCurlyOption::Same,
                "ALONE" => RightCurlyOption::Alone,
                "ALONE_OR_SINGLELINE" => RightCurlyOption::AloneOrSingleline,
                _ => RightCurlyOption::Same,
            })
            .unwrap_or(RightCurlyOption::Same);

        Self { option }
    }
}

/// Violation for right curly should be on same line as next part.
#[derive(Debug, Clone)]
pub struct RightCurlyShouldBeSameLine {
    pub column: usize,
}

impl Violation for RightCurlyShouldBeSameLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("'}}' at column {} should be on the same line as the next part of a multi-block statement (else, catch, finally)", self.column)
    }
}

/// Violation for right curly should be alone on line.
#[derive(Debug, Clone)]
pub struct RightCurlyShouldBeAlone {
    pub column: usize,
}

impl Violation for RightCurlyShouldBeAlone {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("'}}' at column {} should be alone on a line", self.column)
    }
}

/// Violation for right curly should have line break before.
#[derive(Debug, Clone)]
pub struct RightCurlyShouldHaveLineBreakBefore {
    pub column: usize,
}

impl Violation for RightCurlyShouldHaveLineBreakBefore {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "'}}' at column {} should have line break before",
            self.column
        )
    }
}

impl Rule for RightCurly {
    fn name(&self) -> &'static str {
        "RightCurly"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            "if_statement" => {
                diagnostics.extend(self.check_if_statement(ctx, node));
            }
            "try_statement" | "try_with_resources_statement" => {
                diagnostics.extend(self.check_try_statement(ctx, node));
            }
            "catch_clause" => {
                diagnostics.extend(self.check_catch_clause(ctx, node));
            }
            "finally_clause" => {
                diagnostics.extend(self.check_finally_clause(ctx, node));
            }
            _ => {}
        }

        diagnostics
    }
}

impl RightCurly {
    /// Check if statement for right curly placement.
    fn check_if_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the block (then branch)
        if let Some(consequence) = node.child_by_field_name("consequence")
            && consequence.kind() == "block"
                && let Some(rcurly) = Self::find_right_curly(ctx, &consequence) {
                    if let Some(lcurly) = Self::find_left_curly(ctx, &consequence) {
                        // Check for line break before violation (SAME option)
                        if self.option == RightCurlyOption::Same
                            && !Self::has_line_break_before(ctx, &rcurly)
                                && !are_on_same_line(ctx.source(), &lcurly, &rcurly)
                            {
                                diagnostics.push(Diagnostic::new(
                                    RightCurlyShouldHaveLineBreakBefore {
                                        column: Self::get_column(ctx, &rcurly),
                                    },
                                    rcurly.range(),
                                ));
                                // Return early - don't check other violations
                                return diagnostics;
                            }
                    }

                    // Check if there's an else clause
                    if let Some(alternative) = node.child_by_field_name("alternative") {
                        // This is not the last block, check SAME logic
                        if self.option == RightCurlyOption::Same
                            && !are_on_same_line(ctx.source(), &rcurly, &alternative) {
                                diagnostics.push(Diagnostic::new(
                                    RightCurlyShouldBeSameLine {
                                        column: Self::get_column(ctx, &rcurly),
                                    },
                                    rcurly.range(),
                                ));
                            }
                    }
                }

        // If there's an else clause that is a block, check it too
        if let Some(alternative) = node.child_by_field_name("alternative")
            && alternative.kind() == "block"
                && let Some(rcurly) = Self::find_right_curly(ctx, &alternative) {
                    // This is the last block in the if-else chain
                    if self.option == RightCurlyOption::Same {
                        // For SAME option, last block should be alone
                        if !Self::is_alone_on_line(ctx, &rcurly) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                }

        diagnostics
    }

    /// Check try statement for right curly placement.
    fn check_try_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the try block
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
                && let Some(rcurly) = Self::find_right_curly(ctx, &body) {
                    // Check if there's a catch or finally clause
                    let has_next = node
                        .named_children()
                        .any(|c| c.kind() == "catch_clause" || c.kind() == "finally_clause");

                    if has_next && self.option == RightCurlyOption::Same {
                        // Find the next catch or finally
                        if let Some(next) = node
                            .named_children()
                            .find(|c| c.kind() == "catch_clause" || c.kind() == "finally_clause")
                            && !are_on_same_line(ctx.source(), &rcurly, &next) {
                                diagnostics.push(Diagnostic::new(
                                    RightCurlyShouldBeSameLine {
                                        column: Self::get_column(ctx, &rcurly),
                                    },
                                    rcurly.range(),
                                ));
                            }
                    }
                }

        diagnostics
    }

    /// Check catch clause for right curly placement.
    fn check_catch_clause(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the catch block
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
                && let Some(rcurly) = Self::find_right_curly(ctx, &body) {
                    if let Some(lcurly) = Self::find_left_curly(ctx, &body) {
                        // Check for line break before violation (SAME option)
                        if self.option == RightCurlyOption::Same
                            && !Self::has_line_break_before(ctx, &rcurly)
                                && !are_on_same_line(ctx.source(), &lcurly, &rcurly)
                            {
                                diagnostics.push(Diagnostic::new(
                                    RightCurlyShouldHaveLineBreakBefore {
                                        column: Self::get_column(ctx, &rcurly),
                                    },
                                    rcurly.range(),
                                ));
                                // Return early - don't check other violations
                                return diagnostics;
                            }
                    }

                    // Check if there's a next catch or finally clause
                    // Skip over comments to find the actual next catch/finally
                    let mut next_sibling = node.next_named_sibling();
                    while let Some(ref sibling) = next_sibling {
                        if sibling.kind() == "catch_clause" || sibling.kind() == "finally_clause" {
                            // Found a catch or finally
                            if self.option == RightCurlyOption::Same
                                && !are_on_same_line(ctx.source(), &rcurly, sibling) {
                                    diagnostics.push(Diagnostic::new(
                                        RightCurlyShouldBeSameLine {
                                            column: Self::get_column(ctx, &rcurly),
                                        },
                                        rcurly.range(),
                                    ));
                                }
                            break;
                        } else if sibling.kind() == "line_comment"
                            || sibling.kind() == "block_comment"
                        {
                            // Skip comments
                            next_sibling = sibling.next_named_sibling();
                        } else {
                            // Some other node, stop looking
                            break;
                        }
                    }
                }

        diagnostics
    }

    /// Check finally clause for right curly placement.
    fn check_finally_clause(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the finally block
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
                && let Some(rcurly) = Self::find_right_curly(ctx, &body) {
                    // Finally is always the last in a try-catch-finally chain
                    if self.option == RightCurlyOption::Same
                        && !Self::is_alone_on_line(ctx, &rcurly) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                }

        diagnostics
    }

    /// Find the right curly brace in a block by searching for the "}" token.
    fn find_right_curly<'a>(_ctx: &CheckContext, block: &'a CstNode<'a>) -> Option<CstNode<'a>> {
        // Look for the closing brace "}" in the block's children
        block.children().find(|&child| child.kind() == "}")
    }

    /// Find the left curly brace in a block by searching for the "{" token.
    fn find_left_curly<'a>(_ctx: &CheckContext, block: &'a CstNode<'a>) -> Option<CstNode<'a>> {
        // Look for the opening brace "{" in the block's children
        block.children().find(|&child| child.kind() == "{")
    }

    /// Check if there's a line break before a node (i.e., only whitespace on line before it).
    fn has_line_break_before(ctx: &CheckContext, node: &CstNode) -> bool {
        let line_index = lintal_source_file::LineIndex::from_source_text(ctx.source());
        let source_code = lintal_source_file::SourceCode::new(ctx.source(), &line_index);
        let node_line = source_code.line_column(node.range().start()).line;
        let line_start = line_index.line_start(node_line, ctx.source());

        let before = &ctx.source()[usize::from(line_start)..usize::from(node.range().start())];
        before.chars().all(|c| c.is_whitespace())
    }

    /// Check if a node is alone on its line (only whitespace before and after it on its line).
    fn is_alone_on_line(ctx: &CheckContext, node: &CstNode) -> bool {
        let line_index = lintal_source_file::LineIndex::from_source_text(ctx.source());
        let source_code = lintal_source_file::SourceCode::new(ctx.source(), &line_index);
        let node_line = source_code.line_column(node.range().start()).line;
        let line_start = line_index.line_start(node_line, ctx.source());
        let line_end = line_index.line_end(node_line, ctx.source());

        // Check before the }
        let before = &ctx.source()[usize::from(line_start)..usize::from(node.range().start())];
        let before_ok = before.chars().all(|c| c.is_whitespace());

        // Check after the }
        let after = &ctx.source()[usize::from(node.range().end())..usize::from(line_end)];
        let after_ok = after.chars().all(|c| c.is_whitespace() || c == '\n' || c == '\r');

        before_ok && after_ok
    }

    /// Get column number (1-indexed) for a node.
    fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
        let line_index = lintal_source_file::LineIndex::from_source_text(ctx.source());
        let source_code = lintal_source_file::SourceCode::new(ctx.source(), &line_index);
        source_code.line_column(node.range().start()).column.get()
    }
}
