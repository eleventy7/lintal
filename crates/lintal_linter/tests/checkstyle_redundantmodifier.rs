//! Checkstyle compatibility tests for RedundantModifier rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the RedundantModifier check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::RedundantModifier;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    modifier: String,
}

impl Violation {
    fn new(line: usize, column: usize, modifier: &str) -> Self {
        Self {
            line,
            column,
            modifier: modifier.to_string(),
        }
    }
}

/// Run RedundantModifier rule on source and collect violations.
fn check_redundant_modifier(source: &str, jdk_version: Option<&str>) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    if let Some(version) = jdk_version {
        properties.insert("jdkVersion", version);
    }
    let rule = RedundantModifier::from_config(&properties);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            let message = diagnostic.kind.body.clone();

            // Extract the redundant modifier from the message
            // Message format: "Redundant 'modifier' modifier."
            let modifier = if let Some(start) = message.find('\'') {
                if let Some(end) = message[start + 1..].find('\'') {
                    message[start + 1..start + 1 + end].to_string()
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                modifier,
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
/// Returns None if the checkstyle repo is not available.
fn load_redundantmodifier_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/modifier/redundantmodifier")
        .join(file_name);
    std::fs::read_to_string(&path).ok()
}

/// Helper to verify violations match expected.
fn verify_violations(violations: &[Violation], expected: &[Violation]) {
    let mut missing = vec![];
    let mut unexpected = vec![];

    for exp in expected {
        let matched = violations.iter().any(|v| {
            v.line == exp.line && v.column == exp.column && v.modifier == exp.modifier
        });

        if !matched {
            missing.push(exp.clone());
        }
    }

    for v in violations {
        let matched = expected.iter().any(|exp| {
            v.line == exp.line && v.column == exp.column && v.modifier == exp.modifier
        });

        if !matched {
            unexpected.push(v.clone());
        }
    }

    if !missing.is_empty() || !unexpected.is_empty() {
        eprintln!("\n=== Violation Mismatch ===");
        if !missing.is_empty() {
            eprintln!("\nMissing violations (expected but not found):");
            for v in &missing {
                eprintln!("  {}:{} - {}", v.line, v.column, v.modifier);
            }
        }
        if !unexpected.is_empty() {
            eprintln!("\nUnexpected violations (found but not expected):");
            for v in &unexpected {
                eprintln!("  {}:{} - {}", v.line, v.column, v.modifier);
            }
        }
        panic!("Violations do not match expected");
    }
}

#[test]
fn test_it_one() {
    let Some(source) = load_redundantmodifier_fixture("InputRedundantModifierItOne.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, Some("11"));

    // Task 5: Interface/annotation modifier violations only
    // Lines 82, 91 are Task 7 (final methods in private/final class)
    let expected = vec![
        Violation::new(57, 12, "static"),   // static nested interface
        Violation::new(60, 9, "public"),    // public interface method
        Violation::new(66, 9, "abstract"),  // abstract interface method
        Violation::new(69, 9, "public"),    // public interface field
        Violation::new(75, 9, "final"),     // final interface field
        Violation::new(102, 1, "abstract"), // abstract interface definition
    ];

    verify_violations(&violations, &expected);
}

#[test]
fn test_it_two() {
    let Some(source) = load_redundantmodifier_fixture("InputRedundantModifierItTwo.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, Some("11"));

    let expected = vec![
        Violation::new(22, 5, "public"),
        Violation::new(23, 5, "final"),
        Violation::new(24, 5, "static"),
        Violation::new(26, 5, "public"),
        Violation::new(27, 5, "abstract"),
    ];

    verify_violations(&violations, &expected);
}

#[test]
fn test_classes_inside_of_interfaces() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierClassesInsideOfInterfaces.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    let expected = vec![
        Violation::new(19, 5, "static"),
        Violation::new(25, 5, "public"),
        Violation::new(28, 5, "public"),
        Violation::new(34, 5, "static"),
    ];

    verify_violations(&violations, &expected);
}
