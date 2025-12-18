//! Checkstyle compatibility tests for RightCurly rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the RightCurly check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::RightCurly;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    message_key: &'static str,
}

impl Violation {
    fn line_same(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message_key: "line.same",
        }
    }

    fn line_alone(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message_key: "line.alone",
        }
    }

    fn line_break_before(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message_key: "line.break.before",
        }
    }
}

/// Run RightCurly rule with custom config on source and collect violations.
fn check_right_curly(source: &str, option: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    properties.insert("option", option);

    let rule = RightCurly::from_config(&properties);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            let message = diagnostic.kind.body.clone();

            // Parse message to determine violation type
            let message_key = if message.contains("same line") {
                "line.same"
            } else if message.contains("alone") {
                "line.alone"
            } else if message.contains("line break before") {
                "line.break.before"
            } else {
                "unknown"
            };

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                message_key,
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
/// Returns None if the checkstyle repo is not available.
fn load_rightcurly_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/blocks/rightcurly")
        .join(file_name);
    std::fs::read_to_string(&path).ok()
}

/// Helper to verify violations match expected.
fn verify_violations(violations: &[Violation], expected: &[Violation]) {
    let mut missing = vec![];
    let mut unexpected = vec![];

    for exp in expected {
        let matched = violations
            .iter()
            .any(|v| v.line == exp.line && v.column == exp.column && v.message_key == exp.message_key);

        if !matched {
            missing.push(exp.clone());
        }
    }

    for actual in violations {
        let matched = expected
            .iter()
            .any(|v| v.line == actual.line && v.column == actual.column && v.message_key == actual.message_key);

        if !matched {
            unexpected.push(actual.clone());
        }
    }

    if !missing.is_empty() || !unexpected.is_empty() {
        println!("\n=== Violations Report ===");
        if !missing.is_empty() {
            println!("\nMissing violations:");
            for v in &missing {
                println!("  {}:{}: {}", v.line, v.column, v.message_key);
            }
        }
        if !unexpected.is_empty() {
            println!("\nUnexpected violations:");
            for v in &unexpected {
                println!("  {}:{}: {}", v.line, v.column, v.message_key);
            }
        }
        panic!("Violation mismatch detected");
    }
}

// =============================================================================
// Test: testDefault (SAME option with default tokens)
// File: InputRightCurlyLeftTestDefault.java
// Expected violations from checkstyle test:
//   25:17: line.same
//   28:17: line.same
//   40:13: line.same
//   44:13: line.same
//   93:27: line.break.before
// =============================================================================

#[test]
fn test_right_curly_default() {
    let Some(source) = load_rightcurly_fixture("InputRightCurlyLeftTestDefault.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_right_curly(&source, "same");

    let expected = vec![
        Violation::line_same(25, 17),
        Violation::line_same(28, 17),
        Violation::line_same(40, 13),
        Violation::line_same(44, 13),
        Violation::line_break_before(93, 27),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testSame (SAME option with more tokens)
// File: InputRightCurlyLeftTestSame.java
// Expected violations from checkstyle test:
//   26:17: line.same
//   29:17: line.same
//   41:13: line.same
//   45:13: line.same
//   87:5: line.alone
//   94:27: line.break.before
//   189:9: line.alone
//   190:41: line.alone
// =============================================================================

#[test]
fn test_right_curly_same() {
    let Some(source) = load_rightcurly_fixture("InputRightCurlyLeftTestSame.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_right_curly(&source, "same");

    let expected = vec![
        Violation::line_same(26, 17),
        Violation::line_same(29, 17),
        Violation::line_same(41, 13),
        Violation::line_same(45, 13),
        Violation::line_alone(87, 5),
        Violation::line_break_before(94, 27),
        Violation::line_alone(189, 9),
        Violation::line_alone(190, 41),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testCatchWithoutFinally (SAME option)
// File: InputRightCurlyTestWithoutFinally.java
// Expected violations from checkstyle test:
//   19:9: line.same
// =============================================================================

#[test]
fn test_right_curly_catch_without_finally() {
    let Some(source) = load_rightcurly_fixture("InputRightCurlyTestWithoutFinally.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_right_curly(&source, "same");

    let expected = vec![Violation::line_same(19, 9)];

    verify_violations(&violations, &expected);
}
