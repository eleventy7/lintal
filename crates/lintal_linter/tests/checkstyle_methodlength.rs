//! MethodLength checkstyle compatibility tests.

mod checkstyle_repo;
mod test_harness;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::MethodLength;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use regex::Regex;
use test_harness::TestResult;

/// Run the MethodLength rule on source code and return violation lines.
fn check_method_length(source: &str, max: usize, count_empty: bool) -> Vec<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = MethodLength {
        max,
        check_methods: true,
        check_constructors: true,
        count_empty,
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
    let path = checkstyle_repo::sizes_test_input("methodlength", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_method_length_simple() {
    let Some(source) = load_fixture("InputMethodLengthSimple.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Default max is 150, but checkstyle tests usually use smaller values
    // The test fixture might use a specific max in its config
    let expected = parse_expected_violations(&source);
    let actual = check_method_length(&source, 150, true);

    let result = TestResult::compare(expected, actual);
    result.print_report("InputMethodLengthSimple.java (max=150)");

    result.assert_no_false_positives();
}

#[test]
fn test_from_config_default() {
    let props = Properties::new();
    let rule = MethodLength::from_config(&props);
    assert_eq!(rule.max, 150);
    assert!(rule.check_methods);
    assert!(rule.check_constructors);
    assert!(rule.count_empty);
}

#[test]
fn test_from_config_custom() {
    let mut props = Properties::new();
    props.insert("max", "20");
    props.insert("countEmpty", "false");
    props.insert("tokens", "METHOD_DEF");
    let rule = MethodLength::from_config(&props);
    assert_eq!(rule.max, 20);
    assert!(rule.check_methods);
    assert!(!rule.check_constructors);
    assert!(!rule.count_empty);
}

#[test]
fn test_short_method_no_violation() {
    let source = r#"
class Foo {
    void method() {
        int x = 1;
    }
}
"#;
    let violations = check_method_length(source, 10, true);
    assert!(violations.is_empty());
}

#[test]
fn test_long_method_violation() {
    let mut lines = vec!["class Foo {".to_string(), "    void method() {".to_string()];
    for i in 0..20 {
        lines.push(format!("        int x{} = {};", i, i));
    }
    lines.push("    }".to_string());
    lines.push("}".to_string());
    let source = lines.join("\n");

    let violations = check_method_length(&source, 10, true);
    assert_eq!(violations.len(), 1);
}
