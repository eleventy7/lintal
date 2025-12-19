//! Checkstyle compatibility tests for FinalLocalVariable rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the FinalLocalVariable check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::FinalLocalVariable;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
}

impl Violation {
    fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Run FinalLocalVariable rule on source and collect violations.
fn check_final_local_variable(source: &str, properties: HashMap<&str, &str>) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = FinalLocalVariable::from_config(&properties);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
/// Returns None if the checkstyle repo is not available.
fn load_finallocalvariable_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/coding/finallocalvariable")
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
            .any(|v| v.line == exp.line && v.column == exp.column);

        if !matched {
            missing.push(exp.clone());
        }
    }

    for actual in violations {
        let matched = expected
            .iter()
            .any(|v| v.line == actual.line && v.column == actual.column);

        if !matched {
            unexpected.push(actual.clone());
        }
    }

    if !missing.is_empty() || !unexpected.is_empty() {
        println!("\n=== Violations Report ===");
        if !missing.is_empty() {
            println!("\nMissing violations:");
            for v in &missing {
                println!("  {}:{}", v.line, v.column);
            }
        }
        if !unexpected.is_empty() {
            println!("\nUnexpected violations:");
            for v in &unexpected {
                println!("  {}:{}", v.line, v.column);
            }
        }
        panic!("Violation mismatch detected");
    }
}

// =============================================================================
// Test: testInputFinalLocalVariableOne
// File: InputFinalLocalVariableOne.java
// Config: validateEnhancedForLoopVariable = (default)false
// Expected violations from checkstyle test:
//   17:13 - Variable 'i' should be declared final
//   17:16 - Variable 'j' should be declared final
//   19:18 - Variable 'runnable' should be declared final
//   29:13 - Variable 'i' should be declared final
//   33:13 - Variable 'z' should be declared final
//   35:16 - Variable 'obj' should be declared final
//   39:16 - Variable 'x' should be declared final
//   45:18 - Variable 'runnable' should be declared final
//   49:21 - Variable 'q' should be declared final
//   65:13 - Variable 'i' should be declared final
//   69:13 - Variable 'z' should be declared final
//   71:16 - Variable 'obj' should be declared final
//   75:16 - Variable 'x' should be declared final
//   83:21 - Variable 'w' should be declared final
//   85:26 - Variable 'runnable' should be declared final
// =============================================================================

#[test]
fn test_input_final_local_variable_one() {
    let Some(source) = load_finallocalvariable_fixture("InputFinalLocalVariableOne.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_local_variable(&source, properties);

    let expected = vec![
        Violation::new(17, 13),
        Violation::new(17, 16),
        Violation::new(19, 18),
        Violation::new(29, 13),
        Violation::new(33, 13),
        Violation::new(35, 16),
        Violation::new(39, 16),
        Violation::new(45, 18),
        Violation::new(49, 21),
        Violation::new(65, 13),
        Violation::new(69, 13),
        Violation::new(71, 16),
        Violation::new(75, 16),
        Violation::new(83, 21),
        Violation::new(85, 26),
    ];

    verify_violations(&violations, &expected);
}

// Minimal test to debug assignment tracking
#[test]
fn test_minimal_assignment_tracking() {
    let source = r#"
public class Test {
    public void test() {
        // Should report: never reassigned
        int a = 0;

        // Should NOT report: has final
        final int b = 1;

        // Should NOT report: incremented
        int c = 0;
        c++;

        // Should NOT report: compound assignment
        int d = 0;
        d += 5;

        // Should NOT report: reassigned
        int e = 0;
        e = 5;
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should only report 'a' at line 5
    let expected = vec![Violation::new(5, 13)];

    verify_violations(&violations, &expected);
}

// Test all forms of assignment operators
#[test]
fn test_all_assignment_operators() {
    let source = r#"
public class Test {
    public void test() {
        // Should report: never reassigned
        int a = 0;

        // Should NOT report: simple assignment
        int b = 0;
        b = 5;

        // Should NOT report: compound assignments
        int c1 = 0;
        c1 += 5;

        int c2 = 0;
        c2 -= 3;

        int c3 = 0;
        c3 *= 2;

        int c4 = 0;
        c4 /= 2;

        int c5 = 0;
        c5 %= 3;

        int c6 = 0;
        c6 &= 1;

        int c7 = 0;
        c7 |= 2;

        int c8 = 0;
        c8 ^= 4;

        int c9 = 0;
        c9 <<= 1;

        int c10 = 0;
        c10 >>= 1;

        int c11 = 0;
        c11 >>>= 1;

        // Should NOT report: increment/decrement
        int d1 = 0;
        d1++;

        int d2 = 0;
        ++d2;

        int d3 = 0;
        d3--;

        int d4 = 0;
        --d4;

        // Should NOT report: assigned multiple times
        int e = 0;
        e = 1;
        e = 2;
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should only report 'a' at line 5
    let expected = vec![Violation::new(5, 13)];

    verify_violations(&violations, &expected);
}

// Test if/else control flow scenarios
#[test]
fn test_if_else_control_flow() {
    let source = r#"
public class Test {
    public void test() {
        // Assigned in both branches - should be final candidate
        int a; // violation at line 5
        if (true) {
            a = 1;
        } else {
            a = 2;
        }

        // Assigned in only one branch - should be final candidate
        int b; // violation at line 13
        if (true) {
            b = 1;
        }

        // Assigned before if, then again in if - NOT a candidate
        int c = 0;
        if (true) {
            c = 1;
        }

        // Assigned in both branches, then again after - NOT a candidate
        int d;
        if (true) {
            d = 1;
        } else {
            d = 2;
        }
        d = 3;

        // Assigned in if and else if - should be final candidate
        int e; // violation at line 34
        if (true) {
            e = 1;
        } else if (true) {
            e = 2;
        } else {
            e = 3;
        }
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    let expected = vec![
        Violation::new(5, 13),  // a
        Violation::new(13, 13), // b
        Violation::new(34, 13), // e
    ];

    verify_violations(&violations, &expected);
}

// Test case from checkstyle: variable assigned in all branches then again later
#[test]
fn test_reassignment_after_branches() {
    let source = r#"
public class Test {
    private void foo7() {
        int index;
        if (true) {
            index = 0;
        }
        else if (true) {
            index = 2;
        }
        else {
            return;
        }
        if (true) {
            index += 1;
        }
    }

    private void simple() {
        int a;
        if (true) {
            a = 1;
        } else {
            a = 2;
        }
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // 'index' is not a candidate because it's reassigned after branches
    // 'a' should be a candidate because it's only assigned in branches
    let expected = vec![
        Violation::new(20, 13), // a
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testFinalLocalVariableSwitchAssignment
// File: InputFinalLocalVariableCheckSwitchAssignment.java
// Config: validateEnhancedForLoopVariable = (default)false
// Expected violations from checkstyle test:
//   21:13 - Variable 'a' should be declared final
//   44:13 - Variable 'b' should be declared final
//   46:21 - Variable 'x' should be declared final
//   72:16 - Variable 'res' should be declared final
//   92:16 - Variable 'res' should be declared final
// =============================================================================

#[test]
fn test_final_local_variable_switch_assignment() {
    let Some(source) =
        load_finallocalvariable_fixture("InputFinalLocalVariableCheckSwitchAssignment.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_local_variable(&source, properties);

    let expected = vec![
        Violation::new(21, 13), // a
        Violation::new(44, 13), // b
        Violation::new(46, 21), // x
        Violation::new(72, 16), // res
        Violation::new(92, 16), // res
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testVariableIsAssignedInsideAndOutsideSwitch
// File: InputFinalLocalVariableAssignedInsideAndOutsideSwitch.java
// Config: validateEnhancedForLoopVariable = (default)false
// Expected violations from checkstyle test:
//   39:13 - Variable 'b' should be declared final
// =============================================================================

#[test]
fn test_variable_is_assigned_inside_and_outside_switch() {
    let Some(source) =
        load_finallocalvariable_fixture("InputFinalLocalVariableAssignedInsideAndOutsideSwitch.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_local_variable(&source, properties);

    let expected = vec![
        Violation::new(39, 13), // b
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testFinalLocalVariableSwitchStatement
// File: InputFinalLocalVariableSwitchStatement.java
// Config: validateEnhancedForLoopVariable = (default)false
// Expected violations from checkstyle test: (none)
// =============================================================================

#[test]
fn test_final_local_variable_switch_statement() {
    let Some(source) =
        load_finallocalvariable_fixture("InputFinalLocalVariableSwitchStatement.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_local_variable(&source, properties);

    // No violations expected - variable x is assigned in all cases but falls through
    let expected = vec![];

    verify_violations(&violations, &expected);
}

// Test switch with all branches assigning
#[test]
fn test_switch_all_branches_assign() {
    let source = r#"
public class Test {
    void test(int x) {
        // Should report: assigned in all branches (including default)
        int a;
        switch (x) {
            case 1:
                a = 10;
                break;
            case 2:
                a = 20;
                break;
            default:
                a = 30;
                break;
        }
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should report 'a' at line 5
    let expected = vec![Violation::new(5, 13)];

    verify_violations(&violations, &expected);
}

// Test switch with some branches assigning
#[test]
fn test_switch_some_branches_assign() {
    let source = r#"
public class Test {
    void test(int x) {
        // SHOULD report: assigned in some branches, never reassigned
        int a;
        switch (x) {
            case 1:
                a = 10;
                break;
            case 2:
                // no assignment
                break;
            default:
                a = 30;
                break;
        }
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should report 'a' - assigned in switch (even though not all branches)
    // because it's never reassigned after the first assignment in each path
    let expected = vec![Violation::new(5, 13)];

    verify_violations(&violations, &expected);
}

// Test switch with assignment after branches
#[test]
fn test_switch_reassignment_after_branches() {
    let source = r#"
public class Test {
    void test(int x) {
        // Should NOT report: assigned in branches then reassigned
        int a;
        switch (x) {
            case 1:
                a = 10;
                break;
            case 2:
                a = 20;
                break;
            default:
                a = 30;
                break;
        }
        a = 40;
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should not report 'a' - reassigned after switch
    let expected = vec![];

    verify_violations(&violations, &expected);
}

// Test switch expression (arrow syntax)
#[test]
fn test_switch_expression_arrow() {
    let source = r#"
public class Test {
    void test(int x) {
        // Should report: result of switch expression never reassigned
        int a = switch (x) {
            case 1 -> 10;
            case 2 -> 20;
            default -> 30;
        };

        // Should NOT report: reassigned
        int b = switch (x) {
            case 1 -> 100;
            default -> 200;
        };
        b = 50;
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should report 'a' at line 5
    let expected = vec![Violation::new(5, 13)];

    verify_violations(&violations, &expected);
}

// Test switch rule with assignment
#[test]
fn test_switch_rule_assignment() {
    let source = r#"
public class Test {
    void test(int x) {
        // Should report: assigned in all arrow cases
        String res;
        switch (x) {
            case 1 -> res = "A";
            case 2 -> res = "B";
            default -> res = "C";
        }
    }
}
"#;

    let properties = HashMap::new();
    let violations = check_final_local_variable(source, properties);

    // Should report 'res' at line 5
    let expected = vec![Violation::new(5, 16)];

    verify_violations(&violations, &expected);
}
