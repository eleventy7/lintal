//! LineLength rule implementation.
//!
//! Checks that lines do not exceed a specified length.
//!
//! Checkstyle equivalent: LineLengthCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: line is too long.
#[derive(Debug, Clone)]
pub struct LineLengthViolation {
    pub max: usize,
    pub len: usize,
}

impl Violation for LineLengthViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Line is longer than {} characters (found {}).",
            self.max, self.len
        )
    }
}

/// Configuration for LineLength rule.
#[derive(Debug, Clone)]
pub struct LineLength {
    /// Maximum allowed line length (default: 80).
    pub max: usize,
    /// Optional regex pattern for lines to ignore.
    pub ignore_pattern: Option<Regex>,
}

const RELEVANT_KINDS: &[&str] = &["program"];

impl Default for LineLength {
    fn default() -> Self {
        Self {
            max: 80,
            ignore_pattern: None,
        }
    }
}

impl FromConfig for LineLength {
    const MODULE_NAME: &'static str = "LineLength";

    fn from_config(properties: &Properties) -> Self {
        let max = properties
            .get("max")
            .and_then(|s| s.parse().ok())
            .unwrap_or(80);

        let ignore_pattern = properties
            .get("ignorePattern")
            .and_then(|s| Regex::new(s).ok());

        Self {
            max,
            ignore_pattern,
        }
    }
}

impl Rule for LineLength {
    fn name(&self) -> &'static str {
        "LineLength"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at the root node
        if node.parent().is_some() {
            return vec![];
        }

        let source = ctx.source();
        let source_code = ctx.source_code();
        let mut diagnostics = vec![];

        for (line_no, line_text) in source.lines().enumerate() {
            // Count characters, not bytes — matches checkstyle behavior for Unicode
            let char_len = line_text.chars().count();
            if char_len <= self.max {
                continue;
            }

            // Skip lines matching ignore pattern
            if let Some(ref pattern) = self.ignore_pattern
                && pattern.is_match(line_text)
            {
                continue;
            }

            let line_idx = lintal_source_file::OneIndexed::new(line_no + 1).unwrap();
            let line_start = source_code.line_start(line_idx);

            // Calculate byte offset for the character at position `max`
            let byte_offset_at_max: usize = line_text
                .char_indices()
                .nth(self.max)
                .map(|(i, _)| i)
                .unwrap_or(line_text.len());

            let diag_start = usize::from(line_start) + byte_offset_at_max;
            let diag_end = usize::from(line_start) + line_text.len();

            let diag_range = TextRange::new(
                TextSize::new(diag_start as u32),
                TextSize::new(diag_end as u32),
            );

            diagnostics.push(Diagnostic::new(
                LineLengthViolation {
                    max: self.max,
                    len: char_len,
                },
                diag_range,
            ));
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str, max: usize) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = LineLength {
            max,
            ignore_pattern: None,
        };
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push(loc.line.get());
            }
        }
        violations
    }

    #[test]
    fn test_short_lines_no_violation() {
        let source = "class Foo {\n    int x;\n}\n";
        let violations = check_source(source, 80);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_long_line_violation() {
        let long_line = format!("class Foo {{ String s = \"{}\"; }}", "x".repeat(100));
        let violations = check_source(&long_line, 80);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0], 1);
    }

    #[test]
    fn test_custom_max() {
        let source = "class Foo { int x = 42; }\n";
        // Line is 25 chars
        let violations = check_source(source, 20);
        assert_eq!(violations.len(), 1);

        let violations = check_source(source, 30);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_ignore_pattern() {
        let source = "class Foo {\n    // This is a very long comment that exceeds the limit for sure by a lot of characters\n    int x;\n}\n";
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = LineLength {
            max: 40,
            ignore_pattern: Some(Regex::new(r"^\s*//").unwrap()),
        };
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push(loc.line.get());
            }
        }

        // The comment line should be ignored
        assert!(violations.is_empty(), "Comment lines should be ignored");
    }

    #[test]
    fn test_unicode_counts_characters_not_bytes() {
        // Arabic text: each char is 2 bytes in UTF-8
        // "روباه" is 5 characters but 10 bytes
        // Line: class + space + Foo + space + { + space + String + space + s + space + = + space
        //       + "روباه" (5 chars in quotes = 7 with quotes) + ; + space + }
        // Build a line that is under 80 chars but over 80 bytes
        let arabic = "روباه قهوه ای سریع از روی سگ تنبل می پرد";
        let line = format!("class Foo {{ String s = \"{}\"; }}", arabic);
        let char_count = line.chars().count();
        let byte_count = line.len();

        // Verify the test setup: chars < 80 but bytes > 80
        assert!(
            char_count < 80,
            "Test line should be under 80 chars, got {}",
            char_count
        );
        assert!(
            byte_count > 80,
            "Test line should be over 80 bytes, got {}",
            byte_count
        );

        // Should NOT trigger with max=80 since char count < 80
        let violations = check_source(&line, 80);
        assert!(
            violations.is_empty(),
            "Unicode line with {} chars should not violate max=80 (byte len={})",
            char_count,
            byte_count
        );
    }

    #[test]
    fn test_unicode_long_line_does_violate() {
        // CJK characters: each char is 3 bytes in UTF-8
        let cjk = "あ".repeat(50); // 50 chars, 150 bytes
        let line = format!("class Foo {{ String s = \"{}\"; }}", cjk);
        let char_count = line.chars().count();
        assert!(char_count > 60);

        let violations = check_source(&line, 60);
        assert_eq!(
            violations.len(),
            1,
            "Long Unicode line should still violate"
        );
    }
}
