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

    // Task 5: Interface/annotation modifier violations
    // Task 7: Final method violations (lines 82, 91)
    let expected = vec![
        Violation::new(57, 12, "static"),   // static nested interface
        Violation::new(60, 9, "public"),    // public interface method
        Violation::new(66, 9, "abstract"),  // abstract interface method
        Violation::new(69, 9, "public"),    // public interface field
        Violation::new(75, 9, "final"),     // final interface field
        Violation::new(82, 13, "final"),    // final on private method (Task 7)
        Violation::new(91, 12, "final"),    // final on method in final class (Task 7)
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

#[test]
fn test_enum_constructor_is_implicitly_private() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierConstructorModifier.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Enum constructors are implicitly private
    let expected = vec![Violation::new(14, 5, "private")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_annotation_on_enum_constructor() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierAnnotationOnEnumConstructor.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Private modifier is redundant even with annotation
    let expected = vec![Violation::new(22, 5, "private")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_not_public_class_constructor_has_not_public_modifier() {
    let Some(source) = load_redundantmodifier_fixture(
        "InputRedundantModifierPublicModifierInNotPublicClass.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Public modifier is redundant on constructor of non-public class
    let expected = vec![Violation::new(22, 5, "public")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_nested_static_enum() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierStaticModifierInNestedEnum.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Nested enums are implicitly static
    let expected = vec![
        Violation::new(12, 5, "static"), // nested in class
        Violation::new(16, 9, "static"), // nested in enum
        Violation::new(20, 9, "static"), // nested in interface
    ];

    verify_violations(&violations, &expected);
}

#[test]
fn test_final_in_anonymous_class() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierFinalInAnonymousClass.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on method in anonymous class is redundant
    let expected = vec![Violation::new(22, 20, "final")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_private_method_in_private_class() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierPrivateMethodInPrivateClass.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on private method is redundant
    let expected = vec![Violation::new(13, 17, "final")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_enum_static_methods() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierFinalInEnumStaticMethods.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on static method in enum is redundant (static methods are not overridable)
    let expected = vec![Violation::new(20, 23, "final")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_enum_methods() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierFinalInEnumMethods.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on methods inside enum constant bodies (anonymous classes) is redundant
    let expected = vec![
        Violation::new(15, 16, "final"), // E2 constant body
        Violation::new(30, 16, "final"), // E1 constant body in second enum
    ];

    verify_violations(&violations, &expected);
}

#[test]
fn test_final_in_try_with_resource() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierFinalInTryWithResource.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on try-with-resources variables is redundant
    let expected = vec![
        Violation::new(38, 14, "final"),
        Violation::new(43, 14, "final"),
        Violation::new(44, 17, "final"),
    ];

    verify_violations(&violations, &expected);
}

#[test]
fn test_try_with_resources_block() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierTryWithResources.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on try-with-resources variables is redundant
    let expected = vec![Violation::new(18, 19, "final")];

    verify_violations(&violations, &expected);
}

#[test]
fn test_final_in_abstract_methods() {
    let Some(source) =
        load_redundantmodifier_fixture("InputRedundantModifierFinalInAbstractMethods.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_modifier(&source, None);

    // Final on abstract method parameters is redundant
    let expected = vec![
        Violation::new(12, 33, "final"), // abstract method
        Violation::new(16, 49, "final"), // abstract method
        Violation::new(19, 17, "final"), // interface method
        Violation::new(24, 24, "final"), // native method
        Violation::new(33, 33, "final"), // abstract method in enum
    ];

    verify_violations(&violations, &expected);
}
