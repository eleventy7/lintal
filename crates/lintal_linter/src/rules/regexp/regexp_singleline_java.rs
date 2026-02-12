//! RegexpSinglelineJava rule implementation.
//!
//! Checks that a specified pattern does not match in Java source files.
//! Optionally ignores comments when matching.
//!
//! Checkstyle equivalent: RegexpSinglelineJavaCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use regex::Regex;
use tree_sitter::Node;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: line matches illegal pattern.
#[derive(Debug, Clone)]
pub struct RegexpSinglelineJavaMatchViolation {
    pub message: String,
}

impl Violation for RegexpSinglelineJavaMatchViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        self.message.clone()
    }
}

/// Violation: file does not meet minimum match count.
#[derive(Debug, Clone)]
pub struct RegexpSinglelineJavaMinimumViolation {
    pub message: String,
}

impl Violation for RegexpSinglelineJavaMinimumViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        self.message.clone()
    }
}

/// Configuration for RegexpSinglelineJava rule.
#[derive(Debug, Clone)]
pub struct RegexpSinglelineJava {
    pub format: Regex,
    pub format_str: String,
    pub ignore_case: bool,
    pub ignore_comments: bool,
    pub minimum: usize,
    pub maximum: usize,
    pub message: Option<String>,
}

const RELEVANT_KINDS: &[&str] = &["program"];

impl Default for RegexpSinglelineJava {
    fn default() -> Self {
        Self {
            format: Regex::new("$.").unwrap(),
            format_str: "$.".to_string(),
            ignore_case: false,
            ignore_comments: false,
            minimum: 0,
            maximum: 0,
            message: None,
        }
    }
}

impl FromConfig for RegexpSinglelineJava {
    const MODULE_NAME: &'static str = "RegexpSinglelineJava";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or("$.")
            .to_string();
        let ignore_case = properties.get("ignoreCase").is_some_and(|v| *v == "true");
        let ignore_comments = properties
            .get("ignoreComments")
            .is_some_and(|v| *v == "true");
        let minimum = properties
            .get("minimum")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let maximum = properties
            .get("maximum")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let message = properties
            .get("message")
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());

        let pattern = if ignore_case {
            format!("(?i){}", format_str)
        } else {
            format_str.clone()
        };

        let format = Regex::new(&pattern).unwrap_or_else(|_| Regex::new("$.").unwrap());

        Self {
            format,
            format_str,
            ignore_case,
            ignore_comments,
            minimum,
            maximum,
            message,
        }
    }
}

impl Rule for RegexpSinglelineJava {
    fn name(&self) -> &'static str {
        "RegexpSinglelineJava"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only run at root node
        if node.parent().is_some() {
            return vec![];
        }

        let source = ctx.source();
        let source_code = ctx.source_code();

        // Collect comment ranges if ignoring comments
        let comment_ranges = if self.ignore_comments {
            collect_comment_ranges(node.inner())
        } else {
            vec![]
        };

        let mut match_count = 0usize;
        let mut diagnostics = vec![];

        for (line_idx, line_text) in source.lines().enumerate() {
            let line_num = line_idx + 1;

            let line_one = lintal_source_file::OneIndexed::new(line_num).unwrap();
            let line_start_offset = usize::from(source_code.line_start(line_one));

            // Check if the regex matches this line (outside of comments)
            let has_match = if self.ignore_comments && !comment_ranges.is_empty() {
                // Find all matches and check if any are outside comments
                let mut found_non_comment_match = false;
                for m in self.format.find_iter(line_text) {
                    let match_start = line_start_offset + m.start();
                    let match_end = line_start_offset + m.end();
                    if !overlaps_comment(match_start, match_end, &comment_ranges) {
                        found_non_comment_match = true;
                        break;
                    }
                }
                found_non_comment_match
            } else {
                self.format.is_match(line_text)
            };

            if has_match {
                match_count += 1;

                // Check if this match exceeds the maximum
                if self.maximum == 0 || match_count > self.maximum {
                    let line_start = source_code.line_start(line_one);
                    let diag_range = TextRange::new(
                        line_start,
                        TextSize::new((usize::from(line_start) + line_text.len()) as u32),
                    );

                    let message = if let Some(ref msg) = self.message {
                        msg.clone()
                    } else {
                        format!("Line matches the illegal pattern '{}'.", self.format_str)
                    };

                    diagnostics.push(Diagnostic::new(
                        RegexpSinglelineJavaMatchViolation { message },
                        diag_range,
                    ));
                }
            }
        }

        // Check minimum requirement
        if self.minimum > 0 && match_count < self.minimum {
            let diag_range = TextRange::new(TextSize::new(0), TextSize::new(0));
            let message = format!(
                "File does not contain minimum {} match(es) for pattern '{}'.",
                self.minimum, self.format_str
            );
            diagnostics.push(Diagnostic::new(
                RegexpSinglelineJavaMinimumViolation { message },
                diag_range,
            ));
        }

        diagnostics
    }
}

/// A byte range representing a comment in the source.
struct CommentRange {
    start: usize,
    end: usize,
}

/// Collect all comment ranges from the AST.
fn collect_comment_ranges(root: Node) -> Vec<CommentRange> {
    let mut ranges = vec![];
    let mut cursor = root.walk();
    collect_comments_recursive(&mut cursor, &mut ranges);
    ranges
}

fn collect_comments_recursive(
    cursor: &mut tree_sitter::TreeCursor,
    ranges: &mut Vec<CommentRange>,
) {
    loop {
        let node = cursor.node();
        let kind = node.kind();

        if kind == "line_comment" || kind == "block_comment" {
            ranges.push(CommentRange {
                start: node.start_byte(),
                end: node.end_byte(),
            });
        } else if cursor.goto_first_child() {
            collect_comments_recursive(cursor, ranges);
            cursor.goto_parent();
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

/// Check if a byte range overlaps with any comment.
fn overlaps_comment(start: usize, end: usize, comment_ranges: &[CommentRange]) -> bool {
    for cr in comment_ranges {
        if start < cr.end && end > cr.start {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str, rule: &RegexpSinglelineJava) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
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
    fn test_basic_match() {
        let source = "class Test {\n    System.out.println(\"hello\");\n}\n";
        let rule = RegexpSinglelineJava {
            format: Regex::new(r"System\.out\.println\(").unwrap(),
            format_str: r"System\.out\.println\(".to_string(),
            maximum: 0,
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0], 2);
    }

    #[test]
    fn test_no_match() {
        let source = "class Test {\n    int x = 1;\n}\n";
        let rule = RegexpSinglelineJava {
            format: Regex::new(r"System\.out").unwrap(),
            format_str: r"System\.out".to_string(),
            maximum: 0,
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_ignore_comments() {
        let source = "class Test {\n    // System.out.println(\"hello\");\n    int x = 1;\n}\n";
        let rule = RegexpSinglelineJava {
            format: Regex::new(r"System\.out\.println\(").unwrap(),
            format_str: r"System\.out\.println\(".to_string(),
            maximum: 0,
            ignore_comments: true,
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_minimum_not_met() {
        let source = "class Test {\n    int x = 1;\n}\n";
        let rule = RegexpSinglelineJava {
            format: Regex::new("package").unwrap(),
            format_str: "package".to_string(),
            minimum: 1,
            maximum: 1000,
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 1);
        // File-level violation at offset 0
        assert_eq!(violations[0], 1);
    }

    #[test]
    fn test_default_pattern_no_violations() {
        let source = "class Test {}\n";
        let rule = RegexpSinglelineJava::default();
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }
}
