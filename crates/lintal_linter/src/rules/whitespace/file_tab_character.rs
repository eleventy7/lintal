//! FileTabCharacter rule implementation.
//!
//! Checks that there are no tab characters in the source code.
//! This is the simplest whitespace rule - it scans raw text without needing tree-sitter.
//!
//! Checkstyle equivalent: FileTabCharacterCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: file contains tab character.
#[derive(Debug, Clone)]
pub struct FileContainsTabViolation;

impl Violation for FileContainsTabViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "File contains tab characters (this is the first instance)".to_string()
    }
}

/// Violation: line contains tab character (when eachLine=true).
#[derive(Debug, Clone)]
pub struct LineContainsTabViolation;

impl Violation for LineContainsTabViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Line contains a tab character".to_string()
    }
}

/// Configuration for FileTabCharacter rule.
#[derive(Debug, Clone)]
pub struct FileTabCharacter {
    /// Control whether to report on each line containing a tab, or just the first instance.
    pub each_line: bool,
    /// Tab width for converting tabs to spaces (default: 8).
    pub tab_width: usize,
}

const RELEVANT_KINDS: &[&str] = &["program"];

impl Default for FileTabCharacter {
    fn default() -> Self {
        Self {
            each_line: false,
            tab_width: 8,
        }
    }
}

impl FromConfig for FileTabCharacter {
    const MODULE_NAME: &'static str = "FileTabCharacter";

    fn from_config(properties: &Properties) -> Self {
        let each_line = properties
            .get("eachLine")
            .and_then(|s| s.parse().ok())
            .unwrap_or(false);

        let tab_width = properties
            .get("tabWidth")
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);

        Self {
            each_line,
            tab_width,
        }
    }
}

impl Rule for FileTabCharacter {
    fn name(&self) -> &'static str {
        "FileTabCharacter"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at the root node to avoid scanning the file multiple times
        if node.parent().is_some() {
            return vec![];
        }

        let source = ctx.source();
        let source_code = ctx.source_code();

        let mut diagnostics = vec![];

        // Scan each line for tabs
        for (line_no, line_text) in source.lines().enumerate() {
            let line_no = line_no + 1; // 1-indexed

            if let Some(tab_position) = line_text.find('\t') {
                // Calculate the byte offset of the tab in the file
                // Use OneIndexed for line number
                let line_idx = lintal_source_file::OneIndexed::new(line_no).unwrap();
                let line_start = source_code.line_start(line_idx);
                let tab_offset = usize::from(line_start) + tab_position;

                // Create diagnostic range at the tab position
                let diag_range = TextRange::new(
                    TextSize::new(tab_offset as u32),
                    TextSize::new((tab_offset + 1) as u32),
                );

                // Create the appropriate violation based on eachLine setting
                let diagnostic = if self.each_line {
                    Diagnostic::new(LineContainsTabViolation, diag_range)
                } else {
                    Diagnostic::new(FileContainsTabViolation, diag_range)
                };

                // Add fix: replace tab with spaces
                let fix = create_tab_fix(line_text, tab_position, tab_offset, self.tab_width);
                let diagnostic = diagnostic.with_fix(fix);

                diagnostics.push(diagnostic);

                // If not checking each line, stop after first tab
                if !self.each_line {
                    break;
                }
            }
        }

        diagnostics
    }
}

/// Create a fix for a tab character.
/// Calculates the appropriate number of spaces based on column position and tab width.
fn create_tab_fix(line: &str, tab_position: usize, tab_offset: usize, tab_width: usize) -> Fix {
    // Calculate the visual column of the tab
    let mut visual_column = 0;
    for ch in line.chars().take(tab_position) {
        if ch == '\t' {
            visual_column += tab_width - (visual_column % tab_width);
        } else {
            visual_column += 1;
        }
    }

    // Calculate how many spaces to insert to reach next tab stop
    let spaces_needed = tab_width - (visual_column % tab_width);
    let replacement = " ".repeat(spaces_needed);

    let fix_range = TextRange::new(
        TextSize::new(tab_offset as u32),
        TextSize::new((tab_offset + 1) as u32),
    );

    Fix::safe_edit(Edit::range_replacement(replacement, fix_range))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visual_column_calculation() {
        // Simple case: tab at start of line
        let line = "\tfoo";
        let tab_pos = 0;
        let fix = create_tab_fix(line, tab_pos, 0, 8);
        // Should replace with 8 spaces (visual column 0 -> next stop at 8)
        assert_eq!(
            fix.edits()[0].content().unwrap(),
            "        " // 8 spaces
        );
    }

    #[test]
    fn test_tab_after_text() {
        // Tab after 3 chars: visual column 3, next stop at 8, need 5 spaces
        let line = "foo\tbar";
        let tab_pos = 3;
        let fix = create_tab_fix(line, tab_pos, 3, 8);
        assert_eq!(
            fix.edits()[0].content().unwrap(),
            "     " // 5 spaces
        );
    }

    #[test]
    fn test_tab_width_4() {
        // Tab width of 4: after 3 chars, next stop at 4, need 1 space
        let line = "foo\tbar";
        let tab_pos = 3;
        let fix = create_tab_fix(line, tab_pos, 3, 4);
        assert_eq!(fix.edits()[0].content().unwrap(), " "); // 1 space
    }

    #[test]
    fn test_multiple_tabs() {
        // Two tabs at start with tab_width 4
        let line = "\t\tfoo";

        // First tab: column 0 -> 4 spaces
        let fix1 = create_tab_fix(line, 0, 0, 4);
        assert_eq!(fix1.edits()[0].content().unwrap(), "    ");

        // Second tab would need recalculation after first is fixed
    }
}
