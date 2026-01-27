//! DefaultComesLast checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::DefaultComesLast;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the DefaultComesLast rule on source code and return violation lines.
fn check_default_comes_last(source: &str, skip_if_last_and_shared: bool) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = DefaultComesLast {
        skip_if_last_and_shared_with_case: skip_if_last_and_shared,
    };
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

    // Match patterns like "// violation 'message'"
    let inline_re = Regex::new(r"//\s*violation\s+'[^']*'").unwrap();
    // Match "violation above" pattern
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    // Match "violation below" pattern
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    // Match "violation N lines above" pattern
    let n_above_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

        // Skip lines that say "No violation" or similar
        if line.contains("No violation") || line.contains("no violation") {
            continue;
        }

        // Check for "N lines above" first
        if let Some(caps) = n_above_re.captures(line) {
            if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok())
                && line_num > n
            {
                violations.push(line_num - n);
            }
        } else if above_re.is_match(line) {
            if line_num > 1 {
                violations.push(line_num - 1);
            }
        } else if below_re.is_match(line) {
            violations.push(line_num + 1);
        } else if inline_re.is_match(line) {
            violations.push(line_num);
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::coding_test_input("defaultcomeslast", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_default_comes_last_one() {
    let Some(source) = load_fixture("InputDefaultComesLastOne.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_default_comes_last(&source, false);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDefaultComesLastOne.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_input_default_comes_last_two() {
    let Some(source) = load_fixture("InputDefaultComesLastTwo.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_default_comes_last(&source, false);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDefaultComesLastTwo.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_input_skip_if_last_and_shared_one() {
    let Some(source) = load_fixture("InputDefaultComesLastSkipIfLastAndSharedWithCaseOne.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_default_comes_last(&source, true);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDefaultComesLastSkipIfLastAndSharedWithCaseOne.java (skip=true)");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_input_skip_if_last_and_shared_two() {
    let Some(source) = load_fixture("InputDefaultComesLastSkipIfLastAndSharedWithCaseTwo.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_default_comes_last(&source, true);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDefaultComesLastSkipIfLastAndSharedWithCaseTwo.java (skip=true)");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_input_switch_expressions() {
    let Some(source) = load_fixture("InputDefaultComesLastSwitchExpressions.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_default_comes_last(&source, false);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDefaultComesLastSwitchExpressions.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_input_default_methods_in_interface_no_violations() {
    let Some(source) = load_fixture("InputDefaultComesLastDefaultMethodsInInterface.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // This file has interface default methods which should NOT trigger violations
    let actual = check_default_comes_last(&source, false);

    assert!(
        actual.is_empty(),
        "Interface default methods should not trigger violations, got: {:?}",
        actual
    );
}

#[test]
fn test_input_default_methods_in_interface2_no_violations() {
    let Some(source) = load_fixture("InputDefaultComesLastDefaultMethodsInInterface2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // This file has interface default methods which should NOT trigger violations
    let actual = check_default_comes_last(&source, false);

    assert!(
        actual.is_empty(),
        "Interface default methods should not trigger violations, got: {:?}",
        actual
    );
}

// Unit tests for specific cases
#[test]
fn test_default_last_no_violation() {
    let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            case 2: break;
            default: break;
        }
    }
}
"#;
    let violations = check_default_comes_last(source, false);
    assert!(violations.is_empty(), "Default is last - no violation");
}

#[test]
fn test_default_not_last_violation() {
    let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            default: break;
            case 2: break;
        }
    }
}
"#;
    let violations = check_default_comes_last(source, false);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_no_switch_default_no_violation() {
    let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            case 2: break;
        }
    }
}
"#;
    let violations = check_default_comes_last(source, false);
    assert!(violations.is_empty(), "No default - no violation");
}

#[test]
fn test_interface_default_method_no_violation() {
    let source = r#"
interface Test {
    default void method() {
        System.out.println("default method");
    }
}
"#;
    let violations = check_default_comes_last(source, false);
    assert!(
        violations.is_empty(),
        "Interface default methods should not trigger violations"
    );
}
