//! EmptyLineSeparator rule implementation.
//!
//! Checks that class members are separated by empty lines.
//!
//! Checkstyle equivalent: EmptyLineSeparatorCheck

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: element should be separated from previous line.
#[derive(Debug, Clone)]
pub struct ShouldBeSeparated {
    pub element: String,
}

impl Violation for ShouldBeSeparated {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be separated from previous line.", self.element)
    }
}

/// Violation: element has too many empty lines before it.
#[derive(Debug, Clone)]
pub struct TooManyEmptyLines {
    pub element: String,
}

impl Violation for TooManyEmptyLines {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' has more than 1 empty lines before.", self.element)
    }
}

/// Violation: closing brace has too many empty lines after it.
#[derive(Debug, Clone)]
pub struct TooManyEmptyLinesAfter;

impl Violation for TooManyEmptyLinesAfter {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "'}' has more than 1 empty lines after.".to_string()
    }
}

/// Violation: too many empty lines inside a class member.
#[derive(Debug, Clone)]
pub struct TooManyEmptyLinesInside;

impl Violation for TooManyEmptyLinesInside {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        "There is more than 1 empty line after this line.".to_string()
    }
}

/// Violation: comment has too many empty lines before it.
#[derive(Debug, Clone)]
pub struct CommentTooManyEmptyLines {
    pub comment_start: String, // "//" or "/*"
}

impl Violation for CommentTooManyEmptyLines {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!(
            "'{}' has more than 1 empty lines before.",
            self.comment_start
        )
    }
}

/// Token types that can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmptyLineSeparatorToken {
    PackageDef,
    Import,
    StaticImport,
    ClassDef,
    InterfaceDef,
    EnumDef,
    StaticInit,
    InstanceInit,
    MethodDef,
    CtorDef,
    VariableDef,
    RecordDef,
    CompactCtorDef,
}

impl EmptyLineSeparatorToken {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "PACKAGE_DEF" => Some(Self::PackageDef),
            "IMPORT" => Some(Self::Import),
            "STATIC_IMPORT" => Some(Self::StaticImport),
            "CLASS_DEF" => Some(Self::ClassDef),
            "INTERFACE_DEF" => Some(Self::InterfaceDef),
            "ENUM_DEF" => Some(Self::EnumDef),
            "STATIC_INIT" => Some(Self::StaticInit),
            "INSTANCE_INIT" => Some(Self::InstanceInit),
            "METHOD_DEF" => Some(Self::MethodDef),
            "CTOR_DEF" => Some(Self::CtorDef),
            "VARIABLE_DEF" => Some(Self::VariableDef),
            "RECORD_DEF" => Some(Self::RecordDef),
            "COMPACT_CTOR_DEF" => Some(Self::CompactCtorDef),
            _ => None,
        }
    }

    fn to_checkstyle_name(self) -> &'static str {
        match self {
            Self::PackageDef => "PACKAGE_DEF",
            Self::Import => "IMPORT",
            Self::StaticImport => "STATIC_IMPORT",
            Self::ClassDef => "CLASS_DEF",
            Self::InterfaceDef => "INTERFACE_DEF",
            Self::EnumDef => "ENUM_DEF",
            Self::StaticInit => "STATIC_INIT",
            Self::InstanceInit => "INSTANCE_INIT",
            Self::MethodDef => "METHOD_DEF",
            Self::CtorDef => "CTOR_DEF",
            Self::VariableDef => "VARIABLE_DEF",
            Self::RecordDef => "RECORD_DEF",
            Self::CompactCtorDef => "COMPACT_CTOR_DEF",
        }
    }

    pub fn default_tokens() -> HashSet<Self> {
        [
            Self::PackageDef,
            Self::Import,
            Self::StaticImport,
            Self::ClassDef,
            Self::InterfaceDef,
            Self::EnumDef,
            Self::StaticInit,
            Self::InstanceInit,
            Self::MethodDef,
            Self::CtorDef,
            Self::VariableDef,
            Self::RecordDef,
            Self::CompactCtorDef,
        ]
        .into_iter()
        .collect()
    }
}

/// Configuration for EmptyLineSeparator rule.
#[derive(Debug, Clone)]
pub struct EmptyLineSeparator {
    pub allow_no_empty_line_between_fields: bool,
    pub allow_multiple_empty_lines: bool,
    pub allow_multiple_empty_lines_inside_class_members: bool,
    pub tokens: HashSet<EmptyLineSeparatorToken>,
}

const RELEVANT_KINDS: &[&str] = &[
    "program",
    "class_body",
    "interface_body",
    "enum_body",
    "annotation_type_body",
];

impl Default for EmptyLineSeparator {
    fn default() -> Self {
        Self {
            allow_no_empty_line_between_fields: false,
            allow_multiple_empty_lines: true,
            allow_multiple_empty_lines_inside_class_members: true,
            tokens: EmptyLineSeparatorToken::default_tokens(),
        }
    }
}

impl FromConfig for EmptyLineSeparator {
    const MODULE_NAME: &'static str = "EmptyLineSeparator";

    fn from_config(properties: &Properties) -> Self {
        let allow_no_empty_line_between_fields = properties
            .get("allowNoEmptyLineBetweenFields")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let allow_multiple_empty_lines = properties
            .get("allowMultipleEmptyLines")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let allow_multiple_empty_lines_inside_class_members = properties
            .get("allowMultipleEmptyLinesInsideClassMembers")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let tokens = properties
            .get("tokens")
            .map(|v| {
                v.split(',')
                    .filter_map(|s| EmptyLineSeparatorToken::from_str(s.trim()))
                    .collect()
            })
            .unwrap_or_else(EmptyLineSeparatorToken::default_tokens);

        Self {
            allow_no_empty_line_between_fields,
            allow_multiple_empty_lines,
            allow_multiple_empty_lines_inside_class_members,
            tokens,
        }
    }
}

impl Rule for EmptyLineSeparator {
    fn name(&self) -> &'static str {
        "EmptyLineSeparator"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();
        let source = ctx.source();

        // Handle file-level checks (program node)
        if kind == "program" {
            return self.check_program(node, source);
        }

        // Only process container bodies
        if kind != "class_body"
            && kind != "interface_body"
            && kind != "enum_body"
            && kind != "annotation_type_body"
        {
            return vec![];
        }

        let ts_node = node.inner();
        let mut diagnostics = vec![];

        let mut cursor = ts_node.walk();
        let raw_children: Vec<_> = ts_node.children(&mut cursor).collect();

        // Find the opening brace position for first-member check (before enum handling moves raw_children)
        let open_brace_line = raw_children
            .iter()
            .find(|c| c.kind() == "{")
            .map(|c| c.end_position().row);

        // For enum bodies, we need to handle the structure specially:
        // enum_body contains: {, enum_constant, ..., enum_body_declarations, }
        // enum_body_declarations contains: ;, field_declaration, method_declaration, etc.
        // We need to flatten the members from enum_body_declarations into our children list
        // and track the last enum_constant as the previous element for the first member.
        let (children, enum_const_end_line) = if kind == "enum_body" {
            let mut flattened = Vec::new();
            let mut last_enum_const_line = None;

            for child in &raw_children {
                if child.kind() == "enum_constant" {
                    last_enum_const_line = Some(child.end_position().row);
                    // Don't add enum_constant to children - they're not checked as tokens
                } else if child.kind() == "enum_body_declarations" {
                    // Flatten the children of enum_body_declarations
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() != ";" {
                            // Skip the semicolon separator
                            flattened.push(inner);
                        }
                    }
                } else {
                    flattened.push(*child);
                }
            }
            (flattened, last_enum_const_line)
        } else {
            (raw_children, None)
        };

        // Track previous CODE element end line (for "should be separated" checks)
        // Initialize with opening brace line (or enum_const_end_line for enums)
        let mut prev_code_end_line: Option<usize> = enum_const_end_line.or(open_brace_line);
        // Track previous ANYTHING end line (for "too many empty lines" checks on comments)
        let mut prev_anything_end_line: Option<usize> = enum_const_end_line.or(open_brace_line);
        // Track previous element byte positions for fix generation
        let mut prev_code_end_byte: Option<usize> = children
            .iter()
            .find(|c| c.kind() == "{")
            .map(|c| c.end_byte());
        let mut prev_anything_end_byte: Option<usize> = prev_code_end_byte;
        let mut prev_was_field = false;
        let mut is_first_member = enum_const_end_line.is_none(); // First member (after brace)
        let mut reported_violation_for_gap = false; // Track if we've reported for current gap
        // Track if this class body contains any fields (for checkstyle comment reporting behavior)
        let class_has_fields = children
            .iter()
            .any(|c| self.node_to_token(c.kind()) == Some(EmptyLineSeparatorToken::VariableDef));

        for child in &children {
            // Skip braces
            if child.kind() == "{" || child.kind() == "}" {
                continue;
            }

            // When allowMultipleEmptyLines=false, check comments for too many empty lines before
            let is_comment = child.is_extra()
                && (child.kind() == "line_comment" || child.kind() == "block_comment");

            if !self.allow_multiple_empty_lines && is_comment {
                // Skip trailing comments on same line as previous element
                if let Some(prev_line) = prev_anything_end_line
                    && child.start_position().row == prev_line
                {
                    continue;
                }

                // Check if the next code element requires separation. If it does, let the violation
                // be reported on the code element instead of the comment. This matches checkstyle's
                // behavior where violations are reported on METHOD_DEF, not on javadoc comments.
                let next_code_element = {
                    let child_idx = children.iter().position(|c| c.id() == child.id());
                    child_idx.and_then(|idx| {
                        children[idx + 1..]
                            .iter()
                            .find(|c| !c.is_extra() && c.kind() != "{" && c.kind() != "}")
                    })
                };

                // Determine if the next code element requires separation
                // When allowNoEmptyLineBetweenFields=false, always report on comment (checkstyle
                // reports on each element that has empty lines before it)
                // When allowNoEmptyLineBetweenFields=true, report on code element (methods) OR
                // on comment (fields in classes with mixed content)
                let next_requires_separation = if !self.allow_no_empty_line_between_fields {
                    // When fields require separation, always report on comments
                    // (checkstyle reports "too many empty lines" on whatever has them before it)
                    false
                } else if is_first_member {
                    false // First member doesn't require separation, report on comment
                } else if class_has_fields {
                    // When class has fields and allowNoEmptyLineBetweenFields=true,
                    // checkstyle reports on comments/javadocs
                    false
                } else if let Some(next) = next_code_element {
                    // Methods-only class with allowNoEmptyLineBetweenFields=true
                    // Report on METHOD_DEF (treat javadoc+method as unit)
                    let next_token = self.node_to_token(next.kind());
                    if let Some(token) = next_token
                        && self.tokens.contains(&token)
                    {
                        true // Code element requires separation
                    } else {
                        false // Token not in check list, no separation required
                    }
                } else {
                    false // No next code element
                };

                // Check if this comment has >1 empty lines before it
                let comment_line = child.start_position().row;
                if let Some(prev_line) = prev_anything_end_line
                    && comment_line.saturating_sub(prev_line + 1) > 1
                    && !next_requires_separation
                {
                    // Only report on comment if next element doesn't require separation
                    let comment_start = if child.kind() == "line_comment" {
                        "//"
                    } else {
                        "/*"
                    };
                    let start = TextSize::from(child.start_byte() as u32);
                    let end = TextSize::from(child.start_byte() as u32 + 1);
                    let mut diag = Diagnostic::new(
                        CommentTooManyEmptyLines {
                            comment_start: comment_start.to_string(),
                        },
                        TextRange::new(start, end),
                    );
                    // Add fix to delete excess blank lines
                    if let Some(prev_byte) = prev_anything_end_byte
                        && let Some(fix) = self.create_delete_excess_lines_fix(
                            source,
                            prev_byte,
                            child.start_byte(),
                        )
                    {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                    // Mark that we've reported for this gap
                    reported_violation_for_gap = true;
                }
                // Update prev_anything_end_line to track this comment (but NOT prev_code_end_line)
                prev_anything_end_line = Some(child.end_position().row);
                prev_anything_end_byte = Some(child.end_byte());
                continue;
            }

            // Skip other extra nodes
            if child.is_extra() {
                continue;
            }

            let token_type = self.node_to_token(child.kind());

            // Skip if this token type is not being checked
            // IMPORTANT: Don't track unchecked tokens as "previous" elements
            // Checkstyle only requires blank lines after elements that are IN the tokens set
            if let Some(token) = token_type
                && !self.tokens.contains(&token)
            {
                // Don't update prev_end_line - unchecked tokens don't require separation
                continue;
            }

            // Check for multiple empty lines before comments inside modifiers.
            if !self.allow_multiple_empty_lines {
                diagnostics.extend(self.check_modifier_comment_spacing(child, source));
            }

            // Check if blank line is needed
            if let Some(prev_code_line) = prev_code_end_line {
                // Check for field-to-field transition
                let is_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
                let field_to_field = prev_was_field && is_field;

                // Use prev_code_end_line for "should be separated" checks
                let has_blank = self.has_blank_line_before(&children, child, prev_code_line);

                // For first member after opening brace, only check "too many empty lines"
                // (not "should be separated")
                if is_first_member {
                    // Skip if we already reported on a comment for this gap
                    if !self.allow_multiple_empty_lines
                        && !reported_violation_for_gap
                        && let Some(token) = token_type
                    {
                        // Count empty lines between opening brace and first member,
                        // ignoring lines covered by leading comments.
                        let empty_lines =
                            self.count_empty_lines_before(&children, child, prev_code_line);
                        if empty_lines > 1 {
                            let start = TextSize::from(child.start_byte() as u32);
                            let end = TextSize::from(child.start_byte() as u32 + 1);
                            let mut diag = Diagnostic::new(
                                TooManyEmptyLines {
                                    element: token.to_checkstyle_name().to_string(),
                                },
                                TextRange::new(start, end),
                            );
                            // Add fix to delete excess blank lines
                            if let Some(prev_byte) = prev_code_end_byte
                                && let Some(fix) = self.create_delete_excess_lines_fix(
                                    source,
                                    prev_byte,
                                    child.start_byte(),
                                )
                            {
                                diag = diag.with_fix(fix);
                            }
                            diagnostics.push(diag);
                        }
                    }
                } else if !has_blank {
                    // Skip violation if field-to-field and allowed
                    if field_to_field && self.allow_no_empty_line_between_fields {
                        // OK, no violation
                    } else if let Some(token) = token_type {
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            ShouldBeSeparated {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to insert a blank line
                        if let Some(fix) =
                            self.create_insert_blank_line_fix(source, child.start_byte())
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                } else if !self.allow_multiple_empty_lines
                    && !reported_violation_for_gap
                    && let Some(token) = token_type
                {
                    // Check for multiple empty lines (use prev_code_end_line for consistency)
                    let empty_lines =
                        self.count_empty_lines_before(&children, child, prev_code_line);
                    if empty_lines > 1 {
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            TooManyEmptyLines {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to delete excess blank lines
                        if let Some(prev_byte) = prev_code_end_byte
                            && let Some(fix) = self.create_delete_excess_lines_fix(
                                source,
                                prev_byte,
                                child.start_byte(),
                            )
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                }
            }

            // Update both tracking variables
            prev_code_end_line = Some(child.end_position().row);
            prev_anything_end_line = Some(child.end_position().row);
            prev_code_end_byte = Some(child.end_byte());
            prev_anything_end_byte = Some(child.end_byte());
            prev_was_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
            is_first_member = false;
            // Reset the flag after processing a code element
            reported_violation_for_gap = false;
        }

        // Check for multiple empty lines before the closing brace (TooManyEmptyLinesAfter)
        // Use prev_anything_end_line to account for comments between last member and closing brace
        if !self.allow_multiple_empty_lines
            && let Some(_last_content_end) = prev_anything_end_line
            && let Some(close_brace) = children.iter().rev().find(|c| c.kind() == "}")
        {
            let close_brace_line = close_brace.start_position().row;
            // Count blank lines between last content (including comments) and closing brace
            if let Some(last_child) = children
                .iter()
                .rev()
                .find(|c| c.kind() != "{" && c.kind() != "}")
            {
                let empty_count =
                    close_brace_line.saturating_sub(last_child.end_position().row + 1);
                if empty_count > 1 {
                    let start = TextSize::from(last_child.end_byte() as u32 - 1);
                    let end = TextSize::from(last_child.end_byte() as u32);
                    let mut diag =
                        Diagnostic::new(TooManyEmptyLinesAfter, TextRange::new(start, end));
                    // Add fix to delete excess blank lines (keep 1 line before closing brace)
                    if let Some(fix) = self.create_delete_all_excess_lines_fix(
                        source,
                        last_child.end_byte(),
                        close_brace.start_byte(),
                    ) {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                }
            }
        }

        // Check for multiple empty lines inside class members (TooManyEmptyLinesInside)
        if !self.allow_multiple_empty_lines_inside_class_members {
            diagnostics.extend(self.check_inside_members(&children, source));
        }

        diagnostics
    }
}

impl EmptyLineSeparator {
    fn node_to_token(&self, kind: &str) -> Option<EmptyLineSeparatorToken> {
        match kind {
            "package_declaration" => Some(EmptyLineSeparatorToken::PackageDef),
            "import_declaration" => Some(EmptyLineSeparatorToken::Import),
            "class_declaration" => Some(EmptyLineSeparatorToken::ClassDef),
            "interface_declaration" => Some(EmptyLineSeparatorToken::InterfaceDef),
            "annotation_type_declaration" => Some(EmptyLineSeparatorToken::InterfaceDef),
            "enum_declaration" => Some(EmptyLineSeparatorToken::EnumDef),
            "static_initializer" => Some(EmptyLineSeparatorToken::StaticInit),
            "block" => Some(EmptyLineSeparatorToken::InstanceInit), // instance init block
            "method_declaration" => Some(EmptyLineSeparatorToken::MethodDef),
            "constructor_declaration" => Some(EmptyLineSeparatorToken::CtorDef),
            "field_declaration" => Some(EmptyLineSeparatorToken::VariableDef),
            "constant_declaration" => Some(EmptyLineSeparatorToken::VariableDef), // interface fields
            "record_declaration" => Some(EmptyLineSeparatorToken::RecordDef),
            "compact_constructor_declaration" => Some(EmptyLineSeparatorToken::CompactCtorDef),
            _ => None,
        }
    }

    /// Check for multiple consecutive empty lines inside method/ctor/init bodies.
    /// Only checks members whose token type is in the configured tokens set.
    fn check_inside_members(
        &self,
        children: &[tree_sitter::Node],
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        for child in children {
            // Check inside method bodies, constructor bodies, static initializers, instance initializers
            // Only if the corresponding token is being checked
            match child.kind() {
                "method_declaration" => {
                    if self.tokens.contains(&EmptyLineSeparatorToken::MethodDef)
                        && let Some(body) = child.child_by_field_name("body")
                    {
                        diagnostics
                            .extend(self.check_block_for_multiple_empty_lines(&body, source));
                        // Also check nested array initializers
                        diagnostics.extend(self.find_and_check_array_initializers(&body, source));
                    }
                }
                "constructor_declaration" => {
                    if self.tokens.contains(&EmptyLineSeparatorToken::CtorDef)
                        && let Some(body) = child.child_by_field_name("body")
                    {
                        diagnostics
                            .extend(self.check_block_for_multiple_empty_lines(&body, source));
                        // Also check nested array initializers
                        diagnostics.extend(self.find_and_check_array_initializers(&body, source));
                    }
                }
                "static_initializer" => {
                    if self.tokens.contains(&EmptyLineSeparatorToken::StaticInit) {
                        let mut cursor = child.walk();
                        for inner in child.children(&mut cursor) {
                            if inner.kind() == "block" {
                                diagnostics.extend(
                                    self.check_block_for_multiple_empty_lines(&inner, source),
                                );
                                // Also check nested array initializers
                                diagnostics
                                    .extend(self.find_and_check_array_initializers(&inner, source));
                                break;
                            }
                        }
                    }
                }
                "block" => {
                    // This is an instance initializer - check the block itself
                    if self.tokens.contains(&EmptyLineSeparatorToken::InstanceInit) {
                        diagnostics
                            .extend(self.check_block_for_multiple_empty_lines(child, source));
                        // Also check nested array initializers
                        diagnostics.extend(self.find_and_check_array_initializers(child, source));
                    }
                }
                "compact_constructor_declaration" => {
                    // Compact constructor in records (Java 16+)
                    if self
                        .tokens
                        .contains(&EmptyLineSeparatorToken::CompactCtorDef)
                    {
                        let mut cursor = child.walk();
                        for inner in child.children(&mut cursor) {
                            if inner.kind() == "block" {
                                diagnostics.extend(
                                    self.check_block_for_multiple_empty_lines(&inner, source),
                                );
                                diagnostics
                                    .extend(self.find_and_check_array_initializers(&inner, source));
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }

    /// Check a block (or constructor_body or array_initializer) for consecutive empty lines.
    fn check_block_for_multiple_empty_lines(
        &self,
        block: &tree_sitter::Node,
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Handle "block", "constructor_body", and "array_initializer" node types
        if block.kind() != "block"
            && block.kind() != "constructor_body"
            && block.kind() != "array_initializer"
        {
            return diagnostics;
        }

        // Get all the lines in the block
        let start_line = block.start_position().row;
        let end_line = block.end_position().row;

        if end_line <= start_line + 1 {
            return diagnostics;
        }

        // Collect all lines that have content (nodes or comments)
        let mut content_lines = std::collections::HashSet::new();
        let mut comment_lines = std::collections::HashSet::new();
        let mut code_lines = std::collections::HashSet::new(); // Lines with actual code (not just comments)
        let mut non_brace_code_lines = std::collections::HashSet::new(); // Lines with non-brace code
        let mut brace_only_lines = std::collections::HashSet::new(); // Lines with only braces
        // Collect nested class body regions to skip (they'll be checked separately)
        let mut nested_block_regions: Vec<(usize, usize)> = vec![];
        content_lines.insert(start_line); // Opening brace line
        content_lines.insert(end_line); // Closing brace line
        brace_only_lines.insert(start_line); // Opening brace
        brace_only_lines.insert(end_line); // Closing brace

        // Recursively collect all node positions, tracking comments and code separately
        // Only mark lines for LEAF nodes (nodes with no non-extra children)
        // Container nodes span across empty lines without having content on them
        fn collect_child_lines(
            node: &tree_sitter::Node,
            content_lines: &mut std::collections::HashSet<usize>,
            comment_lines: &mut std::collections::HashSet<usize>,
            code_lines: &mut std::collections::HashSet<usize>,
            non_brace_code_lines: &mut std::collections::HashSet<usize>,
            brace_only_lines: &mut std::collections::HashSet<usize>,
            nested_block_regions: &mut Vec<(usize, usize)>,
        ) {
            // Check for nested class/interface/enum bodies - these will be checked separately
            // Don't recurse into them, but mark their entire span as "content" so we don't
            // report false positives for empty lines inside them
            if node.kind() == "class_body"
                || node.kind() == "interface_body"
                || node.kind() == "enum_body"
                || node.kind() == "annotation_type_body"
            {
                // Mark the entire nested body span as content
                for row in node.start_position().row..=node.end_position().row {
                    content_lines.insert(row);
                    code_lines.insert(row);
                }
                // Track this region so we know to skip empty line checks inside it
                nested_block_regions.push((node.start_position().row, node.end_position().row));
                return; // Don't recurse into nested class bodies
            }

            // Check for array initializers - these are checked separately via find_and_check_array_initializers
            // Don't recurse into them from the method body level
            if node.kind() == "array_initializer" {
                // Mark the entire array initializer span as content
                for row in node.start_position().row..=node.end_position().row {
                    content_lines.insert(row);
                    code_lines.insert(row);
                }
                // Track this region so we know to skip empty line checks inside it
                nested_block_regions.push((node.start_position().row, node.end_position().row));
                return; // Don't recurse - array initializers are checked separately
            }

            let is_comment =
                node.kind() == "line_comment" || node.kind() == "block_comment" || node.is_extra();
            let is_brace = node.kind() == "{" || node.kind() == "}";

            // Check if this node has non-extra children
            let mut cursor = node.walk();
            let children: Vec<_> = node.children(&mut cursor).collect();
            let has_non_extra_children = children.iter().any(|c| !c.is_extra());

            // Only mark lines for leaf nodes or comments
            // Container nodes should have their lines marked by their children
            if !has_non_extra_children || is_comment {
                for row in node.start_position().row..=node.end_position().row {
                    content_lines.insert(row);
                    if is_comment {
                        comment_lines.insert(row);
                    } else {
                        code_lines.insert(row);
                        if !is_brace {
                            non_brace_code_lines.insert(row);
                            brace_only_lines.remove(&row);
                        }
                    }
                }
                // If it's a brace on its own line, add to brace_only_lines
                if is_brace {
                    let row = node.start_position().row;
                    // Only mark as brace_only if no non-brace code is on this line
                    if !non_brace_code_lines.contains(&row) {
                        brace_only_lines.insert(row);
                    }
                }
            }

            // Recurse into children
            for child in children {
                collect_child_lines(
                    &child,
                    content_lines,
                    comment_lines,
                    code_lines,
                    non_brace_code_lines,
                    brace_only_lines,
                    nested_block_regions,
                );
            }
        }

        // Collect lines for all direct children of the block (not the block itself)
        let mut cursor = block.walk();
        for child in block.children(&mut cursor) {
            collect_child_lines(
                &child,
                &mut content_lines,
                &mut comment_lines,
                &mut code_lines,
                &mut non_brace_code_lines,
                &mut brace_only_lines,
                &mut nested_block_regions,
            );
        }

        // Helper to check if a line is inside a nested block region
        let is_inside_nested_block = |line: usize| -> bool {
            nested_block_regions
                .iter()
                .any(|(start, end)| line > *start && line < *end)
        };

        // Find consecutive empty lines
        let mut consecutive_empty = 0;
        let mut last_content_line = start_line;
        let mut last_code_line = start_line; // Track last CODE line separately (not comments)
        // Track if we've already reported a violation for the current gap
        let mut reported_for_current_gap = false;
        // Track if we had a gap and need to report on the next code element
        // Only applies when there was no comment in the gap
        let mut need_report_on_next_code = false;
        // Track if we hit a comment since the last code line
        let mut had_comment_in_gap = false;
        // Track if last code line is a closing brace (comments after braces are standalone)
        let mut last_code_is_close_brace = true; // Start as true (opening brace is like close)

        for line in (start_line + 1)..end_line {
            // Skip lines inside nested class bodies - they'll be checked separately
            if is_inside_nested_block(line) {
                // Reset tracking when entering/exiting nested blocks
                consecutive_empty = 0;
                reported_for_current_gap = false;
                need_report_on_next_code = false;
                had_comment_in_gap = false;
                continue;
            }

            if !content_lines.contains(&line) {
                // This is an empty line
                consecutive_empty += 1;

                // Report violation once when we find more than 1 consecutive empty line
                if consecutive_empty > 1 && !reported_for_current_gap {
                    // Determine which line to report on:
                    // If comment immediately follows real code (not just braces), report on code
                    //   (checkstyle treats such comments as attached to the preceding statement)
                    // Otherwise (comment after brace or standalone), report on the comment
                    let report_line = if comment_lines.contains(&last_content_line)
                        && last_code_line + 1 == last_content_line
                        && last_code_line != start_line
                        && !last_code_is_close_brace
                    {
                        // Comment immediately follows code (not brace) - report on code
                        last_code_line
                    } else {
                        // Comment is standalone, after brace, or last content is code
                        last_content_line
                    };

                    let byte_offset = self.find_line_end_byte(block, report_line);
                    let start = TextSize::from(byte_offset as u32);
                    let end = TextSize::from(byte_offset as u32 + 1);
                    let mut diag =
                        Diagnostic::new(TooManyEmptyLinesInside, TextRange::new(start, end));
                    // Add fix to delete excess blank lines
                    // Find the actual next content line (not just line + 1, which might be empty)
                    let next_content_line = (line + 1..=end_line)
                        .find(|l| content_lines.contains(l))
                        .unwrap_or(end_line);
                    let next_content_byte = self.find_line_start_byte(block, next_content_line);
                    if let Some(fix) =
                        self.create_delete_excess_lines_fix(source, byte_offset, next_content_byte)
                    {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                    // Mark that we've reported for this gap - don't report again until we hit content
                    reported_for_current_gap = true;
                    // For array initializers, also report on the element after the gap,
                    // but ONLY if there was no comment between code elements
                    if block.kind() == "array_initializer" && !had_comment_in_gap {
                        need_report_on_next_code = true;
                    }
                }
            } else {
                // Hit content
                let is_code = code_lines.contains(&line);
                let is_comment = comment_lines.contains(&line);

                // Check if we need to report on this code element
                if need_report_on_next_code && is_code {
                    // Report on this code element for having too many empty lines BEFORE it
                    let byte_offset = self.find_line_start_byte(block, line);
                    let start = TextSize::from(byte_offset as u32);
                    let end = TextSize::from(byte_offset as u32 + 1);
                    // Note: Fix was already added to the previous violation
                    diagnostics.push(Diagnostic::new(
                        TooManyEmptyLinesInside,
                        TextRange::new(start, end),
                    ));
                }

                // Track if we hit a comment (for the next gap check)
                if is_comment {
                    had_comment_in_gap = true;
                }

                // Reset tracking for next potential gap
                last_content_line = line;
                // Only update last_code_line for actual code, not comments
                if is_code {
                    last_code_line = line;
                    // Track if this code line is a brace-only line (closing brace)
                    last_code_is_close_brace = brace_only_lines.contains(&line);
                    // Reset comment tracking when we hit code
                    had_comment_in_gap = false;
                }
                consecutive_empty = 0;
                reported_for_current_gap = false;
                need_report_on_next_code = false;
            }
        }

        diagnostics
    }

    /// Find the byte position of the end of a line within a block.
    fn find_line_end_byte(&self, block: &tree_sitter::Node, target_line: usize) -> usize {
        let mut best_byte = block.start_byte();

        fn find_in_node(node: &tree_sitter::Node, target_line: usize, best: &mut usize) {
            if node.end_position().row == target_line {
                *best = (*best).max(node.end_byte());
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_in_node(&child, target_line, best);
            }
        }

        find_in_node(block, target_line, &mut best_byte);
        best_byte
    }

    /// Find the byte position of the start of content on a line within a block.
    fn find_line_start_byte(&self, block: &tree_sitter::Node, target_line: usize) -> usize {
        let mut best_byte = block.end_byte();

        fn find_in_node(node: &tree_sitter::Node, target_line: usize, best: &mut usize) {
            if node.start_position().row == target_line {
                *best = (*best).min(node.start_byte());
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_in_node(&child, target_line, best);
            }
        }

        find_in_node(block, target_line, &mut best_byte);
        best_byte
    }

    /// Recursively find all array_initializer nodes and check them for consecutive empty lines.
    fn find_and_check_array_initializers(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check if this node is an array_initializer
        if node.kind() == "array_initializer" {
            diagnostics.extend(self.check_block_for_multiple_empty_lines(node, source));
        }

        // Recursively check children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            diagnostics.extend(self.find_and_check_array_initializers(&child, source));
        }

        diagnostics
    }

    /// Check if there's at least one blank line between prev_end_line and the target
    /// (including the region occupied by comments before the target).
    ///
    /// Checkstyle considers a blank line ANYWHERE in the comment block before an element
    /// to satisfy the separation requirement.
    fn has_blank_line_before(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
        prev_end_line: usize,
    ) -> bool {
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Collect all rows occupied by comments between prev element and target
            let mut comment_rows = std::collections::HashSet::new();
            for i in (0..idx).rev() {
                let child = &children[i];
                if child.kind() == "{" || child.kind() == "}" {
                    continue;
                }
                if child.kind() != "line_comment" && child.kind() != "block_comment" {
                    if !child.is_extra() {
                        break;
                    }
                    continue;
                }
                // Skip trailing comments on same line as prev element
                if child.start_position().row == prev_end_line {
                    continue;
                }
                // Add all rows this comment occupies
                for row in child.start_position().row..=child.end_position().row {
                    comment_rows.insert(row);
                }
            }

            // Check each row between prev_end_line and target.start_position()
            let target_start = target.start_position().row;
            for row in (prev_end_line + 1)..target_start {
                if !comment_rows.contains(&row) {
                    // This row is not a comment, so it must be blank
                    return true;
                }
            }

            false
        } else {
            false
        }
    }

    /// Check for multiple empty lines before comments inside a modifiers block.
    fn check_modifier_comment_spacing(
        &self,
        node: &tree_sitter::Node,
        source: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let modifiers = node.child_by_field_name("modifiers").or_else(|| {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .find(|child| child.kind() == "modifiers")
        });

        let Some(modifiers) = modifiers else {
            return diagnostics;
        };

        let mut cursor = modifiers.walk();
        let children: Vec<_> = modifiers.children(&mut cursor).collect();
        let mut prev_end_line: Option<usize> = None;
        let mut prev_end_byte: Option<usize> = None;

        for child in children {
            let is_comment = child.kind() == "line_comment" || child.kind() == "block_comment";
            if is_comment {
                if let Some(_prev_line) = prev_end_line {
                    let empty_lines = child.start_position().row.saturating_sub(_prev_line + 1);
                    if empty_lines > 1 {
                        let comment_start = if child.kind() == "line_comment" {
                            "//"
                        } else {
                            "/*"
                        };
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            CommentTooManyEmptyLines {
                                comment_start: comment_start.to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to delete excess blank lines
                        if let Some(prev_byte) = prev_end_byte
                            && let Some(fix) = self.create_delete_excess_lines_fix(
                                source,
                                prev_byte,
                                child.start_byte(),
                            )
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                }
                prev_end_line = Some(child.end_position().row);
                prev_end_byte = Some(child.end_byte());
            } else if !child.is_extra() {
                prev_end_line = Some(child.end_position().row);
                prev_end_byte = Some(child.end_byte());
            }
        }

        diagnostics
    }

    /// Count the number of empty lines between prev_end_line and the target.
    /// This uses the max of empty lines BEFORE the leading comment block and AFTER
    /// the last comment, but NOT empty lines BETWEEN comments.
    fn count_empty_lines_before(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
        prev_end_line: usize,
    ) -> usize {
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Find the first and last comment in the leading comment block (walking backwards)
            let mut first_comment_start: Option<usize> = None;
            let mut last_comment_end: Option<usize> = None;
            for i in (0..idx).rev() {
                let child = &children[i];
                if child.kind() == "{" || child.kind() == "}" {
                    continue;
                }
                if child.kind() != "line_comment" && child.kind() != "block_comment" {
                    if !child.is_extra() {
                        break;
                    }
                    continue;
                }
                // Skip trailing comments on same line as prev element
                if child.start_position().row == prev_end_line {
                    continue;
                }
                // Track the earliest comment start (since we're walking backwards)
                first_comment_start = Some(child.start_position().row);
                // Track the latest comment end (first one we encounter when walking backwards)
                if last_comment_end.is_none() {
                    last_comment_end = Some(child.end_position().row);
                }
            }

            // If no comments, count all empty lines.
            if first_comment_start.is_none() {
                let mut count = 0;
                for _ in (prev_end_line + 1)..target.start_position().row {
                    count += 1;
                }
                return count;
            }

            // Count empty lines BEFORE first comment
            let mut before_count = 0;
            if let Some(first_start) = first_comment_start {
                for _ in (prev_end_line + 1)..first_start {
                    before_count += 1;
                }
            }

            // Count empty lines AFTER last comment
            let mut after_count = 0;
            if let Some(last_end) = last_comment_end {
                for _ in (last_end + 1)..target.start_position().row {
                    after_count += 1;
                }
            }

            before_count.max(after_count)
        } else {
            0
        }
    }

    /// Check file-level separation (package, imports, type declarations).
    fn check_program(&self, node: &CstNode, source: &str) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let mut diagnostics = vec![];

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node.children(&mut cursor).collect();

        // Find the end of file-level header comments (block comment at start of file)
        let mut file_header_end_line: Option<usize> = None;
        let mut file_header_end_byte: Option<usize> = None;
        for child in &children {
            if child.kind() == "block_comment" && child.start_position().row == 0 {
                file_header_end_line = Some(child.end_position().row);
                file_header_end_byte = Some(child.end_byte());
            } else if !child.is_extra() {
                break;
            }
        }

        // Track previous CODE element end line (for "should be separated" checks)
        let mut prev_code_end_line: Option<usize> = file_header_end_line;
        let mut prev_code_start_line: Option<usize> = None;
        let mut prev_code_end_byte: Option<usize> = file_header_end_byte;
        // Track previous ANYTHING end line (for "too many empty lines" checks on comments)
        let mut prev_anything_end_line: Option<usize> = file_header_end_line;
        let mut prev_anything_end_byte: Option<usize> = file_header_end_byte;
        let mut prev_token: Option<EmptyLineSeparatorToken> = None;
        let mut is_first_code_element = true;
        let mut reported_violation_for_gap = false; // Track if we've reported for current gap
        let mut had_trailing_comment_after_package = false;

        for child in &children {
            let is_comment = child.is_extra()
                && (child.kind() == "line_comment" || child.kind() == "block_comment");

            // Check comments for separation from package
            if is_comment {
                // Skip trailing comments on same line as previous element
                // But update prev_anything_end_line so subsequent elements are compared against this
                if let Some(prev_line) = prev_anything_end_line
                    && child.start_position().row == prev_line
                {
                    // Trailing comment - update tracking but don't process for violations
                    if prev_token == Some(EmptyLineSeparatorToken::PackageDef) {
                        had_trailing_comment_after_package = true;
                    }
                    prev_anything_end_line = Some(child.end_position().row);
                    continue;
                }

                let comment_line = child.start_position().row;

                // Comments on a new line after package (with no intervening trailing comment on a
                // different line) need blank line separation
                let skip_due_to_trailing_block_comment =
                    had_trailing_comment_after_package && child.kind() == "block_comment";
                if prev_token == Some(EmptyLineSeparatorToken::PackageDef)
                    && !skip_due_to_trailing_block_comment
                    && let Some(prev_line) = prev_code_end_line
                    && prev_anything_end_line == prev_code_end_line
                {
                    // No trailing comment after package on different line - check separation
                    let has_blank = comment_line > prev_line + 1;
                    if !has_blank {
                        let comment_start = if child.kind() == "line_comment" {
                            "//"
                        } else {
                            "/*"
                        };
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            ShouldBeSeparated {
                                element: comment_start.to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to insert a blank line
                        if let Some(fix) =
                            self.create_insert_blank_line_fix(source, child.start_byte())
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                }

                // When allowMultipleEmptyLines=false, check comments for too many empty lines before
                if !self.allow_multiple_empty_lines
                    && let Some(_prev_line) = prev_anything_end_line
                    && comment_line.saturating_sub(_prev_line + 1) > 1
                {
                    let comment_start = if child.kind() == "line_comment" {
                        "//"
                    } else {
                        "/*"
                    };
                    let start = TextSize::from(child.start_byte() as u32);
                    let end = TextSize::from(child.start_byte() as u32 + 1);
                    let mut diag = Diagnostic::new(
                        CommentTooManyEmptyLines {
                            comment_start: comment_start.to_string(),
                        },
                        TextRange::new(start, end),
                    );
                    // Add fix to delete excess blank lines
                    if let Some(prev_byte) = prev_anything_end_byte
                        && let Some(fix) = self.create_delete_excess_lines_fix(
                            source,
                            prev_byte,
                            child.start_byte(),
                        )
                    {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                    // Mark that we've reported for this gap
                    reported_violation_for_gap = true;
                }

                // Update prev_anything_end_line to track this comment (but NOT prev_code_end_line)
                prev_anything_end_line = Some(child.end_position().row);
                prev_anything_end_byte = Some(child.end_byte());
                // Reset trailing-comment flag once we hit a new-line comment
                had_trailing_comment_after_package = false;
                continue;
            }

            // Skip other extra nodes
            if child.is_extra() {
                continue;
            }

            let token_type = self.node_to_token(child.kind());

            // Determine if this is a static import
            let is_static_import = child.kind() == "import_declaration"
                && child.child_by_field_name("name").is_none()
                && child
                    .children(&mut child.walk())
                    .any(|c| c.kind() == "static");

            // Map to the appropriate token type
            let effective_token = if is_static_import {
                Some(EmptyLineSeparatorToken::StaticImport)
            } else {
                token_type
            };

            // Skip if this token type is not being checked
            if let Some(token) = effective_token {
                if !self.tokens.contains(&token) {
                    continue;
                }
            } else {
                continue;
            }

            let token = effective_token.unwrap();

            // Check for multiple empty lines before comments inside modifiers.
            if !self.allow_multiple_empty_lines {
                diagnostics.extend(self.check_modifier_comment_spacing(child, source));
            }

            // For first code element after file header, check for separation and multiple empty lines
            // Skip if we already reported on a comment for this gap
            if is_first_code_element
                && let Some(header_end) = file_header_end_line
                && prev_code_end_line == file_header_end_line
            {
                let element_line = child.start_position().row;

                // Check separation from either:
                // 1. The file header (if no comment between header and package)
                // 2. The immediately preceding comment (if there's one after the header)
                let separation_from = if let Some(prev_anything) = prev_anything_end_line
                    && prev_anything > header_end
                {
                    // There's a comment after the header - check separation from that comment
                    prev_anything
                } else {
                    // No intervening comment - check separation from file header
                    header_end
                };

                let has_blank = element_line > separation_from + 1;

                // Check for "should be separated" - package after file header/comment needs a blank line
                if !has_blank && token == EmptyLineSeparatorToken::PackageDef {
                    let start = TextSize::from(child.start_byte() as u32);
                    let end = TextSize::from(child.start_byte() as u32 + 1);
                    let mut diag = Diagnostic::new(
                        ShouldBeSeparated {
                            element: token.to_checkstyle_name().to_string(),
                        },
                        TextRange::new(start, end),
                    );
                    // Add fix to insert a blank line
                    if let Some(fix) = self.create_insert_blank_line_fix(source, child.start_byte())
                    {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                } else if !self.allow_multiple_empty_lines && !reported_violation_for_gap {
                    // Check for "too many empty lines"
                    let empty_lines = element_line.saturating_sub(header_end + 1);
                    if empty_lines > 1 {
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            TooManyEmptyLines {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to delete excess blank lines
                        if let Some(header_byte) = file_header_end_byte
                            && let Some(fix) = self.create_delete_excess_lines_fix(
                                source,
                                header_byte,
                                child.start_byte(),
                            )
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                }
            }
            is_first_code_element = false;

            // Check if blank line is needed
            if let Some(prev_code_line) = prev_code_end_line {
                let has_blank =
                    self.has_blank_line_before_program(&children, child, prev_code_line);

                // Determine if separation is required
                let same_line_as_prev = child.start_position().row == prev_code_line;
                let prev_spans_multiple_lines = prev_code_start_line
                    .map(|start| start < prev_code_line)
                    .unwrap_or(false);

                let needs_separation = match (prev_token, token) {
                    // Package spanning multiple lines: ignore same-line import separation.
                    (
                        Some(EmptyLineSeparatorToken::PackageDef),
                        EmptyLineSeparatorToken::Import | EmptyLineSeparatorToken::StaticImport,
                    ) if prev_spans_multiple_lines && same_line_as_prev => false,
                    // Package → anything needs separation
                    (Some(EmptyLineSeparatorToken::PackageDef), _) => true,
                    // Import → type declaration needs separation
                    (Some(EmptyLineSeparatorToken::Import), t)
                    | (Some(EmptyLineSeparatorToken::StaticImport), t)
                        if matches!(
                            t,
                            EmptyLineSeparatorToken::ClassDef
                                | EmptyLineSeparatorToken::InterfaceDef
                                | EmptyLineSeparatorToken::EnumDef
                                | EmptyLineSeparatorToken::RecordDef
                        ) =>
                    {
                        true
                    }
                    // All imports (regular and static) can be consecutive without blank lines
                    (Some(EmptyLineSeparatorToken::Import), EmptyLineSeparatorToken::Import) => {
                        false
                    }
                    (
                        Some(EmptyLineSeparatorToken::StaticImport),
                        EmptyLineSeparatorToken::StaticImport,
                    ) => false,
                    (
                        Some(EmptyLineSeparatorToken::StaticImport),
                        EmptyLineSeparatorToken::Import,
                    ) => false,
                    (
                        Some(EmptyLineSeparatorToken::Import),
                        EmptyLineSeparatorToken::StaticImport,
                    ) => false,
                    // Type → type needs separation
                    (Some(prev), curr)
                        if matches!(
                            prev,
                            EmptyLineSeparatorToken::ClassDef
                                | EmptyLineSeparatorToken::InterfaceDef
                                | EmptyLineSeparatorToken::EnumDef
                                | EmptyLineSeparatorToken::RecordDef
                        ) && matches!(
                            curr,
                            EmptyLineSeparatorToken::ClassDef
                                | EmptyLineSeparatorToken::InterfaceDef
                                | EmptyLineSeparatorToken::EnumDef
                                | EmptyLineSeparatorToken::RecordDef
                        ) =>
                    {
                        true
                    }
                    _ => false,
                };

                if needs_separation && !has_blank {
                    let start = TextSize::from(child.start_byte() as u32);
                    let end = TextSize::from(child.start_byte() as u32 + 1);
                    let mut diag = Diagnostic::new(
                        ShouldBeSeparated {
                            element: token.to_checkstyle_name().to_string(),
                        },
                        TextRange::new(start, end),
                    );
                    // Add fix to insert a blank line
                    if let Some(fix) = self.create_insert_blank_line_fix(source, child.start_byte())
                    {
                        diag = diag.with_fix(fix);
                    }
                    diagnostics.push(diag);
                } else if has_blank && !self.allow_multiple_empty_lines && prev_token.is_some() {
                    // Check for multiple empty lines (both when needs_separation and when not)
                    // At program level, we check each element independently
                    let empty_lines =
                        self.count_empty_lines_before_program(&children, child, prev_code_line);
                    if empty_lines > 1 {
                        let start = TextSize::from(child.start_byte() as u32);
                        let end = TextSize::from(child.start_byte() as u32 + 1);
                        let mut diag = Diagnostic::new(
                            TooManyEmptyLines {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            TextRange::new(start, end),
                        );
                        // Add fix to delete excess blank lines
                        if let Some(prev_byte) = prev_code_end_byte
                            && let Some(fix) = self.create_delete_excess_lines_fix(
                                source,
                                prev_byte,
                                child.start_byte(),
                            )
                        {
                            diag = diag.with_fix(fix);
                        }
                        diagnostics.push(diag);
                    }
                }
            }

            // Update both tracking variables
            prev_code_start_line = Some(child.start_position().row);
            prev_code_end_line = Some(child.end_position().row);
            prev_code_end_byte = Some(child.end_byte());
            prev_anything_end_line = Some(child.end_position().row);
            prev_anything_end_byte = Some(child.end_byte());
            prev_token = Some(token);
            had_trailing_comment_after_package = false;
            // Reset the flag after processing a code element
            reported_violation_for_gap = false;
        }

        diagnostics
    }

    /// Check if there's at least one blank line between prev_end_line and the target (program level).
    fn has_blank_line_before_program(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
        prev_end_line: usize,
    ) -> bool {
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Collect all rows occupied by comments between prev element and target
            let mut comment_rows = std::collections::HashSet::new();
            for i in (0..idx).rev() {
                let child = &children[i];
                if !child.is_extra() {
                    break;
                }
                if child.kind() != "line_comment" && child.kind() != "block_comment" {
                    continue;
                }
                // Skip trailing comments on same line as prev element
                if child.start_position().row == prev_end_line {
                    continue;
                }
                // Add all rows this comment occupies
                for row in child.start_position().row..=child.end_position().row {
                    comment_rows.insert(row);
                }
            }

            // Check each row between prev_end_line and target.start_position()
            let target_start = target.start_position().row;
            for row in (prev_end_line + 1)..target_start {
                if !comment_rows.contains(&row) {
                    // This row is not a comment, so it must be blank
                    return true;
                }
            }

            false
        } else {
            false
        }
    }

    /// Count the number of empty lines between prev_end_line and the target (program level).
    /// For code elements, only count empty lines AFTER the last comment in the leading block.
    /// Empty lines BEFORE comments are handled by comment violations separately.
    fn count_empty_lines_before_program(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
        prev_end_line: usize,
    ) -> usize {
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Find the last comment in the leading comment block (walking backwards)
            let mut last_comment_end: Option<usize> = None;
            for i in (0..idx).rev() {
                let child = &children[i];
                if !child.is_extra() {
                    break;
                }
                if child.kind() != "line_comment" && child.kind() != "block_comment" {
                    continue;
                }
                // Skip trailing comments on same line as prev element
                if child.start_position().row == prev_end_line {
                    continue;
                }
                // Track the latest comment end (first one we encounter when walking backwards)
                if last_comment_end.is_none() {
                    last_comment_end = Some(child.end_position().row);
                }
            }

            // Count empty lines AFTER last comment (before target)
            // If there are comments, only count empty lines between last comment and target
            // If no comments, count all empty lines from prev_end_line to target
            let boundary = last_comment_end.unwrap_or(prev_end_line);
            let mut count = 0;
            for _ in (boundary + 1)..target.start_position().row {
                count += 1;
            }

            count
        } else {
            0
        }
    }

    /// Create a fix that inserts a blank line before an element.
    /// `prev_line_end_byte` is the byte position at the end of the previous line's content.
    fn create_insert_blank_line_fix(&self, source: &str, element_start_byte: usize) -> Option<Fix> {
        // Find the start of the line containing the element
        let line_start = source[..element_start_byte]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        // Insert a newline at the start of this line (which adds a blank line before)
        let insert_pos = TextSize::new(line_start as u32);
        Some(Fix::safe_edit(Edit::insertion(
            "\n".to_string(),
            insert_pos,
        )))
    }

    /// Create a fix that deletes excess blank lines, keeping exactly one.
    /// `content_end_byte` is the byte position after the last content before the gap.
    /// `next_content_start_byte` is the byte position of the content after the gap.
    fn create_delete_excess_lines_fix(
        &self,
        source: &str,
        content_end_byte: usize,
        next_content_start_byte: usize,
    ) -> Option<Fix> {
        // Find where we need to delete from and to
        // We want to keep: content + \n + \n (one blank line) + next_content
        // So we delete from content_end_byte + 2 newlines to the start of next_content's line

        // Find the end of the line after content (first \n after content_end_byte)
        let first_newline = source[content_end_byte..]
            .find('\n')
            .map(|pos| content_end_byte + pos)?;

        // Find the second newline (end of the blank line we want to keep)
        let second_newline = source[first_newline + 1..]
            .find('\n')
            .map(|pos| first_newline + 1 + pos)?;

        // Find the start of the line containing next_content
        let next_line_start = source[..next_content_start_byte]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        // Delete from after second newline to the start of next_content's line
        if second_newline + 1 < next_line_start {
            let start = TextSize::new((second_newline + 1) as u32);
            let end = TextSize::new(next_line_start as u32);
            Some(Fix::safe_edit(Edit::deletion(start, end)))
        } else {
            None
        }
    }

    /// Create a fix that deletes excess blank lines when we don't want any blank lines.
    /// Used for TooManyEmptyLinesAfter where we want 0 blank lines, not 1.
    fn create_delete_all_excess_lines_fix(
        &self,
        source: &str,
        content_end_byte: usize,
        next_content_start_byte: usize,
    ) -> Option<Fix> {
        // Find the end of the content line (first \n after content_end_byte)
        let first_newline = source[content_end_byte..]
            .find('\n')
            .map(|pos| content_end_byte + pos)?;

        // Find the start of the line containing next_content
        let next_line_start = source[..next_content_start_byte]
            .rfind('\n')
            .map(|pos| pos + 1)
            .unwrap_or(0);

        // Delete from after first newline to the start of next_content's line
        // This keeps exactly one newline (the line break after content)
        if first_newline + 1 < next_line_start {
            let start = TextSize::new((first_newline + 1) as u32);
            let end = TextSize::new(next_line_start as u32);
            Some(Fix::safe_edit(Edit::deletion(start, end)))
        } else {
            None
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
        let rule = EmptyLineSeparator::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_with_config(source: &str, rule: EmptyLineSeparator) -> Vec<Diagnostic> {
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
    fn test_method_needs_blank_line() {
        let source = r#"
class Test {
    void method1() {}
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "method2 should need blank line before it"
        );
    }

    #[test]
    fn test_method_has_blank_line_ok() {
        let source = r#"
class Test {
    void method1() {}

    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "method2 has blank line, should be OK"
        );
    }

    #[test]
    fn test_constructor_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    Test() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.iter().any(|d| d.kind.body.contains("CTOR_DEF")),
            "constructor should need blank line"
        );
    }

    #[test]
    fn test_field_needs_blank_line_default() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "field y should need blank line (default config)"
        );
    }

    #[test]
    fn test_field_no_blank_line_allowed() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let rule = EmptyLineSeparator {
            allow_no_empty_line_between_fields: true,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics.is_empty(),
            "fields without blank lines should be OK when allowNoEmptyLineBetweenFields=true"
        );
    }

    #[test]
    fn test_static_init_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    static {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("STATIC_INIT")),
            "static init should need blank line"
        );
    }

    #[test]
    fn test_comment_before_method_ok() {
        let source = r#"
class Test {
    void method1() {}

    // comment before method2
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "blank line before comment should satisfy requirement"
        );
    }

    #[test]
    fn test_first_member_no_violation() {
        let source = r#"
class Test {
    void method1() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "first member should not need blank line"
        );
    }

    #[test]
    fn test_javadoc_before_method_ok() {
        let source = r#"
class Test {
    void method1() {}

    /**
     * Javadoc comment
     */
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "blank line before javadoc should satisfy requirement"
        );
    }

    #[test]
    fn test_agrona_like_structure() {
        // Mimics the UnsafeBufferPosition.java structure
        let source = r#"
class Test {
    private int x;

    @SuppressWarnings("unused")
    private int y;

    /**
     * Doc
     */
    public Test() {}
}
"#;
        let diagnostics = check_source(source);
        println!("\nDiagnostics:");
        for d in &diagnostics {
            println!("  {:?}", d);
        }
        assert!(
            diagnostics.is_empty(),
            "should have no violations - there are blank lines before all members"
        );
    }

    #[test]
    #[ignore] // Only run manually to debug
    fn test_actual_agrona_file() {
        test_real_file(
            "/Users/shaunlaurens/src/lintal/target/agrona/agrona/src/main/java/org/agrona/concurrent/status/UnsafeBufferPosition.java",
        );
    }

    #[test]
    #[ignore] // Only run manually to debug
    fn test_aeron_counter_file() {
        test_real_file(
            "/Users/shaunlaurens/src/lintal/target/aeron/aeron-client/src/main/java/io/aeron/Counter.java",
        );
    }

    fn test_real_file(path: &str) {
        use std::collections::HashSet;

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                println!("File not found, skipping");
                return;
            }
        };

        use lintal_java_parser::JavaParser;
        let mut parser = JavaParser::new();
        let result = parser.parse(&source).unwrap();
        let root = result.tree.root_node();

        fn find_class_bodies<'a>(
            node: tree_sitter::Node<'a>,
            bodies: &mut Vec<tree_sitter::Node<'a>>,
        ) {
            if node.kind() == "class_body" {
                bodies.push(node);
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                find_class_bodies(child, bodies);
            }
        }

        let mut bodies = vec![];
        find_class_bodies(root, &mut bodies);

        for class_body in bodies {
            println!("\nClass body @ row {}:", class_body.start_position().row);
            let mut cursor = class_body.walk();
            let children: Vec<_> = class_body.children(&mut cursor).collect();

            for (i, child) in children.iter().enumerate() {
                println!(
                    "  [{}] {} @ row {}-{} extra={}",
                    i,
                    child.kind(),
                    child.start_position().row,
                    child.end_position().row,
                    child.is_extra()
                );
            }
        }

        // Now check with actual rule
        let ctx = CheckContext::new(&source);
        let mut tokens = HashSet::new();
        tokens.insert(EmptyLineSeparatorToken::MethodDef);
        tokens.insert(EmptyLineSeparatorToken::CtorDef);
        tokens.insert(EmptyLineSeparatorToken::ClassDef);
        tokens.insert(EmptyLineSeparatorToken::InterfaceDef);
        tokens.insert(EmptyLineSeparatorToken::EnumDef);
        tokens.insert(EmptyLineSeparatorToken::StaticInit);
        tokens.insert(EmptyLineSeparatorToken::InstanceInit);
        tokens.insert(EmptyLineSeparatorToken::Import);

        let rule = EmptyLineSeparator {
            allow_no_empty_line_between_fields: true,
            tokens,
            ..Default::default()
        };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), &source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }

        println!("\nDiagnostics:");
        for d in &diagnostics {
            let line = ctx.source_code().line_column(d.range.start()).line.get();
            println!("  Line {}: {}", line, d.kind.body);
        }
    }

    #[test]
    fn test_multiple_empty_lines_inside_method() {
        let source = r#"
class Test {
    void method() {
        int x = 1;


        int y = 2;
    }
}
"#;
        let rule = EmptyLineSeparator {
            allow_multiple_empty_lines_inside_class_members: false,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("more than 1 empty line")),
            "should detect multiple empty lines inside method body"
        );
    }

    #[test]
    fn test_multiple_empty_lines_inside_constructor() {
        let source = r#"
class Test {
    Test() {
        int x = 1;


    }
}
"#;
        let rule = EmptyLineSeparator {
            allow_multiple_empty_lines_inside_class_members: false,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("more than 1 empty line")),
            "should detect multiple empty lines inside constructor body"
        );
    }

    #[test]
    fn test_multiple_empty_lines_inside_nested_block() {
        // Test empty lines inside a try block (nested in method)
        let source = r#"
class Test {
    void method() {
        try {
            int x = 1;


        } catch (Exception e) {
        }
    }
}
"#;
        let rule = EmptyLineSeparator {
            allow_multiple_empty_lines_inside_class_members: false,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        println!("Diagnostics for nested block test:");
        for d in &diagnostics {
            println!("  {}", d.kind.body);
        }
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("more than 1 empty line")),
            "should detect multiple empty lines inside nested try block"
        );
    }
}
