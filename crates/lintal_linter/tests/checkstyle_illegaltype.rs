//! IllegalType checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::IllegalType;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use std::collections::HashMap;
use test_harness::TestResult;

/// Run the IllegalType rule on source code and return violation lines.
fn check_illegal_type(source: &str, rule: &IllegalType) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
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

/// Parse config from the header block of a checkstyle test file.
fn parse_config_from_header(source: &str) -> HashMap<String, String> {
    let mut props = HashMap::new();

    // Find the first /* ... */ block
    let Some(start) = source.find("/*") else {
        return props;
    };
    let Some(end) = source[start..].find("*/") else {
        return props;
    };
    let block = &source[start + 2..start + end];

    // Handle multi-line values with backslash continuation
    let mut full_block = String::new();
    for line in block.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_suffix('\\') {
            full_block.push_str(stripped);
        } else {
            full_block.push_str(trimmed);
            full_block.push('\n');
        }
    }

    for line in full_block.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("IllegalType")
            || line.starts_with("Config:")
            || line.starts_with("*")
        {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let value = line[eq_pos + 1..].trim().to_string();

            // Skip (default) values and tokens (we don't use them)
            if value.starts_with("(default)") || key == "tokens" {
                continue;
            }

            props.insert(key, value);
        }
    }

    props
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::coding_test_input("illegaltype", file_name)?;
    std::fs::read_to_string(&path).ok()
}

fn build_rule_from_source(source: &str) -> IllegalType {
    let config = parse_config_from_header(source);
    let mut props: Properties = HashMap::new();
    for (k, v) in &config {
        props.insert(k.as_str(), v.as_str());
    }
    IllegalType::from_config(&props)
}

#[test]
fn test_input_defaults() {
    let Some(source) = load_fixture("InputIllegalTypeTestDefaults.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestDefaults.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_input_abstract_class_names_true() {
    let Some(source) = load_fixture("InputIllegalTypeTestAbstractClassNamesTrue.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestAbstractClassNamesTrue.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_abstract_class_names_false() {
    let Some(source) = load_fixture("InputIllegalTypeTestAbstractClassNamesFalse.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestAbstractClassNamesFalse.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_ignore_method_names() {
    let Some(source) = load_fixture("InputIllegalTypeTestIgnoreMethodNames.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestIgnoreMethodNames.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_member_modifiers() {
    let Some(source) = load_fixture("InputIllegalTypeTestMemberModifiers.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestMemberModifiers.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_extends_implements() {
    let Some(source) = load_fixture("InputIllegalTypeTestExtendsImplements.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestExtendsImplements.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_generics() {
    let Some(source) = load_fixture("InputIllegalTypeTestGenerics.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeTestGenerics.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_input_greg_cal() {
    let Some(source) = load_fixture("InputIllegalTypeGregCal.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // This is a helper class with no config header
    let rule = IllegalType::default();
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeGregCal.java");

    result.assert_no_false_positives();
}

#[test]
fn test_input_similar_class_name() {
    let Some(source) = load_fixture("InputIllegalTypeSimilarClassName.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_illegal_type(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputIllegalTypeSimilarClassName.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

// Unit tests

#[test]
fn test_basic_illegal_type() {
    let source = r#"
class Test {
    private HashMap<String, String> map;
    private TreeSet<String> set;
}
"#;
    let rule = IllegalType::default();
    let violations = check_illegal_type(source, &rule);
    assert_eq!(violations.len(), 2);
    assert_eq!(violations[0], 3);
    assert_eq!(violations[1], 4);
}

#[test]
fn test_override_skipped() {
    let source = r#"
class Test {
    @Override
    public HashMap<String, String> foo() { return null; }
}
"#;
    let rule = IllegalType::default();
    let violations = check_illegal_type(source, &rule);
    assert!(violations.is_empty());
}

#[test]
fn test_ignored_method() {
    let source = r#"
class Test {
    private TreeSet<String> getEnvironment() { return null; }
}
"#;
    let rule = IllegalType::default();
    let violations = check_illegal_type(source, &rule);
    assert!(violations.is_empty());
}

#[test]
fn test_qualified_name() {
    let source = r#"
class Test {
    private java.util.TreeSet table1() { return null; }
}
"#;
    let rule = IllegalType::default();
    let violations = check_illegal_type(source, &rule);
    assert_eq!(violations.len(), 1);
}
