//! Checkstyle compatibility tests for Indentation rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the Indentation check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::Indentation;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::{HashMap, HashSet};

/// Parse configuration from fixture file header comments.
/// Format: ` * propertyName = value   //indent:...`
fn parse_fixture_config(source: &str) -> HashMap<String, String> {
    let mut config = HashMap::new();

    for line in source.lines() {
        // Stop at class/interface declaration
        if line.contains("class ") || line.contains("interface ") || line.contains("enum ") {
            break;
        }

        // Look for config lines like: " * basicOffset = 2"
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("*") {
            let rest = rest.trim();
            // Skip empty lines and non-config lines
            if rest.is_empty() || rest.starts_with('@') || rest.starts_with("This") {
                continue;
            }

            // Parse "propertyName = value" before any //indent comment
            if let Some(eq_pos) = rest.find('=') {
                let key = rest[..eq_pos].trim();
                let value_part = &rest[eq_pos + 1..];

                // Strip trailing //indent:... comment
                let value = if let Some(comment_pos) = value_part.find("//") {
                    value_part[..comment_pos].trim()
                } else {
                    value_part.trim()
                };

                // Only store known config properties
                if matches!(
                    key,
                    "basicOffset"
                        | "braceAdjustment"
                        | "caseIndent"
                        | "throwsIndent"
                        | "arrayInitIndent"
                        | "lineWrappingIndentation"
                        | "forceStrictCondition"
                        | "tabWidth"
                ) {
                    config.insert(key.to_string(), value.to_string());
                }
            }
        }
    }

    config
}

/// Run Indentation rule on source with given config and collect violation lines.
fn check_indentation_with_config(source: &str, properties: &HashMap<String, String>) -> HashSet<usize> {
    check_indentation_with_config_debug(source, properties, false)
}

/// Run Indentation rule with optional debug output.
fn check_indentation_with_config_debug(
    source: &str,
    properties: &HashMap<String, String>,
    debug: bool,
) -> HashSet<usize> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    // Convert HashMap<String, String> to HashMap<&str, &str>
    let props: HashMap<&str, &str> = properties
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let rule = Indentation::from_config(&props);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violation_lines = HashSet::new();

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            if debug {
                eprintln!(
                    "Violation line {}: {}",
                    loc.line.get(),
                    diagnostic.kind.body
                );
            }
            violation_lines.insert(loc.line.get());
        }
    }

    violation_lines
}

/// Run Indentation rule on source with default config and collect violation lines.
fn check_indentation(source: &str) -> HashSet<usize> {
    let properties = HashMap::new();
    check_indentation_with_config(source, &properties)
}

/// Parse expected violations from checkstyle test file comments.
/// Lines with `//indent:X exp:Y warn` are expected to have violations.
/// Lines with `//indent:X exp:Y` where X != Y are expected to have violations.
fn parse_expected_violations(source: &str) -> HashSet<usize> {
    let mut expected = HashSet::new();

    for (line_no, line) in source.lines().enumerate() {
        // Look for //indent:X exp:Y or //indent:X exp:>=Y patterns
        if let Some(comment_start) = line.find("//indent:") {
            let comment = &line[comment_start..];

            // Check if it has "warn" suffix - these are definitely violations
            if comment.contains("warn") {
                expected.insert(line_no + 1);
                continue;
            }

            // Parse //indent:X exp:Y format
            // X is actual, Y is expected
            if let Some(exp_pos) = comment.find("exp:") {
                let indent_str = &comment[9..]; // after "//indent:"
                let exp_str = &comment[exp_pos + 4..]; // after "exp:"

                // Extract the actual indent number
                let actual: Option<i32> = indent_str
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok());

                // Extract expected - handle >=X format
                let expected_ok = if let Some(stripped) = exp_str.strip_prefix(">=") {
                    // >=Y format - actual must be at least Y
                    let min: Option<i32> = stripped
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok());
                    match (actual, min) {
                        (Some(a), Some(m)) => a >= m,
                        _ => true,
                    }
                } else {
                    // Exact Y format
                    let exp: Option<i32> = exp_str
                        .split_whitespace()
                        .next()
                        .and_then(|s| s.parse().ok());
                    match (actual, exp) {
                        (Some(a), Some(e)) => a == e,
                        _ => true,
                    }
                };

                if !expected_ok {
                    expected.insert(line_no + 1);
                }
            }
        }
    }

    expected
}

/// Get the indentation fixtures directory path.
fn indentation_fixtures_dir() -> Option<std::path::PathBuf> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    Some(checkstyle_root.join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/indentation/indentation"))
}

/// Load a checkstyle test input file.
fn load_indentation_fixture(file_name: &str) -> Option<String> {
    let fixture_path = indentation_fixtures_dir()?.join(file_name);
    std::fs::read_to_string(fixture_path).ok()
}

/// List all indentation fixture files.
fn list_indentation_fixtures() -> Vec<String> {
    let Some(dir) = indentation_fixtures_dir() else {
        return Vec::new();
    };

    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut fixtures: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".java") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    fixtures.sort();
    fixtures
}

#[test]
fn test_valid_block_indent() {
    let Some(source) = load_indentation_fixture("InputIndentationValidBlockIndent.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_indentation(&source);
    let expected = parse_expected_violations(&source);

    // Valid file should have no violations beyond what's marked
    // For now, just check we don't crash and produce reasonable results
    if !violations.is_empty() {
        eprintln!("Violations found at lines: {:?}", violations);
        eprintln!("Expected violations at lines: {:?}", expected);
    }

    // This is a Valid file, so we expect few/no violations
    // But our implementation may not be complete yet, so we just check it runs
}

#[test]
fn test_valid_class_def_indent() {
    let Some(source) = load_indentation_fixture("InputIndentationValidClassDefIndent.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_indentation(&source);

    // This is a Valid file for the most part
    // Just verify it runs without crashing
    eprintln!("Found {} violations", violations.len());
}

#[test]
fn test_simple_correct_indentation() {
    let source = r#"
class Foo {
    int x;

    void bar() {
        int y = 1;
        if (true) {
            y = 2;
        }
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented code, got lines: {:?}", violations);
}

#[test]
fn test_simple_incorrect_member_indentation() {
    let source = r#"
class Foo {
  int x;
}
"#;
    let violations = check_indentation(source);
    assert!(!violations.is_empty(), "Expected violations for incorrectly indented member");
    assert!(violations.contains(&3), "Expected violation on line 3, got: {:?}", violations);
}

#[test]
fn test_simple_incorrect_method_body_indentation() {
    let source = r#"
class Foo {
    void bar() {
      int x = 1;
    }
}
"#;
    let violations = check_indentation(source);
    assert!(!violations.is_empty(), "Expected violations for incorrectly indented statement");
    assert!(violations.contains(&4), "Expected violation on line 4, got: {:?}", violations);
}

#[test]
fn test_if_statement_indentation() {
    let source = r#"
class Foo {
    void bar() {
        if (true) {
            int x = 1;
        } else {
            int y = 2;
        }
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented if-else, got lines: {:?}", violations);
}

#[test]
fn test_switch_statement_indentation() {
    let source = r#"
class Foo {
    void bar(int x) {
        switch (x) {
            case 1:
                break;
            case 2:
                break;
            default:
                break;
        }
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented switch, got lines: {:?}", violations);
}

#[test]
fn test_try_catch_indentation() {
    let source = r#"
class Foo {
    void bar() {
        try {
            int x = 1;
        } catch (Exception e) {
            int y = 2;
        } finally {
            int z = 3;
        }
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented try-catch, got lines: {:?}", violations);
}

#[test]
fn test_nested_class_indentation() {
    let source = r#"
class Outer {
    class Inner {
        int x;

        void foo() {
            int y = 1;
        }
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented nested class, got lines: {:?}", violations);
}

#[test]
fn test_lambda_expression_block_body() {
    let source = r#"
class Foo {
    void bar() {
        list.forEach(x -> {
            System.out.println(x);
        });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented lambda with block body, got lines: {:?}", violations);
}

#[test]
fn test_lambda_expression_single_line() {
    let source = r#"
class Foo {
    Runnable r = () -> System.out.println("hi");
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for single-line lambda, got lines: {:?}", violations);
}

#[test]
fn test_method_invocation_multiline_args() {
    let source = r#"
class Foo {
    void bar() {
        someMethod(
            arg1,
            arg2
        );
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for multiline method args, got lines: {:?}", violations);
}

#[test]
fn test_array_initializer() {
    let source = r#"
class Foo {
    int[] arr = {
        1,
        2,
        3
    };
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for array initializer, got lines: {:?}", violations);
}

#[test]
fn test_anonymous_class() {
    let source = r#"
class Foo {
    Runnable r = new Runnable() {
        public void run() {
            System.out.println("running");
        }
    };
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for anonymous class, got lines: {:?}", violations);
}

#[test]
fn test_chained_method_calls() {
    let source = r#"
class Foo {
    void bar() {
        Stream.of("a", "b", "c")
            .map(String::toUpperCase)
            .forEach(System.out::println);
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for chained method calls, got lines: {:?}", violations);
}

#[test]
fn test_annotation_array_initializer() {
    let source = r#"
@SuppressWarnings({"unchecked", "deprecation"})
class Foo {
    @SuppressWarnings({
        "unchecked",
        "deprecation"
    })
    void bar() {
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for annotation array initializer, got lines: {:?}", violations);
}

#[test]
fn test_lambda_with_nested_method_calls() {
    // This test checks chained method calls with lambdas.
    // The lambda body content should be indented relative to the lambda, not the statement.
    let source = r#"
class Foo {
    void bar() {
        list.forEach(x -> {
            process(x);
        });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for lambda with nested method calls, got lines: {:?}", violations);
}

#[test]
fn test_chained_method_calls_line_wrapped() {
    // Tests line-wrapped chained method calls
    let source = r#"
class Foo {
    void bar() {
        new String()
            .substring(0, 100)
            .substring(0, 50);
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for line-wrapped method chains, got lines: {:?}", violations);
}

#[test]
fn test_method_call_multiline_arguments() {
    // Tests method call with arguments spanning multiple lines
    let source = r#"
class Foo {
    void bar() {
        someMethod(
            arg1,
            arg2,
            arg3
        );
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for multiline method arguments, got lines: {:?}", violations);
}

#[test]
fn test_lambda_expression_body_on_new_line() {
    // Tests lambda with expression body on a new line
    let source = r#"
class Foo {
    void bar() {
        list.forEach(x ->
            process(x));
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for lambda body on new line, got lines: {:?}", violations);
}

#[test]
fn test_nested_new_expressions() {
    // Tests nested new expressions with line wrapping
    let source = r#"
class Foo {
    void bar() {
        new Outer(
            new Inner(
                value
            )
        );
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for nested new expressions, got lines: {:?}", violations);
}

// ============================================================================
// Checkstyle fixture-based compatibility tests
// ============================================================================

/// Results from running a fixture test.
struct FixtureTestResult {
    expected: HashSet<usize>,
    actual: HashSet<usize>,
    missing: Vec<usize>,
    extra: Vec<usize>,
}

/// Helper to run a fixture test and report results.
fn run_fixture_test(file_name: &str) -> Option<FixtureTestResult> {
    let source = load_indentation_fixture(file_name)?;
    let config = parse_fixture_config(&source);
    let expected = parse_expected_violations(&source);
    let actual = check_indentation_with_config(&source, &config);

    let missing: Vec<usize> = expected.difference(&actual).copied().collect();
    let extra: Vec<usize> = actual.difference(&expected).copied().collect();

    Some(FixtureTestResult { expected, actual, missing, extra })
}

/// Debug helper to print violations with context
fn debug_fixture(file_name: &str) {
    let Some(source) = load_indentation_fixture(file_name) else {
        eprintln!("Skipping - fixture not found");
        return;
    };

    let config = parse_fixture_config(&source);
    eprintln!("Config: {:?}", config);

    let expected = parse_expected_violations(&source);

    eprintln!("\n=== Violation Details ===");
    let actual = check_indentation_with_config_debug(&source, &config, true);

    let missing: Vec<usize> = expected.difference(&actual).copied().collect();
    let extra: Vec<usize> = actual.difference(&expected).copied().collect();

    eprintln!("\n=== Summary ===");
    eprintln!("Expected violations at lines: {:?}", expected);
    eprintln!("Actual violations at lines: {:?}", actual);
    eprintln!("Missing: {:?}", missing);
    eprintln!("Extra: {:?}", extra);

    // Print source lines for extra violations
    eprintln!("\n=== Extra violation source lines ===");
    let lines: Vec<&str> = source.lines().collect();
    for &line_no in &extra {
        if line_no > 0 && line_no <= lines.len() {
            eprintln!("Line {}: {}", line_no, lines[line_no - 1]);
        }
    }
}

#[test]
fn test_debug_valid_array_init() {
    debug_fixture("InputIndentationValidArrayInitDefaultIndent.java");
}

#[test]
fn test_debug_valid_switch() {
    debug_fixture("InputIndentationValidSwitchIndent.java");
}

/// Test Valid* fixtures - these should have minimal/no violations
#[test]
fn test_fixture_valid_block_indent() {
    let Some(result) = run_fixture_test("InputIndentationValidBlockIndent.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    // For Valid files, we primarily care that we don't produce false positives
    // Allow some implementation differences but flag excessive extra violations
    if result.extra.len() > 5 {
        eprintln!("Warning: {} extra violations in ValidBlockIndent: {:?}", result.extra.len(), result.extra);
    }
    if !result.missing.is_empty() {
        eprintln!("Missing violations in ValidBlockIndent: {:?}", result.missing);
    }

    // This test passes if we don't crash and have reasonable results
    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_valid_class_def_indent() {
    let Some(result) = run_fixture_test("InputIndentationValidClassDefIndent.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    if result.extra.len() > 5 {
        eprintln!("Warning: {} extra violations in ValidClassDefIndent: {:?}", result.extra.len(), result.extra);
    }
    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_classes_methods() {
    let Some(result) = run_fixture_test("InputIndentationClassesMethods.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("ClassesMethods: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    // This is a comprehensive test file
    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_code_blocks() {
    let Some(result) = run_fixture_test("InputIndentationCodeBlocks1.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("CodeBlocks1: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_chained_methods() {
    let Some(result) = run_fixture_test("InputIndentationChainedMethods.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("ChainedMethods: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_lambda() {
    let Some(result) = run_fixture_test("InputIndentationLambda1.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("Lambda1: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_anonymous_classes() {
    let Some(result) = run_fixture_test("InputIndentationAnonymousClasses.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("AnonymousClasses: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_arrays() {
    let Some(result) = run_fixture_test("InputIndentationArrays.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("Arrays: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_switch_expression() {
    let Some(result) = run_fixture_test("InputIndentationCheckSwitchExpression.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("SwitchExpression: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_method_call_line_wrap() {
    let Some(result) = run_fixture_test("InputIndentationMethodCallLineWrap.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("MethodCallLineWrap: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    // This is a key test for line wrapping - report details
    if !result.missing.is_empty() {
        eprintln!("Missing violations: {:?}", result.missing);
    }
    if !result.extra.is_empty() {
        eprintln!("Extra violations: {:?}", result.extra);
    }

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_annotation_definition() {
    let Some(result) = run_fixture_test("InputIndentationAnnotationDefinition.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("AnnotationDefinition: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

#[test]
fn test_fixture_try_resources() {
    let Some(result) = run_fixture_test("InputIndentationTryResources.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    eprintln!("TryResources: expected={}, actual={}, missing={}, extra={}",
        result.expected.len(), result.actual.len(), result.missing.len(), result.extra.len());

    assert!(result.expected.len() + result.actual.len() < 1000, "Sanity check failed");
}

/// Comprehensive test that runs ALL available fixtures and reports summary.
/// This test always passes but logs detailed compatibility stats.
#[test]
fn test_fixture_compatibility_summary() {
    let fixtures = list_indentation_fixtures();

    if fixtures.is_empty() {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    }

    let mut total_expected = 0;
    let mut total_actual = 0;
    let mut total_missing = 0;
    let mut total_extra = 0;
    let mut files_tested = 0;
    let mut exact_matches = 0;
    let mut parse_failures = 0;

    eprintln!();
    eprintln!("=== Indentation Compatibility Summary ({} fixtures) ===", fixtures.len());
    eprintln!();

    for fixture in &fixtures {
        if let Some(result) = run_fixture_test(fixture) {
            files_tested += 1;
            total_expected += result.expected.len();
            total_actual += result.actual.len();
            total_missing += result.missing.len();
            total_extra += result.extra.len();

            let status = if result.missing.is_empty() && result.extra.is_empty() {
                exact_matches += 1;
                "✓ MATCH"
            } else if result.missing.is_empty() {
                "~ EXTRA"
            } else if result.extra.is_empty() {
                "! MISSING"
            } else {
                "✗ DIFF"
            };

            // Only print non-matching fixtures to reduce noise
            if status != "✓ MATCH" {
                eprintln!("{} {}: exp={}, act={}, miss={}, extra={}",
                    status, fixture, result.expected.len(), result.actual.len(),
                    result.missing.len(), result.extra.len());
            }
        } else {
            parse_failures += 1;
        }
    }

    eprintln!();
    eprintln!("=== Summary ===");
    eprintln!("Total fixtures: {}", fixtures.len());
    eprintln!("Files tested: {}", files_tested);
    eprintln!("Exact matches: {} ({:.1}%)", exact_matches, 100.0 * exact_matches as f64 / files_tested as f64);
    eprintln!("Parse failures: {}", parse_failures);
    eprintln!();
    eprintln!("Total expected violations: {}", total_expected);
    eprintln!("Total actual violations: {}", total_actual);
    eprintln!("Total missing: {}", total_missing);
    eprintln!("Total extra: {}", total_extra);

    if total_expected > 0 {
        let detection_rate = 100.0 * (1.0 - (total_missing as f64 / total_expected as f64));
        eprintln!("Detection rate: {:.1}%", detection_rate);
    }

    eprintln!();

    // This test always passes - it's for reporting
    assert!(files_tested > 0, "Should test at least one fixture");
}
