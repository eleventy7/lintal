//! MissingSwitchDefault checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::MissingSwitchDefault;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the MissingSwitchDefault rule on source code and return violation lines.
fn check_missing_switch_default(source: &str) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = MissingSwitchDefault;
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
    let inline_re = Regex::new(r"//\s*violation,?\s+'[^']*'").unwrap();
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    let n_above_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;
        if line.contains("No violation") || line.contains("no violation") {
            continue;
        }
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
    let path = checkstyle_repo::coding_test_input("missingswitchdefault", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_missing_switch_default() {
    let Some(source) = load_fixture("InputMissingSwitchDefault.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_missing_switch_default(&source);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputMissingSwitchDefault.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_switch_with_default_no_violation() {
    let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1: break;
            default: break;
        }
    }
}
"#;
    let violations = check_missing_switch_default(source);
    assert!(violations.is_empty());
}

#[test]
fn test_switch_without_default_violation() {
    let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1: break;
            case 2: break;
        }
    }
}
"#;
    let violations = check_missing_switch_default(source);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_arrow_switch_with_default_no_violation() {
    let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1 -> System.out.println(1);
            default -> System.out.println(0);
        }
    }
}
"#;
    let violations = check_missing_switch_default(source);
    assert!(violations.is_empty());
}

#[test]
fn test_arrow_switch_without_default_violation() {
    let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1 -> System.out.println(1);
            case 2 -> System.out.println(2);
        }
    }
}
"#;
    let violations = check_missing_switch_default(source);
    assert_eq!(violations.len(), 1);
}
