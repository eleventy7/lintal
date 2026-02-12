//! NestedTryDepth checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::NestedTryDepth;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the NestedTryDepth rule on source code and return violation lines.
fn check_nested_try_depth(source: &str, max: usize) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = NestedTryDepth { max };
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
    let path = checkstyle_repo::coding_test_input("nestedtrydepth", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_nested_try_depth() {
    let Some(source) = load_fixture("InputNestedTryDepth.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = parse_expected_violations(&source);
    let actual = check_nested_try_depth(&source, 1);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputNestedTryDepth.java (max=1)");

    result.assert_no_false_positives();
    result.assert_detection_rate(80.0);
}

#[test]
fn test_from_config_default() {
    let props = Properties::new();
    let rule = NestedTryDepth::from_config(&props);
    assert_eq!(rule.max, 1);
}

#[test]
fn test_from_config_custom() {
    let mut props = Properties::new();
    props.insert("max", "3");
    let rule = NestedTryDepth::from_config(&props);
    assert_eq!(rule.max, 3);
}

#[test]
fn test_single_try_no_violation() {
    let source = r#"
class Foo {
    void method() {
        try {
            doSomething();
        } catch (Exception e) {
        }
    }
}
"#;
    let violations = check_nested_try_depth(source, 1);
    assert!(violations.is_empty());
}

#[test]
fn test_nested_try_exceeds_limit() {
    let source = r#"
class Foo {
    void method() {
        try {
            try {
                try {
                    doSomething();
                } catch (Exception e) {
                }
            } catch (Exception e) {
            }
        } catch (Exception e) {
        }
    }
}
"#;
    let violations = check_nested_try_depth(source, 1);
    assert_eq!(violations.len(), 1);
}
