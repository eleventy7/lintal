//! RightCurly rule implementation.
//!
//! Checks the placement of right curly braces ('}') for code blocks.
//! This is a port of the checkstyle RightCurlyCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

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

const RELEVANT_KINDS: &[&str] = &[
    "if_statement",
    "try_statement",
    "try_with_resources_statement",
    "catch_clause",
    "finally_clause",
    "enum_declaration",
    "class_declaration",
    "interface_declaration",
    "annotation_type_declaration",
    "record_declaration",
    "method_declaration",
    "constructor_declaration",
    "static_initializer",
    "while_statement",
    "for_statement",
    "enhanced_for_statement",
    "do_statement",
    "switch_expression",
];

struct BraceLineInfo {
    line_start: TextSize,
    line_end: TextSize,
    comment: Option<String>,
    before_is_whitespace: bool,
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
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

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
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

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
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

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

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
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
    fn parse_trailing_comment(after: &str) -> Option<Option<String>> {
        let trimmed = after.trim_start_matches([' ', '\t']);
        if trimmed.is_empty() {
            return Some(None);
        }
        if trimmed.starts_with("//") {
            return Some(Some(trimmed.trim_end().to_string()));
        }
        if trimmed.starts_with("/*") {
            if let Some(end) = trimmed.find("*/") {
                let (comment, rest) = trimmed.split_at(end + 2);
                if rest.trim().is_empty() {
                    return Some(Some(comment.to_string()));
                }
            }
            return None;
        }
        None
    }

    fn brace_line_info(ctx: &CheckContext, brace: &CstNode) -> Option<BraceLineInfo> {
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let line = source_code.line_column(brace.range().start()).line;
        let line_start = line_index.line_start(line, ctx.source());
        let line_end = line_index.line_end(line, ctx.source());
        let line_end_exclusive = line_index.line_end_exclusive(line, ctx.source());
        let line_text = &ctx.source()[usize::from(line_start)..usize::from(line_end_exclusive)];
        let brace_offset = usize::from(brace.range().start() - line_start);
        if brace_offset >= line_text.len() {
            return None;
        }
        let before = &line_text[..brace_offset];
        let before_is_whitespace = before.chars().all(|c| c.is_whitespace());
        let after = &line_text[brace_offset + 1..];
        let comment = Self::parse_trailing_comment(after)?;
        Some(BraceLineInfo {
            line_start,
            line_end,
            comment,
            before_is_whitespace,
        })
    }

    fn line_indent(ctx: &CheckContext, pos: TextSize) -> String {
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let line = source_code.line_column(pos).line;
        let line_start = line_index.line_start(line, ctx.source());
        let prefix = &ctx.source()[usize::from(line_start)..usize::from(pos)];
        prefix.chars().take_while(|c| c.is_whitespace()).collect()
    }

    fn line_has_comment(
        ctx: &CheckContext,
        line_start: TextSize,
        line_end_exclusive: TextSize,
    ) -> bool {
        let line_text = &ctx.source()[usize::from(line_start)..usize::from(line_end_exclusive)];
        line_text.contains("//") || line_text.contains("/*")
    }

    fn fix_same_line(
        &self,
        ctx: &CheckContext,
        rcurly: &CstNode,
        next_token: &CstNode,
    ) -> Option<Fix> {
        let info = Self::brace_line_info(ctx, rcurly)?;
        if !info.before_is_whitespace {
            return None;
        }
        let next_start = next_token.range().start();
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let next_line = source_code.line_column(next_start).line;
        let next_line_start = line_index.line_start(next_line, ctx.source());
        let next_line_end_exclusive = line_index.line_end_exclusive(next_line, ctx.source());
        if let Some(comment) = &info.comment {
            if Self::line_has_comment(ctx, next_line_start, next_line_end_exclusive) {
                return None;
            }
            let delete = Edit::range_deletion(TextRange::new(info.line_start, info.line_end));
            let insert = Edit::insertion("} ".to_string(), next_start);
            let insert_comment = Edit::insertion(format!(" {}", comment), next_line_end_exclusive);
            return Some(Fix::safe_edits(delete, [insert, insert_comment]));
        }
        let delete = Edit::range_deletion(TextRange::new(info.line_start, info.line_end));
        let insert = Edit::insertion("} ".to_string(), next_start);
        Some(Fix::safe_edits(delete, [insert]))
    }

    fn fix_make_alone(
        &self,
        ctx: &CheckContext,
        rcurly: &CstNode,
        next_token: &CstNode,
    ) -> Option<Fix> {
        let between = &ctx.source()
            [usize::from(rcurly.range().end())..usize::from(next_token.range().start())];
        if !between.chars().all(|c| c == ' ' || c == '\t') {
            return None;
        }
        let indent = Self::line_indent(ctx, rcurly.range().start());
        let replacement = format!("\n{}", indent);
        let edit = Edit::range_replacement(
            replacement,
            TextRange::new(rcurly.range().end(), next_token.range().start()),
        );
        Some(Fix::safe_edit(edit))
    }
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
                    && !are_on_same_line(ctx, &lcurly, &rcurly)
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
                let else_token = node.children().find(|c| c.kind() == "else");
                let next_token = else_token.unwrap_or(alternative);
                match self.option {
                    RightCurlyOption::Same => {
                        if !are_on_same_line(ctx, &rcurly, &alternative) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_same_line(ctx, &rcurly, &next_token) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline, // Only ALONE_OR_SINGLELINE allows single-line blocks
                            &consequence,
                        ) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_make_alone(ctx, &rcurly, &next_token) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
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
                        if !are_on_same_line(ctx, &rcurly, &next_clause) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_same_line(ctx, &rcurly, &next_clause) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline, // Only ALONE_OR_SINGLELINE allows single-line blocks
                            &body,
                        ) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_make_alone(ctx, &rcurly, &next_clause) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
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
                    && !are_on_same_line(ctx, &lcurly, &rcurly)
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
                        if !are_on_same_line(ctx, &rcurly, &next) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeSameLine {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_same_line(ctx, &rcurly, &next) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                    RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                        if !Self::should_be_alone(
                            ctx,
                            &rcurly,
                            self.option == RightCurlyOption::AloneOrSingleline, // Only ALONE_OR_SINGLELINE allows single-line blocks
                            &body,
                        ) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_make_alone(ctx, &rcurly, &next) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
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
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let node_line = source_code.line_column(node.range().start()).line;
        let line_start = line_index.line_start(node_line, ctx.source());

        let before = &ctx.source()[usize::from(line_start)..usize::from(node.range().start())];
        before.chars().all(|c| c.is_whitespace())
    }

    /// Check if a node is alone on its line (only whitespace before and after it on its line).
    fn is_alone_on_line(ctx: &CheckContext, node: &CstNode) -> bool {
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
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
        ctx.source_code()
            .line_column(node.range().start())
            .column
            .get()
    }

    /// Check if } should be alone on line.
    /// For ALONE option: } must be alone (single-line blocks NOT allowed).
    /// For ALONE_OR_SINGLELINE: } must be alone OR block can be single-line.
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
            && are_on_same_line(ctx, &lcurly, rcurly)
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
                    are_on_same_line(ctx, &lcurly, rcurly)
                } else {
                    false
                };

                if !is_single_line && !Self::is_alone_on_line(ctx, rcurly) {
                    let mut diagnostic = Diagnostic::new(
                        RightCurlyShouldBeAlone {
                            column: Self::get_column(ctx, rcurly),
                        },
                        rcurly.range(),
                    );
                    if let Some(next_token) = Self::get_next_token(block)
                        && are_on_same_line(ctx, rcurly, &next_token)
                        && let Some(fix) = self.fix_make_alone(ctx, rcurly, &next_token)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    diagnostics.push(diagnostic);
                }
            }
            RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                if !Self::should_be_alone(
                    ctx,
                    rcurly,
                    self.option == RightCurlyOption::AloneOrSingleline, // Only ALONE_OR_SINGLELINE allows single-line blocks
                    block,
                ) {
                    let mut diagnostic = Diagnostic::new(
                        RightCurlyShouldBeAlone {
                            column: Self::get_column(ctx, rcurly),
                        },
                        rcurly.range(),
                    );
                    if let Some(next_token) = Self::get_next_token(block)
                        && are_on_same_line(ctx, rcurly, &next_token)
                        && let Some(fix) = self.fix_make_alone(ctx, rcurly, &next_token)
                    {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    diagnostics.push(diagnostic);
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
                        if are_on_same_line(ctx, &rcurly, &next_token) {
                            let mut diagnostic = Diagnostic::new(
                                RightCurlyShouldBeAlone {
                                    column: Self::get_column(ctx, &rcurly),
                                },
                                rcurly.range(),
                            );
                            if let Some(fix) = self.fix_make_alone(ctx, &rcurly, &next_token) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
                        }
                    }
                }
                RightCurlyOption::Alone | RightCurlyOption::AloneOrSingleline => {
                    if !Self::should_be_alone(
                        ctx,
                        &rcurly,
                        self.option == RightCurlyOption::AloneOrSingleline, // Only ALONE_OR_SINGLELINE allows single-line blocks
                        &block,
                    ) {
                        let mut diagnostic = Diagnostic::new(
                            RightCurlyShouldBeAlone {
                                column: Self::get_column(ctx, &rcurly),
                            },
                            rcurly.range(),
                        );
                        if let Some(next_token) = Self::get_next_token(node)
                            && are_on_same_line(ctx, &rcurly, &next_token)
                            && let Some(fix) = self.fix_make_alone(ctx, &rcurly, &next_token)
                        {
                            diagnostic = diagnostic.with_fix(fix);
                        }
                        diagnostics.push(diagnostic);
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

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_diagnostics::Edit;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_text_size::Ranged;

    fn check_source_with_config(source: &str, rule: &RightCurly) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn apply_edits(source: &str, edits: &[Edit]) -> String {
        let mut result = source.to_string();
        let mut sorted = edits.to_vec();
        sorted.sort_by_key(|e| std::cmp::Reverse(e.start()));
        for edit in sorted {
            let start = usize::from(edit.start());
            let end = usize::from(edit.end());
            let content = edit.content().unwrap_or("");
            result.replace_range(start..end, content);
        }
        result
    }

    #[test]
    fn test_right_curly_same_line_fix_preserves_comment() {
        let source = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        } // end if\n        else {\n            other();\n        }\n    }\n}\n";
        let diagnostics = check_source_with_config(source, &RightCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for right curly SAME");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        } else { // end if\n            other();\n        }\n    }\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_right_curly_same_line_fix_preserves_block_comment() {
        let source = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        } /* end if */\n        else {\n            other();\n        }\n    }\n}\n";
        let diagnostics = check_source_with_config(source, &RightCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for right curly SAME with block comment");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        } else { /* end if */\n            other();\n        }\n    }\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_right_curly_alone_or_singleline_fix_moves_else() {
        let source = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        } else {\n            other();\n        }\n    }\n}\n";
        let rule = RightCurly {
            option: RightCurlyOption::AloneOrSingleline,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for right curly ALONE_OR_SINGLELINE");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo {\n    void m(boolean a) {\n        if (a) {\n            call();\n        }\n        else {\n            other();\n        }\n    }\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_right_curly_try_catch_finally_same_line_fix() {
        let source = "class Foo {\n    void m() {\n        try {\n            call();\n        } // end try\n        catch (Exception e) {\n            handle();\n        } finally {\n            cleanup();\n        }\n    }\n}\n";
        let diagnostics = check_source_with_config(source, &RightCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for right curly try/catch");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo {\n    void m() {\n        try {\n            call();\n        } catch (Exception e) { // end try\n            handle();\n        } finally {\n            cleanup();\n        }\n    }\n}\n";
        assert_eq!(fixed, expected);
    }
}
