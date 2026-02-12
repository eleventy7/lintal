//! RegexpSinglelineJava checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::RegexpSinglelineJava;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use std::collections::HashMap;
use test_harness::TestResult;

/// Run the RegexpSinglelineJava rule on source code and return violation lines.
fn check_regexp(source: &str, rule: &RegexpSinglelineJava) -> Vec<usize> {
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

    let n_above_re = Regex::new(r"//\s*violation\s+(\d+)\s+lines?\s+above").unwrap();
    let above_re = Regex::new(r"//\s*violation\s+above").unwrap();
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    let inline_re = Regex::new(r"//\s*violation\b").unwrap();

    for (i, line) in source.lines().enumerate() {
        let line_num = i + 1;

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

/// Parse config from the header block of a checkstyle test file.
/// Returns owned strings because we need to unescape Java-style backslash sequences.
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

    for line in block.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("RegexpSinglelineJava") {
            continue;
        }
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            // Trim only leading whitespace to preserve trailing content (e.g., trailing spaces in format)
            let value = line[eq_pos + 1..].trim_start();

            // Skip (default) values
            if value.starts_with("(default)") {
                continue;
            }

            // Unescape Java-style double backslashes to single backslashes.
            // Test fixtures use Java string escaping: \\\\ → \\, \\. → \.
            let unescaped = value.replace("\\\\", "\\");

            props.insert(key, unescaped);
        }
    }

    props
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::regexp_test_input("regexpsinglelinejava", file_name)?;
    std::fs::read_to_string(&path).ok()
}

fn build_rule_from_source(source: &str) -> RegexpSinglelineJava {
    let config = parse_config_from_header(source);
    let mut props: Properties = HashMap::new();
    for (k, v) in &config {
        props.insert(k.as_str(), v.as_str());
    }
    RegexpSinglelineJava::from_config(&props)
}

// Semantic tests (format = System\.(out)|(err)\.print(ln)?\()

#[test]
fn test_semantic1() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaSemantic.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_semantic2() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaSemantic2.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_semantic3_ignore_case() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic3.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaSemantic3.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_semantic4_no_ignore_case() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic4.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: UPPERCASE format without ignoreCase - should have NO violations
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "Case-sensitive uppercase pattern should not match lowercase text"
    );
}

#[test]
fn test_semantic5_minimum_maximum() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic5.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="Test case file", min=1, max=1000 - should have no violations
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(actual.is_empty(), "File should satisfy minimum requirement");
}

#[test]
fn test_semantic6_package_match() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic6.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="package", min=1, max=1000 - should have no violations
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "File with 'package' line should satisfy minimum requirement"
    );
}

#[test]
fn test_semantic7_minimum_not_met() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic7.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="This\stext is not in the file", min=1, max=1000
    // Text not in file, so minimum not met - violation at line 1
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaSemantic7.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_semantic8_default_pattern() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaSemantic8.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: default pattern "$." with custom message - no matches expected
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(actual.is_empty(), "Default pattern '$.' should never match");
}

// Trailing comment tests (ignoreComments = true)

#[test]
fn test_trailing_comment_ignore_comments() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="don't use trailing comments", ignoreComments=true
    // Text only in comments, so no violations
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "With ignoreComments=true, text only in comments should not match"
    );
}

#[test]
fn test_trailing_comment2_no_ignore() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="don't\suse trailing comments", ignoreComments=false
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment2.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_trailing_comment3_ignore_cstyle() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment3.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="c-style 1", ignoreComments=true
    // Text only in comments, so no violations
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "With ignoreComments=true, text only in c-style comments should not match"
    );
}

#[test]
fn test_trailing_comment4_cstyle_no_ignore() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment4.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="c-style\s1", ignoreComments=false
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment4.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_trailing_comment5_cstyle2() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment5.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="c-style 2", ignoreComments=true
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "With ignoreComments=true, text only in c-style comments should not match"
    );
}

#[test]
fn test_trailing_comment6_multiline() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment6.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="Let's check multi-line comments", ignoreComments=true
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "With ignoreComments=true, text in multi-line comments should not match"
    );
}

#[test]
fn test_trailing_comment7_method_comment() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment7.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="long ms /", ignoreComments=true
    let rule = build_rule_from_source(&source);
    let actual = check_regexp(&source, &rule);

    assert!(
        actual.is_empty(),
        "With ignoreComments=true, text adjacent to inline comments should not match"
    );
}

#[test]
fn test_trailing_comment8_int_z() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment8.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="int z", ignoreComments=true
    // "int z" appears in code (not in comments), should still match
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment8.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_trailing_comment9_int_y() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment9.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="int y", ignoreComments=true
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment9.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_trailing_comment10_long_ms() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment10.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="long ms  " (with trailing spaces), ignoreComments=true
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment10.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(100.0);
}

#[test]
fn test_trailing_comment11_trailing_whitespace() {
    let Some(source) = load_fixture("InputRegexpSinglelineJavaTrailingComment11.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Config: format="\\s+$", ignoreComments=true
    let rule = build_rule_from_source(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_regexp(&source, &rule);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputRegexpSinglelineJavaTrailingComment11.java");

    result.assert_no_false_positives();
    result.assert_detection_rate(90.0);
}
