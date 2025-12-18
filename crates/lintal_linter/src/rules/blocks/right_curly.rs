//! RightCurly rule implementation.
//!
//! Checks the placement of right curly braces ('}') for code blocks.
//! This is a port of the checkstyle RightCurlyCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::are_on_same_line;

/// Policy for placement of right curly braces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

use std::collections::HashSet;

/// Tokens that can be checked by RightCurly rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RightCurlyToken {
    LiteralTry,
    LiteralCatch,
    LiteralFinally,
    LiteralIf,
    LiteralElse,
    ClassDef,
    MethodDef,
    CtorDef,
    LiteralFor,
    LiteralWhile,
    LiteralDo,
    StaticInit,
    InstanceInit,
    AnnotationDef,
    EnumDef,
    InterfaceDef,
    RecordDef,
    CompactCtorDef,
    LiteralSwitch,
    LiteralCase,
}

impl RightCurlyToken {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "LITERAL_TRY" => Some(Self::LiteralTry),
            "LITERAL_CATCH" => Some(Self::LiteralCatch),
            "LITERAL_FINALLY" => Some(Self::LiteralFinally),
            "LITERAL_IF" => Some(Self::LiteralIf),
            "LITERAL_ELSE" => Some(Self::LiteralElse),
            "CLASS_DEF" => Some(Self::ClassDef),
            "METHOD_DEF" => Some(Self::MethodDef),
            "CTOR_DEF" => Some(Self::CtorDef),
            "LITERAL_FOR" => Some(Self::LiteralFor),
            "LITERAL_WHILE" => Some(Self::LiteralWhile),
            "LITERAL_DO" => Some(Self::LiteralDo),
            "STATIC_INIT" => Some(Self::StaticInit),
            "INSTANCE_INIT" => Some(Self::InstanceInit),
            "ANNOTATION_DEF" => Some(Self::AnnotationDef),
            "ENUM_DEF" => Some(Self::EnumDef),
            "INTERFACE_DEF" => Some(Self::InterfaceDef),
            "RECORD_DEF" => Some(Self::RecordDef),
            "COMPACT_CTOR_DEF" => Some(Self::CompactCtorDef),
            "LITERAL_SWITCH" => Some(Self::LiteralSwitch),
            "LITERAL_CASE" => Some(Self::LiteralCase),
            _ => None,
        }
    }
}

/// Configuration for RightCurly rule.
#[derive(Debug, Clone)]
pub struct RightCurly {
    pub option: RightCurlyOption,
    pub tokens: HashSet<RightCurlyToken>,
}

impl Default for RightCurly {
    fn default() -> Self {
        // Default tokens match checkstyle's getDefaultTokens
        let mut tokens = HashSet::new();
        tokens.insert(RightCurlyToken::LiteralTry);
        tokens.insert(RightCurlyToken::LiteralCatch);
        tokens.insert(RightCurlyToken::LiteralFinally);
        tokens.insert(RightCurlyToken::LiteralIf);
        tokens.insert(RightCurlyToken::LiteralElse);

        Self {
            option: RightCurlyOption::Same,
            tokens,
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

        // Parse tokens if provided
        let tokens = if let Some(tokens_str) = properties.get("tokens") {
            tokens_str
                .split(',')
                .filter_map(|s| RightCurlyToken::from_str(s.trim()))
                .collect()
        } else {
            // Use default tokens
            let mut tokens = HashSet::new();
            tokens.insert(RightCurlyToken::LiteralTry);
            tokens.insert(RightCurlyToken::LiteralCatch);
            tokens.insert(RightCurlyToken::LiteralFinally);
            tokens.insert(RightCurlyToken::LiteralIf);
            tokens.insert(RightCurlyToken::LiteralElse);
            tokens
        };

        Self { option, tokens }
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
        format!(
            "'}}' at column {} should be on the same line as the next part of a multi-block statement (else, catch, finally)",
            self.column
        )
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
            "if_statement" if self.tokens.contains(&RightCurlyToken::LiteralIf) => {
                diagnostics.extend(self.check_if_statement(ctx, node));
            }
            "try_statement" | "try_with_resources_statement"
                if self.tokens.contains(&RightCurlyToken::LiteralTry) =>
            {
                diagnostics.extend(self.check_try_statement(ctx, node));
            }
            "catch_clause" if self.tokens.contains(&RightCurlyToken::LiteralCatch) => {
                diagnostics.extend(self.check_catch_clause(ctx, node));
            }
            "finally_clause" if self.tokens.contains(&RightCurlyToken::LiteralFinally) => {
                diagnostics.extend(self.check_finally_clause(ctx, node));
            }
            // Checkstyle calls these "others" - they have shouldCheckLastRcurly=true
            "enum_declaration" if self.tokens.contains(&RightCurlyToken::EnumDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "class_declaration" if self.tokens.contains(&RightCurlyToken::ClassDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "interface_declaration" if self.tokens.contains(&RightCurlyToken::InterfaceDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "annotation_type_declaration"
                if self.tokens.contains(&RightCurlyToken::AnnotationDef) =>
            {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "record_declaration" if self.tokens.contains(&RightCurlyToken::RecordDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "method_declaration" if self.tokens.contains(&RightCurlyToken::MethodDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "constructor_declaration" if self.tokens.contains(&RightCurlyToken::CtorDef) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "static_initializer" if self.tokens.contains(&RightCurlyToken::StaticInit) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "while_statement" if self.tokens.contains(&RightCurlyToken::LiteralWhile) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "for_statement" | "enhanced_for_statement"
                if self.tokens.contains(&RightCurlyToken::LiteralFor) =>
            {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "do_statement" if self.tokens.contains(&RightCurlyToken::LiteralDo) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
            }
            "switch_expression" if self.tokens.contains(&RightCurlyToken::LiteralSwitch) => {
                diagnostics.extend(self.check_last_rcurly(ctx, node));
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
            && let Some(rcurly) = Self::find_right_curly(ctx, &consequence)
        {
            if let Some(lcurly) = Self::find_left_curly(ctx, &consequence) {
                // Check for line break before violation (SAME option only)
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
                // This is not the last block
                match self.option {
                    RightCurlyOption::Same => {
                        if !are_on_same_line(ctx.source(), &rcurly, &alternative) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline,
                            &consequence,
                        ) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                }
            } else {
                // This is the last block (no else)
                diagnostics.extend(self.check_last_block_rcurly(ctx, &consequence, &rcurly));
            }
        }

        // If there's an else clause that is a block, check it too
        if let Some(alternative) = node.child_by_field_name("alternative")
            && alternative.kind() == "block"
            && let Some(rcurly) = Self::find_right_curly(ctx, &alternative)
        {
            // This is the last block in the if-else chain
            diagnostics.extend(self.check_last_block_rcurly(ctx, &alternative, &rcurly));
        }

        diagnostics
    }

    /// Check try statement for right curly placement.
    fn check_try_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the try block
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(rcurly) = Self::find_right_curly(ctx, &body)
        {
            // Find the next catch or finally
            let next = node
                .named_children()
                .find(|c| c.kind() == "catch_clause" || c.kind() == "finally_clause");

            if let Some(next_clause) = next {
                // This is not the last block
                match self.option {
                    RightCurlyOption::Same => {
                        if !are_on_same_line(ctx.source(), &rcurly, &next_clause) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline,
                            &body,
                        ) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                }
            } else {
                // No catch or finally - this is the last block
                diagnostics.extend(self.check_last_block_rcurly(ctx, &body, &rcurly));
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
            && let Some(rcurly) = Self::find_right_curly(ctx, &body)
        {
            if let Some(lcurly) = Self::find_left_curly(ctx, &body) {
                // Check for line break before violation (SAME option only)
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
            let mut next_sibling = node.next_named_sibling();
            let mut found_next = None;
            while let Some(ref sibling) = next_sibling {
                if sibling.kind() == "catch_clause" || sibling.kind() == "finally_clause" {
                    found_next = Some(*sibling);
                    break;
                } else if sibling.kind() == "line_comment" || sibling.kind() == "block_comment" {
                    next_sibling = sibling.next_named_sibling();
                } else {
                    break;
                }
            }

            if let Some(next) = found_next {
                // Not the last catch
                match self.option {
                    RightCurlyOption::Same => {
                        if !are_on_same_line(ctx.source(), &rcurly, &next) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline,
                            &body,
                        ) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                }
            } else {
                // Last catch (no finally follows)
                diagnostics.extend(self.check_last_block_rcurly(ctx, &body, &rcurly));
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
            && let Some(rcurly) = Self::find_right_curly(ctx, &body)
        {
            // Finally is always the last in a try-catch-finally chain
            diagnostics.extend(self.check_last_block_rcurly(ctx, &body, &rcurly));
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
        let after_ok = after
            .chars()
            .all(|c| c.is_whitespace() || c == '\n' || c == '\r');

        before_ok && after_ok
    }

    /// Get column number (1-indexed) for a node.
    fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
        let line_index = lintal_source_file::LineIndex::from_source_text(ctx.source());
        let source_code = lintal_source_file::SourceCode::new(ctx.source(), &line_index);
        source_code.line_column(node.range().start()).column.get()
    }

    /// Check if } should be alone on line.
    /// For ALONE_OR_SINGLELINE option, allows single-line blocks.
    fn should_be_alone(
        ctx: &CheckContext,
        rcurly: &CstNode,
        allow_singleline: bool,
        block: &CstNode,
    ) -> bool {
        // If it's already alone, return true
        if Self::is_alone_on_line(ctx, rcurly) {
            return true;
        }

        // For ALONE_OR_SINGLELINE, check if entire block is on one line
        if allow_singleline
            && let Some(lcurly) = Self::find_left_curly(ctx, block)
            && are_on_same_line(ctx.source(), &lcurly, rcurly)
        {
            return true;
        }

        false
    }

    /// Check the last block in a construct (no following else/catch/finally).
    fn check_last_block_rcurly(
        &self,
        ctx: &CheckContext,
        block: &CstNode,
        rcurly: &CstNode,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match self.option {
            RightCurlyOption::Same => {
                // For SAME option, last block should be alone
                // BUT single-line blocks are allowed (checkstyle's shouldBeAloneOnLineWithNotAloneOption)
                let is_single_line = if let Some(lcurly) = Self::find_left_curly(ctx, block) {
                    are_on_same_line(ctx.source(), &lcurly, rcurly)
                } else {
                    false
                };

                if !is_single_line && !Self::is_alone_on_line(ctx, rcurly) {
                    diagnostics.push(Diagnostic::new(
                        RightCurlyShouldBeAlone {
                            column: Self::get_column(ctx, rcurly),
                        },
                        rcurly.range(),
                    ));
                }
            }
            RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                if !Self::should_be_alone(
                    ctx,
                    rcurly,
                    self.option == RightCurlyOption::AloneOrSingleline,
                    block,
                ) {
                    diagnostics.push(Diagnostic::new(
                        RightCurlyShouldBeAlone {
                            column: Self::get_column(ctx, rcurly),
                        },
                        rcurly.range(),
                    ));
                }
            }
        }

        diagnostics
    }

    /// Check constructs that are "last" blocks (enum, class, while, for, etc.)
    /// These should have their } alone on a line.
    fn check_last_rcurly(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the block - could be a "body" field or "block" or class_body, enum_body, etc.
        let block = node.child_by_field_name("body").or_else(|| {
            node.children().find(|c| {
                matches!(
                    c.kind(),
                    "block"
                        | "class_body"
                        | "enum_body"
                        | "interface_body"
                        | "annotation_type_body"
                )
            })
        });

        if let Some(block) = block
            && let Some(rcurly) = Self::find_right_curly(ctx, &block)
        {
            match self.option {
                RightCurlyOption::Same => {
                    // For SAME option with last blocks, check if } is alone on line
                    // Find next token after this statement/declaration
                    if let Some(next_token) = Self::get_next_token(node) {
                        // If next token is on same line as }, that's a violation
                        if are_on_same_line(ctx.source(), &rcurly, &next_token) {
                            diagnostics.push(Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            ));
                        }
                    }
                }
                RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                    if !Self::should_be_alone(
                        ctx,
                        &rcurly,
                        self.option == RightCurlyOption::AloneOrSingleline,
                        &block,
                    ) {
                        diagnostics.push(Diagnostic::new(
                            RightCurlyShouldBeAlone {
                                column: Self::get_column(ctx, &rcurly),
                            },
                            rcurly.range(),
                        ));
                    }
                }
            }
        }

        diagnostics
    }

    /// Get the next sibling token after a node, traversing up the tree if needed.
    /// This mimics checkstyle's getNextToken logic.
    fn get_next_token<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
        // Tree-sitter may have the ; as an unnamed sibling, so let's check all children
        // Actually, let's look at all siblings (both named and unnamed)
        let mut found_current = false;
        if let Some(parent) = node.parent() {
            for child in parent.children() {
                if found_current && child.kind() == ";" {
                    return Some(child);
                }
                if child.range() == node.range() {
                    found_current = true;
                }
            }
        }

        // If no sibling found at this level, traverse up to parent
        if let Some(parent) = node.parent() {
            Self::get_next_token(&parent)
        } else {
            None
        }
    }
}
