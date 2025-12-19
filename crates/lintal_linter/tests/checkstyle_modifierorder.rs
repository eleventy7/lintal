//! Checkstyle compatibility tests for ModifierOrder rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the ModifierOrder check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::ModifierOrder;
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
    fn modifier_order(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message_key: "mod.order",
        }
    }

    fn annotation_order(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message_key: "annotation.order",
        }
    }
}

/// Run ModifierOrder rule on source and collect violations.
fn check_modifier_order(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let properties = HashMap::new();
    let rule = ModifierOrder::from_config(&properties);
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
            let message_key = if message.contains("annotation") {
                "annotation.order"
            } else {
                "mod.order"
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
fn load_modifierorder_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/modifier/modifierorder")
        .join(file_name);
    std::fs::read_to_string(&path).ok()
}

/// Helper to verify violations match expected.
fn verify_violations(violations: &[Violation], expected: &[Violation]) {
    let mut missing = vec![];
    let mut unexpected = vec![];

    for exp in expected {
        let matched = violations.iter().any(|v| {
            v.line == exp.line && v.column == exp.column && v.message_key == exp.message_key
        });

        if !matched {
            missing.push(exp.clone());
        }
    }

    for actual in violations {
        let matched = expected.iter().any(|v| {
            v.line == actual.line
                && v.column == actual.column
                && v.message_key == actual.message_key
        });

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
// Test: testItOne
// File: InputModifierOrderItOne.java
// Expected violations from checkstyle test:
//   15:10: mod.order (final)
//   19:12: mod.order (private)
//   25:14: mod.order (private)
//   35:13: annotation.order (@MyAnnotation2)
//   40:13: annotation.order (@MyAnnotation2)
//   50:35: annotation.order (@MyAnnotation4)
// =============================================================================

#[test]
fn test_modifier_order_it_one() {
    let Some(source) = load_modifierorder_fixture("InputModifierOrderItOne.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_modifier_order(&source);

    let expected = vec![
        Violation::modifier_order(15, 10),
        Violation::modifier_order(19, 12),
        Violation::modifier_order(25, 14),
        Violation::annotation_order(35, 13),
        Violation::annotation_order(40, 13),
        Violation::annotation_order(50, 35),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testItTwo
// File: InputModifierOrderItTwo.java
// Expected violations from checkstyle test:
//   15:10: mod.order (final)
//   57:14: mod.order (default)
// =============================================================================

#[test]
fn test_modifier_order_it_two() {
    let Some(source) = load_modifierorder_fixture("InputModifierOrderItTwo.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_modifier_order(&source);

    let expected = vec![
        Violation::modifier_order(15, 10),
        Violation::modifier_order(57, 14),
    ];

    verify_violations(&violations, &expected);
}
