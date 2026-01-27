//! StringLiteralEquality checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::StringLiteralEquality;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the StringLiteralEquality rule on source code and return violation lines.
fn check_string_literal_equality(source: &str) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = StringLiteralEquality;
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
/// Supports formats like:
/// - `// violation` (inline)
/// - `// violation 'message'`
/// - `// violation above` (violation is on previous line)
/// - `// violation below` (violation is on next line)
/// - `// violation 3 lines above` (violation N lines above)
fn parse_expected_violations(source: &str) -> Vec<usize> {
    let mut violations = vec![];

    // Match patterns like "// violation" with optional message
    let inline_re = Regex::new(r"//\s*violation\s*(?:'[^']*')?(?:\s|$)").unwrap();
    // Match "violation above" pattern
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    // Match "violation below" pattern
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    // Match "violation N lines above" pattern
    let n_above_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

        // Check for "N lines above" first (more specific pattern)
        if let Some(caps) = n_above_re.captures(line) {
            if let Some(n) = caps.get(1).and_then(|m| m.as_str().parse::<usize>().ok())
                && line_num > n
            {
                violations.push(line_num - n);
            }
        } else if above_re.is_match(line) {
            // Simple "above" means previous line
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
    let path = checkstyle_repo::coding_test_input("stringliteralequality", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_string_literal_equality() {
    let Some(source) = load_fixture("InputStringLiteralEquality.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_string_literal_equality(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputStringLiteralEquality.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_string_literal_equality_check() {
    let Some(source) = load_fixture("InputStringLiteralEqualityCheck.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_string_literal_equality(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputStringLiteralEqualityCheck.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_string_literal_equality_text_blocks() {
    let Some(source) = load_fixture("InputStringLiteralEqualityCheckTextBlocks.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_string_literal_equality(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputStringLiteralEqualityCheckTextBlocks.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_concatenated_string() {
    let Some(source) = load_fixture("InputStringLiteralEqualityConcatenatedString.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_string_literal_equality(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputStringLiteralEqualityConcatenatedString.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_concatenated_text_blocks() {
    let Some(source) = load_fixture("InputStringLiteralEqualityConcatenatedTextBlocks.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_string_literal_equality(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputStringLiteralEqualityConcatenatedTextBlocks.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

// Unit tests for specific cases
#[test]
fn test_basic_string_equality() {
    let source = r#"
class Test {
    void foo(String name) {
        if (name == "Lars") {}  // violation
    }
}
"#;
    let violations = check_string_literal_equality(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0], 4);
}

#[test]
fn test_reversed_operands() {
    let source = r#"
class Test {
    void foo(String name) {
        if ("Oleg" == name) {}  // violation
    }
}
"#;
    let violations = check_string_literal_equality(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0], 4);
}

#[test]
fn test_two_literals() {
    let source = r#"
class Test {
    void foo() {
        if ("Oliver" == "Oliver") {}  // violation
    }
}
"#;
    let violations = check_string_literal_equality(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0], 4);
}

#[test]
fn test_variable_comparison_no_violation() {
    let source = r#"
class Test {
    void foo(String name) {
        String compare = "Rick";
        if (name == compare) {}  // no violation - two variables
    }
}
"#;
    let violations = check_string_literal_equality(source);
    assert!(
        violations.is_empty(),
        "Comparing two variables should not trigger violation"
    );
}

#[test]
fn test_method_call_result_no_violation() {
    let source = r#"
class Test {
    void foo() {
        if ("Rick".toUpperCase() == "Rick".toLowerCase()) {}  // no violation
    }
}
"#;
    let violations = check_string_literal_equality(source);
    // This should NOT trigger a violation because the operands are method call results,
    // not string literals. The string literals are inside method calls.
    assert!(
        violations.is_empty(),
        "Method calls should not trigger violation"
    );
}

#[test]
fn test_not_equals() {
    let source = r#"
class Test {
    void foo(String s) {
        if (s != "foo") {}  // violation
    }
}
"#;
    let violations = check_string_literal_equality(source);
    assert_eq!(violations.len(), 1);
}
