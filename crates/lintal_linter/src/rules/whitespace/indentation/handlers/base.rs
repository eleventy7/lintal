//! Base handler infrastructure for indentation checking.
//!
//! Provides the common functionality used by all indentation handlers,
//! porting checkstyle's AbstractExpressionHandler.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use std::cell::RefCell;

use super::super::Indentation;
use super::super::indent_level::IndentLevel;

/// Violation for incorrect indentation.
#[derive(Debug, Clone)]
pub struct IndentationError {
    /// Type of element (e.g., "class def", "if", "method def")
    pub element: String,
    /// Actual indentation found
    pub actual: i32,
    /// Expected indentation level(s)
    pub expected: String,
}

impl Violation for IndentationError {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!(
            "'{}' has incorrect indentation level {}, expected level should be {}",
            self.element, self.actual, self.expected
        )
    }
}

/// Violation for incorrect child indentation.
#[derive(Debug, Clone)]
pub struct IndentationChildError {
    /// Type of parent element
    pub parent: String,
    /// Actual indentation found
    pub actual: i32,
    /// Expected indentation level(s)
    pub expected: String,
}

impl Violation for IndentationChildError {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!(
            "'{}' child has incorrect indentation level {}, expected level should be {}",
            self.parent, self.actual, self.expected
        )
    }
}

/// Context for indentation checking, shared across all handlers.
pub struct HandlerContext<'a> {
    /// Source code
    source: &'a str,
    /// Lines of source code (pre-split for efficiency)
    lines: Vec<&'a str>,
    /// Precomputed byte offsets of each line start (for O(log n) line lookup)
    line_offsets: Vec<usize>,
    /// Indentation configuration
    config: &'a Indentation,
    /// Tab width for expanding tabs to spaces
    tab_width: usize,
    /// Accumulated diagnostics
    diagnostics: RefCell<Vec<Diagnostic>>,
}

impl<'a> HandlerContext<'a> {
    /// Creates a new handler context.
    pub fn new(source: &'a str, config: &'a Indentation, tab_width: usize) -> Self {
        let lines: Vec<&str> = source.lines().collect();

        // Precompute line start offsets for O(log n) line number lookup
        // Must handle both LF (\n) and CRLF (\r\n) line endings
        let mut line_offsets = Vec::with_capacity(lines.len() + 1);
        line_offsets.push(0); // First line always starts at 0

        let bytes = source.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\n' {
                // Start of next line is byte after the \n
                line_offsets.push(i + 1);
            }
            i += 1;
        }

        Self {
            source,
            lines,
            line_offsets,
            config,
            tab_width,
            diagnostics: RefCell::new(Vec::new()),
        }
    }

    /// Returns the source code.
    pub fn source(&self) -> &str {
        self.source
    }

    /// Returns the indentation configuration.
    pub fn config(&self) -> &Indentation {
        self.config
    }

    /// Returns the basic offset.
    pub fn basic_offset(&self) -> i32 {
        self.config.basic_offset
    }

    /// Returns the brace adjustment.
    pub fn brace_adjustment(&self) -> i32 {
        self.config.brace_adjustment
    }

    /// Returns the case indent.
    pub fn case_indent(&self) -> i32 {
        self.config.case_indent
    }

    /// Returns the throws indent.
    pub fn throws_indent(&self) -> i32 {
        self.config.throws_indent
    }

    /// Returns the array init indent.
    pub fn array_init_indent(&self) -> i32 {
        self.config.array_init_indent
    }

    /// Returns the line wrapping indentation.
    pub fn line_wrapping_indentation(&self) -> i32 {
        self.config.line_wrapping_indentation
    }

    /// Returns whether strict condition is enforced.
    pub fn force_strict_condition(&self) -> bool {
        self.config.force_strict_condition
    }

    /// Checks if the actual indent is acceptable given the expected indent level.
    /// When force_strict_condition=false, accepts actual >= minimum expected.
    /// Use this for line-wrapped content (method args, lambda bodies, etc.)
    pub fn is_indent_acceptable(&self, actual: i32, expected: &super::super::IndentLevel) -> bool {
        expected.is_acceptable_with_force_strict(actual, self.config.force_strict_condition)
    }

    /// Checks if the actual indent is exactly at an expected level.
    /// Always uses strict checking regardless of forceStrictCondition.
    /// Use this for structural indentation (block children, class members, etc.)
    pub fn is_indent_exact(&self, actual: i32, expected: &super::super::IndentLevel) -> bool {
        expected.is_acceptable(actual)
    }

    /// Returns the tab width.
    pub fn tab_width(&self) -> usize {
        self.tab_width
    }

    /// Gets a line by 0-based line number.
    pub fn get_line(&self, line_no: usize) -> Option<&str> {
        self.lines.get(line_no).copied()
    }

    /// Calculates the column number with tabs expanded.
    /// Converts byte offset within a line to visual column.
    pub fn expanded_tabs_column(&self, line: &str, byte_offset: usize) -> i32 {
        let mut column = 0i32;
        for (i, ch) in line.char_indices() {
            if i >= byte_offset {
                break;
            }
            if ch == '\t' {
                // Round up to next tab stop
                column = ((column / self.tab_width as i32) + 1) * self.tab_width as i32;
            } else {
                column += 1;
            }
        }
        column
    }

    /// Gets the start of the line (column of first non-whitespace) with tabs expanded.
    pub fn get_line_start(&self, line_no: usize) -> i32 {
        let Some(line) = self.get_line(line_no) else {
            return 0;
        };
        self.get_line_start_from_str(line)
    }

    /// Gets the start of a line string with tabs expanded.
    pub fn get_line_start_from_str(&self, line: &str) -> i32 {
        let mut column = 0i32;
        for ch in line.chars() {
            if !ch.is_whitespace() {
                break;
            }
            if ch == '\t' {
                column = ((column / self.tab_width as i32) + 1) * self.tab_width as i32;
            } else {
                column += 1;
            }
        }
        column
    }

    /// Checks if a node is on the start of its line.
    pub fn is_on_start_of_line(&self, node: &CstNode) -> bool {
        let range = node.range();
        let line_no = self.line_no_from_offset(range.start());
        let line_start = self.get_line_start(line_no);
        let node_column = self.column_from_node(node);
        line_start == node_column
    }

    /// Gets the visual column of a node with tabs expanded.
    pub fn column_from_node(&self, node: &CstNode) -> i32 {
        let range = node.range();
        let line_no = self.line_no_from_offset(range.start());
        let Some(line) = self.get_line(line_no) else {
            return 0;
        };

        // Find byte offset within line
        let line_start_offset = self.line_start_offset(line_no);
        let byte_offset_in_line = usize::from(range.start()) - line_start_offset;

        self.expanded_tabs_column(line, byte_offset_in_line)
    }

    /// Gets the 0-based line number from a byte offset.
    /// Uses binary search on precomputed line offsets for O(log n) performance.
    fn line_no_from_offset(&self, offset: TextSize) -> usize {
        let offset = usize::from(offset);
        // Binary search to find the line containing this offset
        match self.line_offsets.binary_search(&offset) {
            Ok(line) => line,                    // Exact match - offset is at start of line
            Err(line) => line.saturating_sub(1), // In the middle of a line
        }
    }

    /// Gets the byte offset of the start of a line.
    /// Uses precomputed line offsets for O(1) performance.
    fn line_start_offset(&self, line_no: usize) -> usize {
        self.line_offsets.get(line_no).copied().unwrap_or(0)
    }

    /// Logs an indentation error.
    pub fn log_error(
        &self,
        node: &CstNode,
        element_type: &str,
        actual_indent: i32,
        expected: &IndentLevel,
    ) {
        let range = node.range();
        let line_no = self.line_no_from_offset(range.start());

        let diagnostic = Diagnostic::new(
            IndentationError {
                element: element_type.to_string(),
                actual: actual_indent,
                expected: expected.to_string(),
            },
            range,
        )
        .with_fix(self.create_fix(line_no, actual_indent, expected.first_level()));

        self.diagnostics.borrow_mut().push(diagnostic);
    }

    /// Logs a child indentation error.
    pub fn log_child_error(
        &self,
        node: &CstNode,
        parent_type: &str,
        actual_indent: i32,
        expected: &IndentLevel,
    ) {
        let range = node.range();
        let line_no = self.line_no_from_offset(range.start());

        let diagnostic = Diagnostic::new(
            IndentationChildError {
                parent: parent_type.to_string(),
                actual: actual_indent,
                expected: expected.to_string(),
            },
            range,
        )
        .with_fix(self.create_fix(line_no, actual_indent, expected.first_level()));

        self.diagnostics.borrow_mut().push(diagnostic);
    }

    /// Creates a fix for incorrect indentation.
    fn create_fix(&self, line_no: usize, _actual: i32, expected: i32) -> Fix {
        let Some(line) = self.get_line(line_no) else {
            return Fix::safe_edit(Edit::insertion(String::new(), TextSize::new(0)));
        };

        let line_start_offset = self.line_start_offset(line_no);

        // Find the end of leading whitespace
        let whitespace_end = line
            .char_indices()
            .find(|(_, ch)| !ch.is_whitespace())
            .map(|(i, _)| i)
            .unwrap_or(line.len());

        let range = TextRange::new(
            TextSize::new(line_start_offset as u32),
            TextSize::new((line_start_offset + whitespace_end) as u32),
        );

        // Create new indentation (using spaces)
        if expected == 0 {
            // No indentation expected - delete existing whitespace
            Fix::safe_edit(Edit::deletion(range.start(), range.end()))
        } else {
            let new_indent = " ".repeat(expected as usize);
            Fix::safe_edit(Edit::range_replacement(new_indent, range))
        }
    }

    /// Takes the accumulated diagnostics.
    pub fn take_diagnostics(&self) -> Vec<Diagnostic> {
        self.diagnostics.take()
    }
}

/// Trait for indentation handlers.
///
/// Each handler is responsible for checking indentation of a specific
/// type of Java construct.
pub trait IndentHandler {
    /// Returns the name of this handler (e.g., "class def", "method def").
    fn name(&self) -> &'static str;

    /// Check the indentation of the construct.
    fn check_indentation(&self, ctx: &HandlerContext, node: &CstNode, parent_indent: &IndentLevel);

    /// Get the expected indentation level for this construct.
    fn get_indent(&self, ctx: &HandlerContext, parent_indent: &IndentLevel) -> IndentLevel {
        // Default: parent indent + basic offset
        parent_indent.with_offset(ctx.basic_offset())
    }

    /// Get the suggested indentation level for children.
    fn get_suggested_child_indent(
        &self,
        ctx: &HandlerContext,
        parent_indent: &IndentLevel,
    ) -> IndentLevel {
        // Default: this handler's indent + basic offset
        self.get_indent(ctx, parent_indent)
            .with_offset(ctx.basic_offset())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Indentation {
        Indentation::default()
    }

    #[test]
    fn test_get_line_start_spaces() {
        let source = "    int x = 1;";
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);
        assert_eq!(ctx.get_line_start(0), 4);
    }

    #[test]
    fn test_get_line_start_tabs() {
        let source = "\t\tint x = 1;";
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);
        assert_eq!(ctx.get_line_start(0), 8);
    }

    #[test]
    fn test_get_line_start_mixed() {
        let source = "  \tint x = 1;"; // 2 spaces + 1 tab
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);
        // 2 spaces = 2, then tab rounds up to next tab stop (4)
        assert_eq!(ctx.get_line_start(0), 4);
    }

    #[test]
    fn test_expanded_tabs_column() {
        let config = create_test_config();
        let ctx = HandlerContext::new("", &config, 4);

        // Tab at column 0 goes to column 4
        assert_eq!(ctx.expanded_tabs_column("\tint x;", 1), 4);

        // Tab at column 2 rounds up to 4
        assert_eq!(ctx.expanded_tabs_column("ab\tx;", 3), 4);

        // Tab at column 4 goes to 8
        assert_eq!(ctx.expanded_tabs_column("abcd\tx;", 5), 8);
    }

    #[test]
    fn test_multiline() {
        let source = "class Foo {\n    int x;\n}";
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);

        assert_eq!(ctx.get_line_start(0), 0);
        assert_eq!(ctx.get_line_start(1), 4);
        assert_eq!(ctx.get_line_start(2), 0);
    }

    #[test]
    fn test_multiline_crlf() {
        // Test CRLF line endings (Windows-style)
        let source = "class Foo {\r\n    int x;\r\n}";
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);

        assert_eq!(ctx.get_line_start(0), 0);
        assert_eq!(ctx.get_line_start(1), 4);
        assert_eq!(ctx.get_line_start(2), 0);
    }

    #[test]
    fn test_line_offsets_crlf() {
        // Verify line number lookup works correctly with CRLF
        // Line 0: "class Foo {" (11 chars) + CRLF = bytes 0-12
        // Line 1: "    int x;" (10 chars) + CRLF = bytes 13-24
        // Line 2: "}" (1 char) = bytes 25-25
        let source = "class Foo {\r\n    int x;\r\n}";
        let config = create_test_config();
        let ctx = HandlerContext::new(source, &config, 4);

        // Byte offset 0 is start of line 0
        assert_eq!(ctx.line_no_from_offset(TextSize::new(0)), 0);
        // Byte offset 5 is still line 0
        assert_eq!(ctx.line_no_from_offset(TextSize::new(5)), 0);
        // Byte offset 13 is start of line 1 (after "class Foo {\r\n")
        assert_eq!(ctx.line_no_from_offset(TextSize::new(13)), 1);
        // Byte offset 17 is still line 1
        assert_eq!(ctx.line_no_from_offset(TextSize::new(17)), 1);
        // Byte offset 25 is start of line 2
        assert_eq!(ctx.line_no_from_offset(TextSize::new(25)), 2);
    }

    #[test]
    fn test_many_lines_crlf() {
        // Simulate the artio test file scenario - many CRLF lines
        // Without correct CRLF handling, offsets drift by 1 byte per line
        let mut source = String::new();
        for i in 0..130 {
            source.push_str(&format!("line{}\r\n", i));
        }
        source.push_str("    final_line");

        let config = create_test_config();
        let ctx = HandlerContext::new(&source, &config, 4);

        // Line 130 should have indentation 4
        assert_eq!(ctx.get_line_start(130), 4);
    }
}
