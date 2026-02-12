//! DescendantToken checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::DescendantToken;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use std::collections::HashMap;
use test_harness::TestResult;

/// Run the DescendantToken rule on source code and return violation lines.
fn check_descendant_token(source: &str, rule: &DescendantToken) -> Vec<usize> {
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
fn parse_config_from_header(source: &str) -> HashMap<&str, &str> {
    let mut props = HashMap::new();

    // Find the first /* ... */ block
    let Some(start) = source.find("/*") else {
        return props;
    };
    let Some(end) = source[start..].find("*/") else {
        return props;
    };
    let block = &source[start + 2..start + end];

    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("DescendantToken") {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim();

            // Skip (default) values
            if value.starts_with("(default)") {
                continue;
            }

            props.insert(key, value);
        }
    }

    props
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::descendanttoken_test_input(file_name)?;
    std::fs::read_to_string(&path).ok()
}

fn build_rule_from_source(source: &str) -> DescendantToken {
    let config = parse_config_from_header(source);
    let mut props: Properties = HashMap::new();
    for (k, v) in &config {
        props.insert(k, v);
    }
    DescendantToken::from_config(&props)
}

#[test]
fn test_return_from_finally() {
    let Some(source) = load_fixture("InputDescendantTokenReturnFromFinally.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenReturnFromFinally.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_missing_switch_default() {
    let Some(source) = load_fixture("InputDescendantTokenMissingSwitchDefault.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenMissingSwitchDefault.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_string_literal_equality() {
    let Some(source) = load_fixture("InputDescendantTokenStringLiteralEquality.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenStringLiteralEquality.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_return_from_catch() {
    let Some(source) = load_fixture("InputDescendantTokenReturnFromCatch.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenReturnFromCatch.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_empty_statement() {
    let Some(source) = load_fixture("InputDescendantTokenEmptyStatement.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenEmptyStatement.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_illegal_tokens() {
    let Some(source) = load_fixture("InputDescendantTokenIllegalTokens.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: default (no tokens configured), no violations expected
    let rule = build_rule_from_source(&source);
    let actual = check_descendant_token(&source, &rule);

    assert!(
        actual.is_empty(),
        "Default config with no tokens should produce no violations"
    );
}

#[test]
fn test_illegal_tokens2_native() {
    let Some(source) = load_fixture("InputDescendantTokenIllegalTokens2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenIllegalTokens2.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_illegal_tokens3_native_message() {
    let Some(source) = load_fixture("InputDescendantTokenIllegalTokens3.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenIllegalTokens3.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_illegal_tokens7_self_match() {
    let Some(source) = load_fixture("InputDescendantTokenIllegalTokens7.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenIllegalTokens7.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_return_from_finally5_sum_minimum() {
    let Some(source) = load_fixture("InputDescendantTokenReturnFromFinally5.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenReturnFromFinally5.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

#[test]
fn test_return_from_finally6_custom_message() {
    let Some(source) = load_fixture("InputDescendantTokenReturnFromFinally6.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_descendant_token(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputDescendantTokenReturnFromFinally6.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}

// Unit tests

#[test]
fn test_unit_return_from_finally() {
    let source = r#"
class Test {
    public void foo() {
        try {
            System.currentTimeMillis();
        } finally {
            return;
        }
    }
}
"#;

    let mut props: Properties = HashMap::new();
    props.insert("tokens", "LITERAL_FINALLY");
    props.insert("limitedTokens", "LITERAL_RETURN");
    props.insert("maximumNumber", "0");
    props.insert("maximumMessage", "Return from finally is not allowed.");
    let rule = DescendantToken::from_config(&props);

    let violations = check_descendant_token(source, &rule);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_unit_switch_with_default() {
    let source = r#"
class Test {
    public void foo() {
        int i = 1;
        switch (i) {
            case 1: break;
            default: return;
        }
    }
}
"#;

    let mut props: Properties = HashMap::new();
    props.insert("tokens", "LITERAL_SWITCH");
    props.insert("limitedTokens", "LITERAL_DEFAULT");
    props.insert("minimumNumber", "1");
    props.insert("maximumDepth", "2");
    let rule = DescendantToken::from_config(&props);

    let violations = check_descendant_token(source, &rule);
    assert!(violations.is_empty());
}

#[test]
fn test_unit_switch_without_default() {
    let source = r#"
class Test {
    public void foo() {
        int i = 1;
        switch (i) {
            case 1: break;
            case 2: break;
        }
    }
}
"#;

    let mut props: Properties = HashMap::new();
    props.insert("tokens", "LITERAL_SWITCH");
    props.insert("limitedTokens", "LITERAL_DEFAULT");
    props.insert("minimumNumber", "1");
    props.insert("maximumDepth", "2");
    let rule = DescendantToken::from_config(&props);

    let violations = check_descendant_token(source, &rule);
    assert_eq!(violations.len(), 1);
}
