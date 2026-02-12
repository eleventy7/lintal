//! InnerAssignment checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::InnerAssignment;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the InnerAssignment rule on source code and return violation lines.
fn check_inner_assignment(source: &str) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = InnerAssignment;
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push(loc.line.get());
        }
    }

    violations
}

/// Parse expected violations from checkstyle test file comments.
fn parse_expected_violations(source: &str) -> Vec<usize> {
    let mut violations = vec![];

    let n_above_lines_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();
    let n_violations_above_re = Regex::new(r"//\s*(\d+)\s+violations?\s+above").unwrap();
    let n_violations_below_re = Regex::new(r"//\s*(\d+)\s+violations?\s+below").unwrap();
    let n_violations_re = Regex::new(r"//\s*(\d+)\s+violations?\b").unwrap();
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    let inline_re = Regex::new(r"//\s*violation\b").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

        if let Some(caps) = n_above_lines_re.captures(line) {
            if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok())
                && line_num > n
            {
                violations.push(line_num - n);
            }
        } else if let Some(caps) = n_violations_above_re.captures(line) {
            if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok())
                && line_num > 1
            {
                for _ in 0..n {
                    violations.push(line_num - 1);
                }
            }
        } else if let Some(caps) = n_violations_below_re.captures(line) {
            if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok()) {
                for _ in 0..n {
                    violations.push(line_num + 1);
                }
            }
        } else if above_re.is_match(line) {
            if line_num > 1 {
                violations.push(line_num - 1);
            }
        } else if below_re.is_match(line) {
            violations.push(line_num + 1);
        } else if let Some(caps) = n_violations_re.captures(line) {
            if !line.contains("above")
                && !line.contains("below")
                && let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok())
            {
                for _ in 0..n {
                    violations.push(line_num);
                }
            }
        } else if inline_re.is_match(line) {
            violations.push(line_num);
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::coding_test_input("innerassignment", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_inner_assignment() {
    let Some(source) = load_fixture("InputInnerAssignment.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_inner_assignment(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputInnerAssignment.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_input_inner_assignment_lambda_expressions() {
    let Some(source) = load_fixture("InputInnerAssignmentLambdaExpressions.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_inner_assignment(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputInnerAssignmentLambdaExpressions.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

// Unit tests for specific cases
#[test]
fn test_assignment_in_if_condition() {
    let source = r#"
class Test {
    void foo() {
        int a;
        if ((a = 5) > 0) {}
    }
}
"#;
    let violations = check_inner_assignment(source);
    assert_eq!(
        violations.len(),
        1,
        "Assignment inside if condition should be a violation"
    );
}

#[test]
fn test_standalone_assignment_no_violation() {
    let source = r#"
class Test {
    void foo() {
        int a;
        a = 5;
    }
}
"#;
    let violations = check_inner_assignment(source);
    assert!(
        violations.is_empty(),
        "Standalone assignment should not trigger violation"
    );
}

#[test]
fn test_assignment_in_while_condition_no_violation() {
    let source = r#"
class Test {
    void foo(java.io.InputStream in) throws Exception {
        int b;
        while ((b = in.read()) != -1) {}
    }
}
"#;
    let violations = check_inner_assignment(source);
    // Assignments in while conditions are a common idiom - checkstyle does not flag them
    assert!(
        violations.is_empty(),
        "Assignment inside while condition should not be a violation"
    );
}
