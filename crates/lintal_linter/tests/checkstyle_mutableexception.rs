//! MutableException checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::MutableException;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the MutableException rule on source code and return violation lines.
fn check_mutable_exception(source: &str) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = MutableException::default();
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
    let path = checkstyle_repo::design_test_input("mutableexception", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_mutable_exception() {
    let Some(source) = load_fixture("InputMutableException.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_mutable_exception(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputMutableException.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

// Unit tests for specific cases
#[test]
fn test_mutable_field_in_exception() {
    let source = r#"
class MyException extends Exception {
    private int errorCode;
}
"#;
    let violations = check_mutable_exception(source);
    assert_eq!(
        violations.len(),
        1,
        "Non-final field in exception class should be a violation"
    );
}

#[test]
fn test_final_field_in_exception_no_violation() {
    let source = r#"
class MyException extends Exception {
    private final int errorCode = 0;
}
"#;
    let violations = check_mutable_exception(source);
    assert!(
        violations.is_empty(),
        "Final field in exception class should not be a violation"
    );
}

#[test]
fn test_non_exception_class_no_violation() {
    let source = r#"
class RegularClass {
    private int value;
}
"#;
    let violations = check_mutable_exception(source);
    assert!(
        violations.is_empty(),
        "Non-exception class with mutable fields should not be a violation"
    );
}
