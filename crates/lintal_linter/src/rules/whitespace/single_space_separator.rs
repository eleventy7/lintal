//! SingleSpaceSeparator rule implementation.
//!
//! Checks that tokens are separated by exactly one space (no multiple spaces).
//! Unlike other whitespace rules, this checks the whitespace at each token position
//! to ensure tokens are separated correctly.
//!
//! Checkstyle equivalent: SingleSpaceSeparatorCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_source_file::{LineIndex, SourceCode};
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: multiple spaces separating non-whitespace characters.
#[derive(Debug, Clone)]
pub struct SingleSpaceSeparatorViolation;

impl Violation for SingleSpaceSeparatorViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "'Use a single space to separate non-whitespace characters".to_string()
    }
}

/// Configuration for SingleSpaceSeparator rule.
#[derive(Debug, Clone, Default)]
pub struct SingleSpaceSeparator {
    /// Control whether to validate whitespaces surrounding comments.
    pub validate_comments: bool,
}

impl FromConfig for SingleSpaceSeparator {
    const MODULE_NAME: &'static str = "SingleSpaceSeparator";

    fn from_config(properties: &Properties) -> Self {
        let validate_comments = properties
            .get("validateComments")
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        Self { validate_comments }
    }
}

impl Rule for SingleSpaceSeparator {
    fn name(&self) -> &'static str {
        "SingleSpaceSeparator"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Skip comment nodes if validateComments is false
        if !self.validate_comments && is_comment(node) {
            return vec![];
        }

        // Skip tokens on lines with comments if validateComments is false
        if !self.validate_comments && has_comment_on_line(node, ctx.source()) {
            return vec![];
        }

        // Check whitespace before this token
        check_token_whitespace(ctx, node, self.validate_comments)
    }
}

/// Check if a node is a comment.
fn is_comment(node: &CstNode) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment" | "comment")
}

/// Check if there's a comment on the same line as this node.
/// This is a simplified check - we look for line_comment or block_comment siblings.
fn has_comment_on_line(node: &CstNode, source: &str) -> bool {
    // Get the line this node is on
    let node_start = node.range().start();
    let node_line_start = source[..usize::from(node_start)]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let node_line_end = source[usize::from(node_start)..]
        .find('\n')
        .map(|pos| usize::from(node_start) + pos)
        .unwrap_or(source.len());

    let line = &source[node_line_start..node_line_end];

    // Simple heuristic: check if the line contains comment markers
    line.contains("//") || (line.contains("/*") && line.contains("*/"))
}

/// Check if a node is inside a string literal or character literal.
fn is_inside_literal(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if matches!(
            parent.kind(),
            "string_literal" | "character_literal" | "text_block"
        ) {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Check the whitespace before a token.
fn check_token_whitespace(
    ctx: &CheckContext,
    node: &CstNode,
    validate_comments: bool,
) -> Vec<Diagnostic> {
    // Skip tokens inside string literals
    if is_inside_literal(node) {
        return vec![];
    }

    let source = ctx.source();
    let range = node.range();
    let start = range.start();

    // Get the line and column for this token
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);
    let loc = source_code.line_column(start);

    // Column is 1-indexed, convert to 0-indexed
    let column_no = loc.column.get() - 1;

    // Per checkstyle: minimum column for second space is 2
    // (e.g., "j  " has second space at column 2)
    if column_no < 2 {
        return vec![];
    }

    // Get the line text
    let line_start_offset = usize::from(line_index.line_start(loc.line, source));
    let line_end_offset = usize::from(line_index.line_end(loc.line, source));
    let line = &source[line_start_offset..line_end_offset];
    let chars: Vec<char> = line.chars().collect();

    if column_no >= chars.len() {
        return vec![];
    }

    // Check if there are multiple spaces BEFORE this token
    // Count consecutive spaces before column_no
    let mut space_count = 0;
    let mut check_pos = column_no;
    while check_pos > 0 && chars[check_pos - 1] == ' ' {
        space_count += 1;
        check_pos -= 1;
    }

    // If there are 0 or 1 spaces before this token, it's fine
    if space_count <= 1 {
        return vec![];
    }

    // We have multiple spaces. Check exemptions:
    // 1. If all before is whitespace (indentation)
    if is_first_in_line(&chars, column_no) {
        return vec![];
    }

    // 2. If !validateComments and there's a block comment end before
    if !validate_comments && is_block_comment_end(&chars, check_pos) {
        return vec![];
    }

    // We have a violation - report it at the position of the SECOND space
    // (per checkstyle convention)
    let space_start = check_pos; // Position after last non-space
    let violation_col = space_start + 1; // Second space position

    let violation_offset = line_start_offset + violation_col;
    let diag_range = TextRange::new(
        TextSize::new(violation_offset as u32),
        TextSize::new((violation_offset + 1) as u32),
    );

    // For the fix: replace all the spaces with a single space
    let fix_range = TextRange::new(
        TextSize::new((line_start_offset + space_start) as u32),
        TextSize::new((line_start_offset + column_no) as u32),
    );

    let diagnostic = Diagnostic::new(SingleSpaceSeparatorViolation, diag_range).with_fix(
        Fix::safe_edit(Edit::range_replacement(" ".to_string(), fix_range)),
    );

    vec![diagnostic]
}

/// Check if the position is the first non-whitespace on the line.
/// All characters before `column_no` must be whitespace.
fn is_first_in_line(chars: &[char], column_no: usize) -> bool {
    chars[..column_no].iter().all(|c| c.is_whitespace())
}

/// Check if the text before column_no ends with a block comment end "*/".
fn is_block_comment_end(chars: &[char], column_no: usize) -> bool {
    // Strip trailing whitespace before column_no and check for "*/"
    let before = &chars[..column_no];

    // Find last non-whitespace sequence
    let mut end_idx = before.len();
    while end_idx > 0 && before[end_idx - 1].is_whitespace() {
        end_idx -= 1;
    }

    if end_idx < 2 {
        return false;
    }

    before[end_idx - 2..end_idx] == ['*', '/']
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_first_in_line() {
        let chars: Vec<char> = "    foo".chars().collect();
        assert!(is_first_in_line(&chars, 4)); // All spaces before index 4
        assert!(!is_first_in_line(&chars, 5)); // 'f' is at index 4
    }

    #[test]
    fn test_is_block_comment_end() {
        let chars: Vec<char> = "foo */  bar".chars().collect();
        assert!(is_block_comment_end(&chars, 8)); // After "*/"

        let chars2: Vec<char> = "foo  bar".chars().collect();
        assert!(!is_block_comment_end(&chars2, 5)); // No "*/"
    }
}
