//! LeftCurly rule implementation.
//!
//! Checks the placement of left curly braces ('{') for code blocks.
//! This is a port of the checkstyle LeftCurlyCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::are_on_same_line;

/// Policy for placement of left curly braces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftCurlyOption {
    /// Left curly must be at end of line (on same line as statement).
    #[default]
    Eol,
    /// Left curly must be on a new line (alone).
    Nl,
    /// Left curly on new line if it won't fit (simplified implementation).
    Nlow,
}

/// Configuration for LeftCurly rule.
#[derive(Debug, Clone)]
pub struct LeftCurly {
    pub option: LeftCurlyOption,
    pub ignore_enums: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "annotation_type_declaration",
    "enum_declaration",
    "record_declaration",
    "method_declaration",
    "constructor_declaration",
    "if_statement",
    "while_statement",
    "for_statement",
    "enhanced_for_statement",
    "do_statement",
    "try_statement",
    "try_with_resources_statement",
    "catch_clause",
    "finally_clause",
    "static_initializer",
    "lambda_expression",
    "switch_expression",
    "switch_statement",
    "switch_block_statement_group",
    "enum_constant",
];

struct BraceLineInfo {
    line: lintal_source_file::OneIndexed,
    line_start: TextSize,
    line_end: TextSize,
    line_end_exclusive: TextSize,
    brace_offset: usize,
    trailing_content: String,
    before_is_whitespace: bool,
}

impl Default for LeftCurly {
    fn default() -> Self {
        Self {
            option: LeftCurlyOption::Eol,
            ignore_enums: true,
        }
    }
}

impl FromConfig for LeftCurly {
    const MODULE_NAME: &'static str = "LeftCurly";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match v.to_uppercase().as_str() {
                "EOL" => LeftCurlyOption::Eol,
                "NL" => LeftCurlyOption::Nl,
                "NLOW" => LeftCurlyOption::Nlow,
                _ => LeftCurlyOption::Eol,
            })
            .unwrap_or(LeftCurlyOption::Eol);

        let ignore_enums = properties
            .get("ignoreEnums")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);

        Self {
            option,
            ignore_enums,
        }
    }
}

/// Violation for left curly should be on a new line.
#[derive(Debug, Clone)]
pub struct LeftCurlyShouldBeOnNewLine {
    pub column: usize,
}

impl Violation for LeftCurlyShouldBeOnNewLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{{' at column {} should be on a new line", self.column)
    }
}

/// Violation for left curly should be on the previous line.
#[derive(Debug, Clone)]
pub struct LeftCurlyShouldBeOnPreviousLine {
    pub column: usize,
}

impl Violation for LeftCurlyShouldBeOnPreviousLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!(
            "'{{' at column {} should be on the previous line",
            self.column
        )
    }
}

/// Violation for left curly should have line break after.
#[derive(Debug, Clone)]
pub struct LeftCurlyShouldHaveLineBreakAfter {
    pub column: usize,
}

impl Violation for LeftCurlyShouldHaveLineBreakAfter {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!(
            "'{{' at column {} should have line break after",
            self.column
        )
    }
}

impl Rule for LeftCurly {
    fn name(&self) -> &'static str {
        "LeftCurly"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            "class_declaration"
            | "interface_declaration"
            | "annotation_type_declaration"
            | "enum_declaration"
            | "record_declaration" => {
                diagnostics.extend(self.check_type_declaration(ctx, node));
            }
            "method_declaration" | "constructor_declaration" => {
                diagnostics.extend(self.check_method_or_ctor(ctx, node));
            }
            "if_statement" => {
                diagnostics.extend(self.check_if_statement(ctx, node));
            }
            "while_statement" | "for_statement" | "enhanced_for_statement" | "do_statement" => {
                diagnostics.extend(self.check_loop_statement(ctx, node));
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
            "static_initializer" => {
                diagnostics.extend(self.check_static_init(ctx, node));
            }
            "lambda_expression" => {
                diagnostics.extend(self.check_lambda(ctx, node));
            }
            "switch_expression" | "switch_statement" => {
                diagnostics.extend(self.check_switch(ctx, node));
            }
            "switch_block_statement_group" => {
                diagnostics.extend(self.check_switch_case(ctx, node));
            }
            "enum_constant" => {
                diagnostics.extend(self.check_enum_constant(ctx, node));
            }
            _ => {}
        }

        diagnostics
    }
}

impl LeftCurly {
    /// Parse trailing content after the left curly brace.
    /// Returns Some(content) if we can safely generate a fix, None if content is too complex.
    ///
    /// Note: We return None for patterns like `{ }` or `{  }` because:
    /// 1. These patterns often have SingleSpaceSeparator violations too
    /// 2. Our fix would conflict with SingleSpaceSeparator's fix
    /// 3. The fixer currently doesn't properly skip ALL edits from a fix when one overlaps
    fn parse_trailing_content(after: &str) -> Option<String> {
        let trimmed = after.trim_end();
        if trimmed.is_empty() {
            return Some(String::new());
        }
        let content = trimmed.trim_start();
        // Don't generate fix for `{ }` patterns to avoid fix conflicts
        if content.starts_with('}') {
            return None;
        }
        // For comments, preserve them (with a leading space)
        if content.starts_with("//") || content.starts_with("/*") {
            return Some(format!(" {}", content));
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
        let trailing_content = Self::parse_trailing_content(after)?;
        Some(BraceLineInfo {
            line,
            line_start,
            line_end,
            line_end_exclusive,
            brace_offset,
            trailing_content,
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

    fn fix_move_to_previous_line(&self, ctx: &CheckContext, brace: &CstNode) -> Option<Fix> {
        let info = Self::brace_line_info(ctx, brace)?;
        if !info.before_is_whitespace {
            return None;
        }
        if info.line == lintal_source_file::OneIndexed::MIN {
            return None;
        }
        let line_index = ctx.line_index();
        let prev_line = info.line.saturating_sub(1);
        let prev_line_start = line_index.line_start(prev_line, ctx.source());
        let prev_line_end_exclusive = line_index.line_end_exclusive(prev_line, ctx.source());
        let prev_line_text =
            &ctx.source()[usize::from(prev_line_start)..usize::from(prev_line_end_exclusive)];
        let needs_space = prev_line_text
            .chars()
            .last()
            .is_some_and(|c| !c.is_whitespace());
        let mut insertion = String::new();
        if needs_space {
            insertion.push(' ');
        }
        insertion.push('{');
        insertion.push_str(&info.trailing_content);
        let delete = Edit::range_deletion(TextRange::new(info.line_start, info.line_end));
        let insert = Edit::insertion(insertion, prev_line_end_exclusive);
        Some(Fix::safe_edits(delete, [insert]))
    }

    fn fix_move_to_new_line(
        &self,
        ctx: &CheckContext,
        brace: &CstNode,
        start_token: &CstNode,
    ) -> Option<Fix> {
        let info = Self::brace_line_info(ctx, brace)?;
        let line_text =
            &ctx.source()[usize::from(info.line_start)..usize::from(info.line_end_exclusive)];
        let mut delete_offset = info.brace_offset;
        while delete_offset > 0 {
            let ch = line_text.as_bytes()[delete_offset - 1];
            if ch == b' ' || ch == b'\t' {
                delete_offset -= 1;
            } else {
                break;
            }
        }
        let delete_start = info.line_start + TextSize::new(delete_offset as u32);
        let delete = Edit::range_deletion(TextRange::new(delete_start, info.line_end_exclusive));
        let indent = Self::line_indent(ctx, start_token.range().start());
        let mut insertion = String::new();
        insertion.push('\n');
        insertion.push_str(&indent);
        insertion.push('{');
        insertion.push_str(&info.trailing_content);
        let insert = Edit::insertion(insertion, info.line_end_exclusive);
        Some(Fix::safe_edits(delete, [insert]))
    }

    /// Find the left curly brace in a node.
    fn find_left_curly<'a>(_ctx: &CheckContext, node: &'a CstNode<'a>) -> Option<CstNode<'a>> {
        node.children().find(|&child| child.kind() == "{")
    }

    /// Get column number (1-indexed) for a node.
    fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
        ctx.source_code()
            .line_column(node.range().start())
            .column
            .get()
    }

    /// Check if there's only whitespace before a node on its line.
    /// This matches checkstyle's CommonUtil.hasWhitespaceBefore logic:
    /// Returns true if the node is at the start of the line OR if there's only whitespace before it.
    fn has_whitespace_before(ctx: &CheckContext, node: &CstNode) -> bool {
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let node_line = source_code.line_column(node.range().start()).line;
        let line_start = line_index.line_start(node_line, ctx.source());

        let before = &ctx.source()[usize::from(line_start)..usize::from(node.range().start())];
        // Return true if empty (at start of line) or if all characters are whitespace
        before.is_empty() || before.chars().all(|c| c.is_whitespace())
    }

    /// Check if there's a line break after the left curly.
    /// This follows checkstyle's hasLineBreakAfter logic.
    fn has_line_break_after(&self, ctx: &CheckContext, lcurly: &CstNode) -> bool {
        let source_code = ctx.source_code();
        let lcurly_line = source_code.line_column(lcurly.range().start()).line;

        // Get the parent to determine the context
        let parent = lcurly.parent();

        let next_token = if let Some(parent) = parent {
            if parent.kind() == "block" || parent.kind() == "switch_block" {
                // For SLIST (statement list blocks), get next sibling after {
                // This will be either a statement or }
                // Use get_next_token to get the next sibling (including anonymous nodes like })
                Self::get_next_token(lcurly)
            } else if parent.kind() == "class_body"
                || parent.kind() == "enum_body"
                || parent.kind() == "interface_body"
                || parent.kind() == "annotation_type_body"
            {
                // For OBJBLOCK (class/enum/interface bodies)
                // Only check if ignoreEnums is false and it's an enum
                if !self.ignore_enums {
                    // Check if this is an enum body by looking up the tree
                    if let Some(grand_parent) = parent.parent() {
                        if grand_parent.kind() == "enum_declaration" {
                            // Get next sibling after the left curly
                            Self::get_next_token(lcurly)
                        } else {
                            // Not an enum, always has line break
                            return true;
                        }
                    } else {
                        return true;
                    }
                } else {
                    // ignoreEnums is true, always return true
                    return true;
                }
            } else {
                // Unknown parent type
                return true;
            }
        } else {
            return true;
        };

        // Check if nextToken exists and is on same line
        if let Some(next) = next_token {
            // If next is }, that's OK (empty block)
            if next.kind() == "}" {
                return true;
            }

            let next_line = source_code.line_column(next.range().start()).line;
            lcurly_line != next_line
        } else {
            true
        }
    }

    /// Get the next sibling token.
    /// Skips comments to match checkstyle's behavior where comments are not in the AST.
    fn get_next_token<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
        // Use the parent to iterate through all children (named and unnamed)
        if let Some(parent) = node.parent() {
            let mut found_current = false;
            for child in parent.children() {
                if found_current && !child.kind().is_empty() {
                    // Skip comments to match checkstyle behavior
                    if !matches!(child.kind(), "line_comment" | "block_comment") {
                        return Some(child);
                    }
                }
                if child.range() == node.range() {
                    found_current = true;
                }
            }
        }
        None
    }

    /// Find the start token for a node (skipping annotations and modifiers).
    fn skip_modifier_annotations<'a>(node: &'a CstNode<'a>) -> CstNode<'a> {
        // Look for modifiers
        if let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") {
            // Find last annotation in modifiers
            let mut last_annotation = None;
            for child in modifiers.children() {
                if child.kind() == "marker_annotation"
                    || child.kind() == "annotation"
                    || child.kind().contains("annotation")
                {
                    last_annotation = Some(child);
                }
            }

            if let Some(_last_anno) = last_annotation {
                // Return the next sibling after modifiers
                if let Some(next) = modifiers.next_named_sibling() {
                    return next;
                }
            }
        }

        *node
    }

    /// Check type declarations (class, interface, enum, annotation, record).
    fn check_type_declaration(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Note: ignoreEnums only affects line break checking in hasLineBreakAfter,
        // not the placement of the enum's own left curly brace.
        // So we always check enum declarations here.

        // Find the class_body, interface_body, enum_body, etc.
        // First try the "body" field, then search children
        let body = node.child_by_field_name("body").or_else(|| {
            node.children().find(|c| {
                matches!(
                    c.kind(),
                    "class_body" | "interface_body" | "enum_body" | "annotation_type_body"
                )
            })
        });

        if let Some(body) = body
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            // Skip modifier annotations to find the start token
            let start_token = Self::skip_modifier_annotations(node);
            diagnostics.extend(self.verify_brace(ctx, &lcurly, &start_token));
        }

        diagnostics
    }

    /// Check method or constructor declarations.
    fn check_method_or_ctor(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the block or constructor_body
        if let Some(body) = node.child_by_field_name("body")
            && (body.kind() == "block" || body.kind() == "constructor_body")
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            let start_token = Self::skip_modifier_annotations(node);
            diagnostics.extend(self.verify_brace(ctx, &lcurly, &start_token));
        }

        diagnostics
    }

    /// Check if statement.
    fn check_if_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check consequence (then block)
        if let Some(consequence) = node.child_by_field_name("consequence")
            && consequence.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &consequence)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        // Check alternative (else block)
        if let Some(alternative) = node.child_by_field_name("alternative")
            && alternative.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &alternative)
        {
            // Find the "else" keyword
            let else_keyword = node.children().find(|c| c.kind() == "else");
            let start = else_keyword.unwrap_or(*node);
            diagnostics.extend(self.verify_brace(ctx, &lcurly, &start));
        }

        diagnostics
    }

    /// Check loop statements (while, for, do).
    fn check_loop_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check try statement.
    fn check_try_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check catch clause.
    fn check_catch_clause(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check finally clause.
    fn check_finally_clause(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check static initializer.
    fn check_static_init(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check lambda expression.
    fn check_lambda(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check switch statement or expression.
    fn check_switch(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "switch_block"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Check switch case/default labels.
    fn check_switch_case(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Look for a block in the case statement
        if let Some(block) = node.children().find(|c| c.kind() == "block")
            && let Some(lcurly) = Self::find_left_curly(ctx, &block)
        {
            // The start token is the case/default label
            let start = node.children().next().unwrap_or(*node);
            diagnostics.extend(self.verify_brace(ctx, &lcurly, &start));
        }

        diagnostics
    }

    /// Check enum constant with body.
    fn check_enum_constant(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Look for a class_body in the enum constant
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "class_body"
            && let Some(lcurly) = Self::find_left_curly(ctx, &body)
        {
            diagnostics.extend(self.verify_brace(ctx, &lcurly, node));
        }

        diagnostics
    }

    /// Verify brace placement according to the configured option.
    fn verify_brace(
        &self,
        ctx: &CheckContext,
        brace: &CstNode,
        start_token: &CstNode,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check for '{}' special case - single line empty block
        let is_empty_block = Self::is_empty_block(ctx, brace);
        if is_empty_block {
            return diagnostics;
        }

        match self.option {
            LeftCurlyOption::Nl => {
                // Brace must be on a new line (alone)
                if !Self::has_whitespace_before(ctx, brace) {
                    let mut diagnostic = Diagnostic::new(
                        LeftCurlyShouldBeOnNewLine {
                            column: Self::get_column(ctx, brace),
                        },
                        brace.range(),
                    );
                    if let Some(fix) = self.fix_move_to_new_line(ctx, brace, start_token) {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    diagnostics.push(diagnostic);
                }
            }
            LeftCurlyOption::Eol => {
                // Brace must be at end of line (on same line as statement)
                if Self::has_whitespace_before(ctx, brace) {
                    let mut diagnostic = Diagnostic::new(
                        LeftCurlyShouldBeOnPreviousLine {
                            column: Self::get_column(ctx, brace),
                        },
                        brace.range(),
                    );
                    if let Some(fix) = self.fix_move_to_previous_line(ctx, brace) {
                        diagnostic = diagnostic.with_fix(fix);
                    }
                    diagnostics.push(diagnostic);
                }
                // Check line break after - note this can report in addition to "previous line"
                if !self.has_line_break_after(ctx, brace) {
                    let diagnostic = Diagnostic::new(
                        LeftCurlyShouldHaveLineBreakAfter {
                            column: Self::get_column(ctx, brace),
                        },
                        brace.range(),
                    );
                    diagnostics.push(diagnostic);
                }
            }
            LeftCurlyOption::Nlow => {
                // NLOW: similar to EOL but more lenient
                // If not on same line as start, it should be on new line
                if !are_on_same_line(ctx, start_token, brace) {
                    // Not on same line - check if it's on next line
                    let source_code = ctx.source_code();
                    let start_line = source_code.line_column(start_token.range().start()).line;
                    let brace_line = source_code.line_column(brace.range().start()).line;

                    if brace_line.get() == start_line.get() + 1 {
                        // On next line - check if it has whitespace before (should be alone)
                        if !Self::has_whitespace_before(ctx, brace) {
                            let mut diagnostic = Diagnostic::new(
                                LeftCurlyShouldBeOnNewLine {
                                    column: Self::get_column(ctx, brace),
                                },
                                brace.range(),
                            );
                            if let Some(fix) = self.fix_move_to_new_line(ctx, brace, start_token) {
                                diagnostic = diagnostic.with_fix(fix);
                            }
                            diagnostics.push(diagnostic);
                        }
                    } else if !Self::has_whitespace_before(ctx, brace) {
                        // Multiple lines away and not alone on line
                        let mut diagnostic = Diagnostic::new(
                            LeftCurlyShouldBeOnNewLine {
                                column: Self::get_column(ctx, brace),
                            },
                            brace.range(),
                        );
                        if let Some(fix) = self.fix_move_to_new_line(ctx, brace, start_token) {
                            diagnostic = diagnostic.with_fix(fix);
                        }
                        diagnostics.push(diagnostic);
                    }
                } else {
                    // On same line - check for line break after (like EOL)
                    // For NLOW on same line, we don't check line break after
                }
            }
        }

        diagnostics
    }

    /// Check if a brace is part of an empty block ('{' followed immediately by '}').
    fn is_empty_block(ctx: &CheckContext, brace: &CstNode) -> bool {
        // Get the line content
        let line_index = ctx.line_index();
        let source_code = ctx.source_code();
        let brace_line = source_code.line_column(brace.range().start()).line;
        let line_start = line_index.line_start(brace_line, ctx.source());
        let line_end = line_index.line_end(brace_line, ctx.source());

        let line = &ctx.source()[usize::from(line_start)..usize::from(line_end)];
        let col = Self::get_column(ctx, brace) - 1; // 0-indexed

        // Check if there's a '}' immediately after the '{'
        if col + 1 < line.len() {
            line.chars().nth(col + 1) == Some('}')
        } else {
            false
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

    fn check_source_with_config(source: &str, rule: &LeftCurly) -> Vec<Diagnostic> {
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
    fn test_left_curly_eol_fix_moves_brace_with_block_comment() {
        let source = "class Foo\n{ /*start*/\n    void m() {}\n}\n";
        let diagnostics = check_source_with_config(source, &LeftCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for left curly EOL");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo { /*start*/\n    void m() {}\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_left_curly_nlow_fix_moves_brace_to_new_line() {
        let source = "class Foo\nextends Bar {\n    void m() {}\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nlow,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for left curly NLOW");
        let fixed = apply_edits(source, fix.edits());
        let expected = "class Foo\nextends Bar\n{\n    void m() {}\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_left_curly_enum_eol_fix_moves_brace_with_comment() {
        let source = "enum Foo\n{ /* enum */\n    A;\n}\n";
        let diagnostics = check_source_with_config(source, &LeftCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for left curly enum EOL");
        let fixed = apply_edits(source, fix.edits());
        let expected = "enum Foo { /* enum */\n    A;\n}\n";
        assert_eq!(fixed, expected);
    }

    #[test]
    fn test_left_curly_class_with_modifiers_and_annotation_eol_fix() {
        let source = "@Deprecated\npublic final class Foo\n{ /* body */\n    void m() {}\n}\n";
        let diagnostics = check_source_with_config(source, &LeftCurly::default());
        let fix = diagnostics
            .iter()
            .find_map(|d| d.fix.as_ref())
            .expect("Expected a fix for left curly class EOL");
        let fixed = apply_edits(source, fix.edits());
        let expected = "@Deprecated\npublic final class Foo { /* body */\n    void m() {}\n}\n";
        assert_eq!(fixed, expected);
    }

    // Tests for empty block patterns - these should NOT have fixes to avoid conflicts
    // with SingleSpaceSeparator

    #[test]
    fn test_left_curly_nl_empty_block_no_fix() {
        // For NL option, `{ }` pattern should report violation but NOT have a fix
        // because fixing it would conflict with SingleSpaceSeparator fixes
        let source = "class Foo\n{\n    private Foo() { }\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nl,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        // Should find a violation for the constructor's `{` (skip class brace at low offset)
        let ctor_diagnostic = diagnostics.iter().find(|d| d.range.start().to_u32() > 15);
        assert!(
            ctor_diagnostic.is_some(),
            "Expected violation for constructor brace"
        );
        // The diagnostic should NOT have a fix (to avoid conflict with SingleSpaceSeparator)
        assert!(
            ctor_diagnostic.unwrap().fix.is_none(),
            "Expected NO fix for empty block pattern to avoid conflicts"
        );
    }

    #[test]
    fn test_left_curly_nl_double_space_empty_block_no_fix() {
        // For NL option, `{  }` (double space) pattern should report violation but NOT have a fix
        let source = "class Foo\n{\n    private Foo() {  }\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nl,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let ctor_diagnostic = diagnostics.iter().find(|d| d.range.start().to_u32() > 15);
        assert!(
            ctor_diagnostic.is_some(),
            "Expected violation for constructor brace"
        );
        assert!(
            ctor_diagnostic.unwrap().fix.is_none(),
            "Expected NO fix for double-space empty block pattern"
        );
    }

    #[test]
    fn test_left_curly_nl_with_statement_no_fix() {
        // For NL option, `{ statement; }` should NOT have a fix
        // (complex content after brace)
        let source = "class Foo\n{\n    void m() { return; }\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nl,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let method_diagnostic = diagnostics.iter().find(|d| d.range.start().to_u32() > 15);
        assert!(
            method_diagnostic.is_some(),
            "Expected violation for method brace"
        );
        // Non-empty blocks with statements also don't have fixes (complex content)
        assert!(
            method_diagnostic.unwrap().fix.is_none(),
            "Expected NO fix for block with statement content"
        );
    }

    #[test]
    fn test_left_curly_nl_with_line_comment_has_fix() {
        // For NL option, `{ // comment` should have a fix (comments can be preserved)
        let source = "class Foo\n{\n    void m() { // todo\n        return;\n    }\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nl,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let method_diagnostic = diagnostics.iter().find(|d| d.range.start().to_u32() > 15);
        assert!(
            method_diagnostic.is_some(),
            "Expected violation for method brace with comment"
        );
        // Comments CAN be preserved, so this should have a fix
        assert!(
            method_diagnostic.unwrap().fix.is_some(),
            "Expected fix for brace followed by line comment"
        );
    }

    #[test]
    fn test_left_curly_nl_with_block_comment_has_fix() {
        // For NL option, `{ /* comment */` should have a fix
        let source = "class Foo\n{\n    void m() { /* start */\n        return;\n    }\n}\n";
        let rule = LeftCurly {
            option: LeftCurlyOption::Nl,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, &rule);
        let method_diagnostic = diagnostics.iter().find(|d| d.range.start().to_u32() > 15);
        assert!(
            method_diagnostic.is_some(),
            "Expected violation for method brace with block comment"
        );
        assert!(
            method_diagnostic.unwrap().fix.is_some(),
            "Expected fix for brace followed by block comment"
        );
    }
}
