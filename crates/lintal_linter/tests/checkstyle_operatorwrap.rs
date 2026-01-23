//! OperatorWrap checkstyle compatibility tests.
//!
//! This test harness validates lintal's OperatorWrap implementation against
//! checkstyle's own test fixtures. It parses expected violations from comments
//! in the test files and reports:
//! - Correct matches: violations we found that checkstyle expects
//! - Missing matches: violations checkstyle expects but we missed
//! - False positives: violations we report that checkstyle doesn't expect

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::whitespace::operator_wrap::{
    OperatorWrap, OperatorWrapToken, WrapOption,
};
use lintal_linter::{CheckContext, Rule};
use regex::Regex;
use std::collections::HashSet;

/// Expected violation parsed from checkstyle test file comments.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedViolation {
    line: usize,
    operator: Option<String>,
}

/// Actual violation found by our implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ActualViolation {
    line: usize,
    column: usize,
    operator: String,
    message: String,
}

/// Configuration parsed from checkstyle test file header.
#[derive(Debug, Clone)]
struct TestConfig {
    option: WrapOption,
    tokens: HashSet<OperatorWrapToken>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            option: WrapOption::Nl,
            tokens: OperatorWrapToken::default_tokens(),
        }
    }
}

/// Results of running a single test fixture.
#[derive(Debug)]
struct TestResult {
    file_name: String,
    config: TestConfig,
    expected: Vec<ExpectedViolation>,
    actual: Vec<ActualViolation>,
    correct: Vec<(ExpectedViolation, ActualViolation)>,
    missing: Vec<ExpectedViolation>,
    false_positives: Vec<ActualViolation>,
}

impl TestResult {
    fn print_summary(&self) {
        println!("\n=== {} ===", self.file_name);
        println!(
            "Config: option={:?}, tokens={} configured",
            self.config.option,
            self.config.tokens.len()
        );
        println!(
            "Expected: {}, Found: {}, Correct: {}, Missing: {}, False Positives: {}",
            self.expected.len(),
            self.actual.len(),
            self.correct.len(),
            self.missing.len(),
            self.false_positives.len()
        );

        if !self.missing.is_empty() {
            println!("\nMissing violations:");
            for v in &self.missing {
                println!(
                    "  Line {}: expected {:?}",
                    v.line,
                    v.operator.as_deref().unwrap_or("?")
                );
            }
        }

        if !self.false_positives.is_empty() {
            println!("\nFalse positives:");
            for v in &self.false_positives {
                println!(
                    "  Line {}:{}: {} - {}",
                    v.line, v.column, v.operator, v.message
                );
            }
        }
    }

    fn is_perfect(&self) -> bool {
        self.missing.is_empty() && self.false_positives.is_empty()
    }
}

/// Parse expected violations from checkstyle test file comments.
/// Looks for patterns like:
/// - `// violation ''&&' should be on a new line.'`
/// - `// violation ''\+' should be on a new line.'`
/// - `// violation below, ''instanceof' should be on a new line.'`
fn parse_expected_violations(source: &str) -> Vec<ExpectedViolation> {
    let mut violations = vec![];

    // Pattern for inline violations: // violation '...'
    let inline_re = Regex::new(r"//\s*violation\s+'[^']*'").unwrap();
    // Pattern for "violation below" comments
    let below_re = Regex::new(r"//\s*violation\s+below").unwrap();
    // Pattern to extract operator from message like ''&&' should be'
    let op_re = Regex::new(r"''([^']+)'").unwrap();

    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let line_num = i + 1;

        if below_re.is_match(line) {
            // Violation is on the next line
            let operator = op_re
                .captures(line)
                .map(|c| c.get(1).unwrap().as_str().to_string());
            violations.push(ExpectedViolation {
                line: line_num + 1,
                operator,
            });
        } else if inline_re.is_match(line) {
            // Violation is on this line
            let operator = op_re
                .captures(line)
                .map(|c| c.get(1).unwrap().as_str().to_string());
            violations.push(ExpectedViolation {
                line: line_num,
                operator,
            });
        }
    }

    violations
}

/// Parse configuration from checkstyle test file header.
/// Looks for patterns like:
/// - `option = nl` or `option = eol` or `option = (default)nl`
/// - `option = \tEOL` (with leading tab that needs trimming)
/// - `option = invalid_option` (invalid options mean no violations expected)
/// - `tokens = ASSIGN,COLON,LAND,LOR`
fn parse_config(source: &str) -> TestConfig {
    let mut config = TestConfig::default();

    // Find the header comment block (first /* ... */ comment)
    let header_end = source.find("*/").unwrap_or(0);
    let header = &source[..header_end];

    // Parse option - handle tabs, whitespace, and (default) prefix
    // Also handle escaped tabs like \t
    let option_re =
        Regex::new(r"option\s*=\s*(?:\\t)?\s*\(?(?:default\)?)?\s*(nl|eol|NL|EOL|invalid_option)")
            .unwrap();
    if let Some(caps) = option_re.captures(header) {
        let opt = caps.get(1).unwrap().as_str().to_lowercase();
        if opt == "invalid_option" {
            // Invalid option means this test expects no violations
            // Use an empty token set to disable all checks
            config.tokens = HashSet::new();
        } else if opt == "eol" {
            config.option = WrapOption::Eol;
        } else {
            config.option = WrapOption::Nl;
        }
    }

    // Parse tokens (may span multiple lines with backslash continuation)
    let tokens_re = Regex::new(r"tokens\s*=\s*(.+?)(?:\n[^\\]|\n\n|\*/)").unwrap();
    if let Some(caps) = tokens_re.captures(header) {
        let tokens_str = caps.get(1).unwrap().as_str();
        // Handle backslash continuation and clean up
        let tokens_str = tokens_str.replace("\\\n", " ").replace('\n', " ");
        let tokens_str = tokens_str.trim();

        // Check if it's (default) - if so, use default tokens
        if !tokens_str.starts_with("(default)") {
            let mut tokens = HashSet::new();
            for token in tokens_str.split(',') {
                let token = token.trim();
                if let Some(t) = OperatorWrapToken::from_checkstyle_name(token) {
                    tokens.insert(t);
                }
            }
            if !tokens.is_empty() {
                config.tokens = tokens;
            }
        }
    }

    config
}

/// Run OperatorWrap rule on source with given config and collect violations.
fn check_operator_wrap(source: &str, config: &TestConfig) -> Vec<ActualViolation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = OperatorWrap {
        option: config.option,
        tokens: config.tokens.clone(),
    };
    let ctx = CheckContext::new(source);
    let source_code = ctx.source_code();
    let op_re = Regex::new(r"'([^']+)'").unwrap();

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            // Extract operator from message
            let message = diagnostic.kind.body.clone();
            let operator = op_re
                .captures(&message)
                .map(|c| c.get(1).unwrap().as_str().to_string())
                .unwrap_or_default();

            violations.push(ActualViolation {
                line: loc.line.get(),
                column: loc.column.get(),
                operator,
                message,
            });
        }
    }

    violations
}

/// Run a single test fixture and compute results.
fn run_fixture(file_name: &str) -> Option<TestResult> {
    let path = checkstyle_repo::whitespace_test_input("operatorwrap", file_name)?;
    let source = std::fs::read_to_string(&path).ok()?;

    let config = parse_config(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_operator_wrap(&source, &config);

    // Match expected to actual violations
    let mut correct = vec![];
    let mut matched_expected: HashSet<usize> = HashSet::new();
    let mut matched_actual: HashSet<usize> = HashSet::new();

    for (ei, exp) in expected.iter().enumerate() {
        for (ai, act) in actual.iter().enumerate() {
            if exp.line == act.line && !matched_actual.contains(&ai) {
                // Optionally check operator matches
                let op_matches = exp
                    .operator
                    .as_ref()
                    .map(|o| {
                        // Handle escaped operators in expected (e.g., \+ for +)
                        let unescaped = o.replace("\\+", "+").replace("\\|", "|");
                        unescaped == act.operator
                    })
                    .unwrap_or(true);

                if op_matches {
                    correct.push((exp.clone(), act.clone()));
                    matched_expected.insert(ei);
                    matched_actual.insert(ai);
                    break;
                }
            }
        }
    }

    let missing: Vec<_> = expected
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_expected.contains(i))
        .map(|(_, v)| v.clone())
        .collect();

    let false_positives: Vec<_> = actual
        .iter()
        .enumerate()
        .filter(|(i, _)| !matched_actual.contains(i))
        .map(|(_, v)| v.clone())
        .collect();

    Some(TestResult {
        file_name: file_name.to_string(),
        config,
        expected,
        actual,
        correct,
        missing,
        false_positives,
    })
}

/// Load a fixture file for manual testing.
#[allow(dead_code)]
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("operatorwrap", file_name)?;
    std::fs::read_to_string(&path).ok()
}

// =============================================================================
// Comprehensive Test Harness
// =============================================================================

/// All checkstyle OperatorWrap test fixtures.
const ALL_FIXTURES: &[&str] = &[
    "InputOperatorWrap1.java",
    "InputOperatorWrap2.java",
    "InputOperatorWrap3.java",
    "InputOperatorWrap4.java",
    "InputOperatorWrap5.java",
    "InputOperatorWrap6.java",
    "InputOperatorWrapNl.java",
    "InputOperatorWrapEol.java",
    "InputOperatorWrapInstanceOfOperator.java",
    "InputOperatorWrapInstanceOfOperatorEndOfLine.java",
    "InputOperatorWrapGuardedPatterns.java",
    "InputOperatorWrapTryWithResources.java",
    "InputOperatorWrapArrayAssign.java",
    "InputOperatorWrapWithTrimOptionProperty.java",
];

#[test]
fn test_all_fixtures_comprehensive() {
    let mut total_expected = 0;
    let mut total_correct = 0;
    let mut total_missing = 0;
    let mut total_false_positives = 0;
    let mut results = vec![];

    for fixture in ALL_FIXTURES {
        match run_fixture(fixture) {
            Some(result) => {
                total_expected += result.expected.len();
                total_correct += result.correct.len();
                total_missing += result.missing.len();
                total_false_positives += result.false_positives.len();
                result.print_summary();
                results.push(result);
            }
            None => {
                println!("\n=== {} ===", fixture);
                println!("SKIPPED: checkstyle repo not available or file not found");
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("OVERALL SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Total expected violations: {}", total_expected);
    println!("Total correct matches: {}", total_correct);
    println!("Total missing (false negatives): {}", total_missing);
    println!("Total false positives: {}", total_false_positives);

    if total_expected > 0 {
        let accuracy = (total_correct as f64 / total_expected as f64) * 100.0;
        println!("Accuracy: {:.1}%", accuracy);
    }

    let perfect_count = results.iter().filter(|r| r.is_perfect()).count();
    println!("Perfect fixtures: {}/{}", perfect_count, results.len());
}

// =============================================================================
// Individual Fixture Tests (for CI granularity)
// =============================================================================

#[test]
fn test_input_operator_wrap_1() {
    let Some(result) = run_fixture("InputOperatorWrap1.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // This fixture tests default NL option with binary and type bound operators
}

#[test]
fn test_input_operator_wrap_nl() {
    let Some(result) = run_fixture("InputOperatorWrapNl.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests NL option with ASSIGN, COLON, LAND, LOR, STAR, QUESTION tokens
}

#[test]
fn test_input_operator_wrap_eol() {
    let Some(result) = run_fixture("InputOperatorWrapEol.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests EOL option with ASSIGN, COLON, LAND, LOR, STAR, QUESTION tokens
}

#[test]
fn test_input_operator_wrap_instanceof() {
    let Some(result) = run_fixture("InputOperatorWrapInstanceOfOperator.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests instanceof operator with NL option
}

#[test]
fn test_input_operator_wrap_instanceof_eol() {
    let Some(result) = run_fixture("InputOperatorWrapInstanceOfOperatorEndOfLine.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests instanceof operator with EOL option
}

#[test]
fn test_input_operator_wrap_guarded_patterns() {
    let Some(result) = run_fixture("InputOperatorWrapGuardedPatterns.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests pattern matching with guards
}

#[test]
fn test_input_operator_wrap_try_with_resources() {
    let Some(result) = run_fixture("InputOperatorWrapTryWithResources.java") else {
        eprintln!("Skipping: checkstyle repo not available");
        return;
    };
    result.print_summary();
    // Tests try-with-resources assignment
}

// =============================================================================
// Basic Sanity Tests
// =============================================================================

#[test]
fn test_simple_nl_violation() {
    let source = r#"
class Test {
    void test() {
        int x = 1 +
            2;
    }
}
"#;
    let config = TestConfig::default();
    let violations = check_operator_wrap(source, &config);
    assert!(
        !violations.is_empty(),
        "Should find violation for operator at end of line with NL option"
    );
    assert_eq!(violations[0].line, 4);
    assert_eq!(violations[0].operator, "+");
}

#[test]
fn test_simple_nl_ok() {
    let source = r#"
class Test {
    void test() {
        int x = 1
            + 2;
    }
}
"#;
    let config = TestConfig::default();
    let violations = check_operator_wrap(source, &config);
    assert!(
        violations.is_empty(),
        "Operator on new line should be OK with NL option"
    );
}

#[test]
fn test_simple_eol_violation() {
    let source = r#"
class Test {
    void test() {
        int x = 1
            + 2;
    }
}
"#;
    let config = TestConfig {
        option: WrapOption::Eol,
        tokens: OperatorWrapToken::default_tokens(),
    };
    let violations = check_operator_wrap(source, &config);
    assert!(
        !violations.is_empty(),
        "Should find violation for operator on new line with EOL option"
    );
}

#[test]
fn test_same_line_no_violation() {
    let source = r#"
class Test {
    void method() {
        int x = 1 + 2 + 3;
        boolean y = true && false;
    }
}
"#;
    let config = TestConfig::default();
    let violations = check_operator_wrap(source, &config);
    assert!(
        violations.is_empty(),
        "Same line expressions should not cause violations"
    );
}

#[test]
fn test_ternary_nl_violation() {
    let source = r#"
class Test {
    void test() {
        int x = true ?
            1 : 2;
    }
}
"#;
    let config = TestConfig::default();
    let violations = check_operator_wrap(source, &config);
    assert!(
        !violations.is_empty(),
        "Should find violation for ternary ? at end of line"
    );
}
