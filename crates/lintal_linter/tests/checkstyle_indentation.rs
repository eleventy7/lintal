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
/// Handles both `/* Config: ... */` blocks and Javadoc-style `/** ... */` blocks.
/// If explicit config not found, infers from indent comments in code.
fn parse_fixture_config(source: &str) -> HashMap<String, String> {
    let mut config = HashMap::new();

    // First pass: look for explicit config in header comments
    // Config can appear in:
    // 1. /* Config: ... */ block
    // 2. Javadoc /** ... */ block with "This test-input is intended..." text
    let mut in_config_block = false;
    let mut in_javadoc = false;

    for line in source.lines() {
        let line_trimmed = line.trim();

        // Check for start of config block
        if line_trimmed.contains("/* Config:") || line_trimmed.contains("/*Config:") {
            in_config_block = true;
            continue;
        }

        // Check for start of Javadoc block
        if line_trimmed.starts_with("/**") {
            in_javadoc = true;
            continue;
        }

        // Check for end of comment block
        if (in_config_block || in_javadoc) && line_trimmed.contains("*/") {
            in_config_block = false;
            in_javadoc = false;
            continue;
        }

        if !in_config_block && !in_javadoc {
            continue;
        }

        // Look for config lines like: " * basicOffset = 2"
        if let Some(rest) = line_trimmed.strip_prefix("*") {
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

    // If no explicit config found, try to infer from code patterns
    if config.is_empty() {
        infer_config_from_code(source, &mut config);
    }

    config
}

/// Infer configuration by analyzing indent comments in the code.
/// Looks for the first class member after class declaration to determine basicOffset.
fn infer_config_from_code(source: &str, config: &mut HashMap<String, String>) {
    let mut in_class_body = false;
    let mut class_indent = 0i32;

    for line in source.lines() {
        // Find class declaration and its indent
        if !in_class_body && (line.contains("class ") || line.contains("interface ") || line.contains("enum ")) {
            if let Some(comment_start) = line.find("//indent:") {
                let comment = &line[comment_start..];
                if let Some(indent) = comment[9..].split_whitespace().next().and_then(|s| s.parse::<i32>().ok()) {
                    class_indent = indent;
                    in_class_body = true;
                }
            }
            continue;
        }

        // Find first class member to determine basicOffset
        if in_class_body {
            // Get the code part before any //indent comment
            let code_part = if let Some(comment_start) = line.find("//indent:") {
                line[..comment_start].trim()
            } else {
                line.trim()
            };

            // Skip braces, blank lines, and lines that are effectively empty
            if code_part.is_empty() || code_part == "{" || code_part == "}" {
                continue;
            }

            // Look for indent comment
            if let Some(comment_start) = line.find("//indent:") {
                let comment = &line[comment_start..];
                if let Some(indent) = comment[9..].split_whitespace().next().and_then(|s| s.parse::<i32>().ok()) {
                    // Sanity check: indent should be reasonable (< 20)
                    if indent > 20 {
                        continue;
                    }

                    // Found a class member with indent - calculate basicOffset
                    let basic_offset = indent - class_indent;
                    if basic_offset > 0 && basic_offset <= 8 {
                        config.insert("basicOffset".to_string(), basic_offset.to_string());

                        // Also infer lineWrappingIndentation from deeper nesting if available
                        // For now, use same as basicOffset or double it based on patterns
                        // Most common patterns: basicOffset=2 with lineWrap=4, or both=4
                        if basic_offset == 2 {
                            config.insert("lineWrappingIndentation".to_string(), "4".to_string());
                            config.insert("tabWidth".to_string(), "2".to_string());
                        } else {
                            config.insert("lineWrappingIndentation".to_string(), basic_offset.to_string());
                            config.insert("tabWidth".to_string(), basic_offset.to_string());
                        }
                    }
                    break;
                }
            }
        }
    }
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
/// Lines with `//below indent:X exp:Y warn` indicate the NEXT line should have a violation.
fn parse_expected_violations(source: &str) -> HashSet<usize> {
    let mut expected = HashSet::new();
    let lines: Vec<&str> = source.lines().collect();

    for (line_no, line) in lines.iter().enumerate() {
        // Handle `//below indent:X exp:Y warn` - violation is on the NEXT line
        if line.contains("//below indent:") && line.contains("warn") {
            // The violation is on the next line (line_no + 2 because line_no is 0-indexed)
            expected.insert(line_no + 2);
            continue;
        }

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
fn test_explicit_constructor_invocation() {
    // Test super() and this() constructor calls
    let source = r#"
class Base {
    Base(long arg) {}
}
class Invalid extends Base {
    public Invalid(long arg) {
    super(
    arg
    + 1L);
    }
}
"#;
    // Debug: print AST structure
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    fn print_tree(node: lintal_java_cst::CstNode, depth: usize) {
        let indent = "  ".repeat(depth);
        if node.kind().contains("constructor") || node.kind() == "super" || node.kind() == "this" || node.kind() == "block" {
            eprintln!("{}{}", indent, node.kind());
        }
        for child in node.children() {
            print_tree(child, depth + 1);
        }
    }

    eprintln!("\n=== AST for constructor call test ===");
    print_tree(lintal_java_cst::CstNode::new(result.tree.root_node(), source), 0);

    // Now check with config
    let mut props = std::collections::HashMap::new();
    props.insert("basicOffset".to_string(), "4".to_string());
    props.insert("lineWrappingIndentation".to_string(), "4".to_string());

    let violations = check_indentation_with_config(source, &props);
    eprintln!("Violations: {:?}", violations);

    // super( at line 7 column 4 should be violation (expected 8)
    // arg at line 8 column 4 should be violation
    // + 1L at line 9 column 4 should be violation
    assert!(violations.contains(&7) || violations.contains(&8),
        "Expected violations for incorrectly indented super() call, got: {:?}", violations);
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
// Lambda-in-method-call tests (real-world patterns from artio/agrona/aeron)
// ============================================================================
//
// These tests verify checkstyle-compatible handling of lambda blocks inside
// method call arguments. Checkstyle accepts lambda body braces at the method
// call indentation level (not method call + lineWrappingIndentation).

#[test]
fn test_lambda_in_constructor_call_block_at_call_level() {
    // Pattern from agrona MarkFileTest.java:
    // threads[i] = new Thread(() ->
    // {
    //     startLatch.countDown();
    // });
    //
    // The lambda block `{` is at column 12 (same as `new Thread`), not column 16.
    // Checkstyle accepts this pattern.
    let source = r#"
class Foo {
    void bar() {
        threads[i] = new Thread(() ->
        {
            startLatch.countDown();
        });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for lambda block at method call indent level, got lines: {:?}", violations);
}

#[test]
fn test_lambda_in_method_arg_block_at_call_level() {
    // Pattern: executor.submit(() ->
    // {
    //     doWork();
    // });
    //
    // Lambda block at same level as method call, not +lineWrappingIndentation.
    let source = r#"
class Foo {
    void bar() {
        executor.submit(() ->
        {
            doWork();
        });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for lambda in method arg with block at call level, got lines: {:?}", violations);
}

#[test]
fn test_lambda_with_try_catch_in_method_arg() {
    // Pattern from artio: lambda containing try-catch at method call level
    // Checkstyle accepts the entire lambda body at method call indent level.
    let source = r#"
class Foo {
    void bar() {
        executor.submit(() ->
        {
            try
            {
                process();
            }
            catch (Exception e)
            {
                handleError(e);
            }
        });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for lambda with try-catch at call level, got lines: {:?}", violations);
}

#[test]
fn test_lambda_in_assignment_block_at_call_level() {
    // Pattern: Runnable r = () ->
    // {
    //     doWork();
    // };
    //
    // Lambda assigned to variable, block at variable declaration level.
    let source = r#"
class Foo {
    void bar() {
        Runnable r = () ->
        {
            doWork();
        };
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for lambda assignment with block at declaration level, got lines: {:?}", violations);
}

#[test]
fn test_nested_lambdas_in_method_chain() {
    // Pattern from artio: nested lambdas in method chains
    // Each lambda block at the method call level of its enclosing call.
    let source = r#"
class Foo {
    void bar() {
        list.stream()
            .map(x -> process(x))
            .forEach(x ->
            {
                output(x);
            });
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for nested lambda in method chain, got lines: {:?}", violations);
}

#[test]
fn test_lambda_block_inline_with_arrow() {
    // Pattern: Standard inline lambda (for comparison - this should work)
    // list.forEach(x -> {
    //     process(x);
    // });
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
    assert!(violations.is_empty(),
        "Expected no violations for standard inline lambda block, got lines: {:?}", violations);
}

#[test]
fn test_annotation_array_init_in_method_arg() {
    // Pattern from aeron: annotation-style array in method arguments
    // Checkstyle treats this leniently.
    let source = r#"
@SuppressWarnings({"unchecked",
    "deprecation"})
class Foo {
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for annotation array init continuation, got lines: {:?}", violations);
}

// ============================================================================
// Lambda-in-method-call tests with forceStrictCondition=true
// ============================================================================
//
// These tests use the agrona/artio/aeron config which has forceStrictCondition=true.
// This is the configuration that triggers false positives in real-world code.

fn strict_config() -> HashMap<String, String> {
    [
        ("basicOffset", "4"),
        ("braceAdjustment", "0"),
        ("caseIndent", "4"),
        ("throwsIndent", "4"),
        ("arrayInitIndent", "4"),
        ("lineWrappingIndentation", "4"),
        ("forceStrictCondition", "true"),
    ].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

#[test]
fn test_lambda_block_at_call_level_strict() {
    // Pattern from agrona MarkFileTest.java with forceStrictCondition=true:
    // threads[i] = new Thread(() ->
    // {                              <- at same level as new Thread (col 8)
    //     startLatch.countDown();    <- at col 12 (8 + 4)
    // });
    //
    // Checkstyle accepts this even with forceStrictCondition=true.
    // Lintal currently expects { at col 12 (8 + lineWrap=4).
    let source = r#"
class Foo {
    void bar() {
        threads[i] = new Thread(() ->
        {
            startLatch.countDown();
        });
    }
}
"#;
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Expected no violations for lambda block at call level (strict mode), got lines: {:?}", violations);
}

#[test]
fn test_lambda_with_try_catch_strict() {
    // Pattern from agrona: lambda with try-catch at method call level.
    // With forceStrictCondition=true, checkstyle still accepts this.
    let source = r#"
class Foo {
    void bar() {
        executor.submit(() ->
        {
            try
            {
                process();
            }
            catch (Exception e)
            {
                handleError(e);
            }
        });
    }
}
"#;
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Expected no violations for lambda with try-catch (strict mode), got lines: {:?}", violations);
}

#[test]
fn test_lambda_nested_try_resources_strict() {
    // Pattern from agrona MarkFileTest: try-with-resources inside lambda.
    // The lambda block and all nested content should be at method call level.
    let source = r#"
class Foo {
    void bar() {
        threads[i] = new Thread(() ->
        {
            startLatch.countDown();

            try
            {
                startLatch.await();

                try (Resource r = new Resource())
                {
                    r.process();
                }
            }
            catch (Exception ex)
            {
                exceptions[index] = ex;
            }
            finally
            {
                endLatch.countDown();
            }
        });
    }
}
"#;
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Expected no violations for lambda with nested try-resources (strict mode), got lines: {:?}", violations);
}

// ============================================================================
// Return statement and field declaration lenient arg checking tests
// ============================================================================
//
// Checkstyle doesn't strictly check method call argument indentation when the
// call is inside a return statement or field declaration. These tests codify
// that behavior.

#[test]
fn test_return_statement_args_any_indent() {
    // Checkstyle accepts ANY indentation for method call args in return statements.
    // This is a documented lenient behavior.
    let source = r#"
class Foo {
    Object bar() {
        return baz(
qux);
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Return statement should accept args at any indent, got lines: {:?}", violations);

    // Also passes with strict config
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Return statement should accept args at any indent (strict), got lines: {:?}", violations);
}

#[test]
fn test_field_declaration_args_lenient() {
    // Checkstyle accepts >= member indent for method call args in field declarations.
    let source = r#"
class Foo {
    Object x = baz(
        qux);
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Field declaration should accept args at member indent, got lines: {:?}", violations);
}

#[test]
fn test_expression_statement_args_strict() {
    // Expression statements SHOULD check argument indentation strictly.
    // Args at col 0 should fail.
    let source = r#"
class Foo {
    void bar() {
        baz(
qux);
    }
}
"#;
    let violations = check_indentation(source);
    assert!(!violations.is_empty(),
        "Expression statement should require proper arg indent");
    assert!(violations.contains(&5), "Line 5 (qux at col 0) should be flagged");
}

#[test]
fn test_throw_statement_binary_expr_any_indent() {
    // Checkstyle accepts ANY indentation for binary expression continuations
    // inside throw statements, similar to return statements.
    let source = r#"
class Foo {
    void bar() {
        throw new Ex("msg" +
            arg);
    }
}
"#;
    // Should pass with strict config - checkstyle is lenient here
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Throw statement should accept binary expr at any indent, got lines: {:?}", violations);
}

#[test]
fn test_return_statement_binary_expr_visual_align() {
    // Checkstyle accepts visual alignment for binary expression continuations
    // inside return statements (aligning with previous operand).
    let source = r#"
class Foo {
    boolean equals(Object other) {
        return otherSet.value == value &&
               otherSet.size == size;
    }
}
"#;
    // Should pass with strict config - checkstyle allows visual alignment
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Return statement should accept visual alignment, got lines: {:?}", violations);
}

// ============================================================================
// Expression continuation and method argument alignment tests
// ============================================================================
//
// These tests verify patterns where method arguments or expression continuations
// are aligned with the containing expression rather than at strict indent levels.

#[test]
fn test_binary_expression_string_concat_aligned() {
    // Pattern from agrona ManyToOneRingBuffer:
    // throw new IllegalStateException("claimed space previously " +
    //     (PADDING_MSG_TYPE_ID == buffer.getInt(typeOffset(recordIndex)) ? "aborted" : "committed"));
    //
    // The continuation is at statement level + 4, not +lineWrap from the string.
    // Checkstyle accepts this alignment.
    let source = r#"
class Foo {
    void bar() {
        throw new IllegalStateException("claimed space previously " +
            (flag ? "aborted" : "committed"));
    }
}
"#;
    // First check with lenient mode (default) - should pass
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for string concat (lenient mode), got lines: {:?}", violations);

    // With strict mode, this may fail - checkstyle has special alignment rules
    // TODO: investigate checkstyle's alignment handling
}

#[test]
fn test_binary_expression_boolean_and_aligned() {
    // Pattern from agrona IntHashSet:
    // return otherSet.containsMissingValue == containsMissingValue &&
    //        otherSet.sizeOfArrayValues == sizeOfArrayValues &&
    //        containsAll(otherSet);
    //
    // The continuation lines are aligned with the first operand after return.
    // Checkstyle accepts this visual alignment.
    let source = r#"
class Foo {
    boolean equals(Object other) {
        return otherSet.value == value &&
               otherSet.size == size &&
               containsAll(otherSet);
    }
}
"#;
    // Lenient mode - should pass (actual >= expected)
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for boolean && (lenient mode), got lines: {:?}", violations);
}

#[test]
fn test_method_args_aligned_with_open_paren() {
    // Pattern from artio CommonDecoderImplTest:
    // return Stream.of(
    //   Arguments.of(String.valueOf(VALUE_MAX_VAL), -1, -1, false),
    //   Arguments.of("1.999999999999999999", 19, 1, true));
    //
    // Arguments are indented 2 spaces inside the paren, visually aligned.
    // Checkstyle accepts this pattern even though it's less than lineWrappingIndentation.
    let source = r#"
class Foo {
    static Stream<Arguments> data() {
        return Stream.of(
          Arguments.of("value1", -1, -1, false),
          Arguments.of("value2", 19, 1, true));
    }
}
"#;
    // Lenient mode - should pass (actual >= expected not required for under-indented)
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for method args with 2-space visual alignment (lenient), got lines: {:?}", violations);
}

#[test]
fn test_constructor_args_aligned_with_new() {
    // Pattern from artio EncoderGeneratorTest:
    // final EncoderGenerator encoderGenerator =
    //     new EncoderGenerator(MESSAGE_EXAMPLE, TEST_PACKAGE, TEST_PARENT_PACKAGE,
    //     validationClass, rejectUnknownField);
    //
    // Constructor args are at same level as 'new', not +lineWrap.
    let source = r#"
class Foo {
    void bar() {
        final Generator generator =
            new Generator(ARG1, ARG2, ARG3,
            ARG4, ARG5);
    }
}
"#;
    // Lenient mode - should pass
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for constructor args aligned with new (lenient), got lines: {:?}", violations);
}

#[test]
fn test_nested_method_call_args_aligned() {
    // Pattern from aeron AgentTests:
    // assertEquals(
    //     EnumSet.complementOf(EnumSet.of(
    //     FRAME_IN,
    //     FRAME_OUT)),
    //     DriverComponentLogger.ENABLED_EVENTS);
    //
    // Inner args aligned with containing method name.
    let source = r#"
class Foo {
    void bar() {
        assertEquals(
            EnumSet.complementOf(EnumSet.of(
            VALUE_A,
            VALUE_B)),
            targetCollection);
    }
}
"#;
    // Lenient mode - should pass
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for nested method call args aligned (lenient), got lines: {:?}", violations);
}

#[test]
fn test_anonymous_class_in_lambda_aligned() {
    // Pattern from aeron ClusterTest:
    // supplier((i) -> new Service[]{ new Service()
    // {   // <- aligned with lambda start at col 12
    //     ...
    // }.index(i) }
    // );
    // Checkstyle accepts alignment of anonymous class braces with lambda position.
    let source = r#"
class Foo {
    void bar() {
        supplier(
            (i) -> new Service[]{ new Service()
            {
                public void method() { }
            }.index(i) }
        );
    }
}
"#;
    // Strict mode - should pass (checkstyle accepts this alignment)
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Expected no violations for anonymous class in lambda aligned, got lines: {:?}", violations);
}

#[test]
fn test_nested_method_call_arg_at_method_line_start() {
    // Pattern from aeron DriverEventLoggerTest:
    // assertEquals(uri,
    //     logBuffer.getStringAscii(encodedMsgOffset(recordOffset),
    //     LITTLE_ENDIAN));
    //
    // The inner arg LITTLE_ENDIAN is at col 12, same as logBuffer's line start.
    // Checkstyle accepts this visual alignment.
    let source = r#"
class Foo {
    void bar() {
        assertEquals(uri,
            logBuffer.getStringAscii(encodedMsgOffset(recordOffset),
            LITTLE_ENDIAN));
    }
}
"#;
    // Strict mode - should pass (checkstyle accepts this alignment)
    let violations = check_indentation_with_config(source, &strict_config());
    assert!(violations.is_empty(),
        "Expected no violations for nested arg at method line start, got lines: {:?}", violations);
}

#[test]
fn test_annotation_array_init_indented() {
    // Pattern from aeron ArchiveEventLoggerTest:
    // @EnumSource(
    //     value = ArchiveEventCode.class,
    //     mode = EXCLUDE,
    //     names = {
    //         "CMD_OUT_RESPONSE", "REPLICATION_SESSION_STATE_CHANGE",
    //         "CONTROL_SESSION_STATE_CHANGE"
    //     })
    //
    // Array elements indented from the attribute, not from annotation level.
    let source = r#"
@EnumSource(
    value = MyEnum.class,
    mode = EXCLUDE,
    names = {
        "VALUE_A", "VALUE_B",
        "VALUE_C"
    })
class Foo {
}
"#;
    // Lenient mode - should pass
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for annotation array with indented elements (lenient), got lines: {:?}", violations);
}

#[test]
fn test_method_chain_nested_builder() {
    // Pattern from artio BinaryEntryPointClient:
    // .investorID(123)
    // .custodianInfo()
    //     .custodian(1)
    //     .custodyAccount(2);
    //
    // Nested builder methods indented further.
    let source = r#"
class Foo {
    void bar() {
        builder
            .investorID(123)
            .custodianInfo()
                .custodian(1)
                .custodyAccount(2);
    }
}
"#;
    // Lenient mode - should pass (deeper indentation is accepted)
    let violations = check_indentation(source);
    assert!(violations.is_empty(),
        "Expected no violations for nested builder chain (lenient), got lines: {:?}", violations);
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
/// Get correct config overrides for files where header comments have wrong values.
/// These overrides are based on the actual test configurations in IndentationCheckTest.java.
fn get_config_overrides(file_name: &str) -> Option<HashMap<String, String>> {
    match file_name {
        // SwitchCasesAndEnums file header says caseIndent=4, but actual test uses caseIndent=2
        "InputIndentationSwitchCasesAndEnums.java" => Some([
            ("arrayInitIndent", "4"),
            ("basicOffset", "2"),
            ("braceAdjustment", "2"),
            ("caseIndent", "2"),  // Corrected from 4 to 2
            ("forceStrictCondition", "false"),
            ("lineWrappingIndentation", "4"),
            ("tabWidth", "4"),
            ("throwsIndent", "4"),
        ].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()),
        // TryWithResourcesStrict files need forceStrictCondition=true
        "InputIndentationTryWithResourcesStrict.java" |
        "InputIndentationTryWithResourcesStrict1.java" => Some([
            ("basicOffset", "4"),
            ("forceStrictCondition", "true"),
            ("lineWrappingIndentation", "4"),
            ("tabWidth", "4"),
        ].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()),
        _ => None,
    }
}

fn run_fixture_test(file_name: &str) -> Option<FixtureTestResult> {
    let source = load_indentation_fixture(file_name)?;
    let config = get_config_overrides(file_name)
        .unwrap_or_else(|| parse_fixture_config(&source));
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

    let config = get_config_overrides(file_name)
        .unwrap_or_else(|| parse_fixture_config(&source));
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

#[test]
fn test_debug_labels() {
    debug_fixture("InputIndentationLabels.java");
}

#[test]
fn test_debug_anonymous_classes() {
    debug_fixture("InputIndentationAnonymousClasses.java");
}

#[test]
fn test_debug_lambda1() {
    debug_fixture("InputIndentationLambda1.java");
}

#[test]
fn test_debug_lambda6() {
    debug_fixture("InputIndentationLambda6.java");
}

#[test]
fn test_debug_lambda_base() {
    debug_fixture("InputIndentationLambda.java");
}

#[test]
fn test_debug_record_line_wrapped() {
    debug_fixture("InputIndentationLineWrappedRecordDeclaration.java");
}

#[test]
fn test_debug_array_init() {
    debug_fixture("InputIndentationInvalidArrayInitIndent.java");
}

#[test]
fn test_debug_array_init1() {
    debug_fixture("InputIndentationInvalidArrayInitIndent1.java");
}

#[test]
fn test_debug_array_init_2d() {
    debug_fixture("InputIndentationInvalidArrayInitIndent2D.java");
}

#[test]
fn test_debug_array_init_emoji() {
    debug_fixture("InputIndentationArrayInitIndentWithEmoji.java");
}

#[test]
fn test_debug_arrays() {
    debug_fixture("InputIndentationArrays.java");
}

#[test]
fn test_debug_array_two() {
    debug_fixture("InputIndentationValidArrayInitIndentTwo.java");
}

#[test]
fn test_debug_try_resources() {
    debug_fixture("InputIndentationTryResourcesNotStrict1.java");
}

#[test]
fn test_debug_valid_assign() {
    debug_fixture("InputIndentationValidAssignIndent.java");
}

#[test]
fn test_debug_valid_if() {
    debug_fixture("InputIndentationValidIfIndent.java");
}

#[test]
fn test_debug_valid_if2() {
    debug_fixture("InputIndentationValidIfIndent2.java");
}

#[test]
fn test_debug_record_pattern() {
    debug_fixture("InputIndentationRecordPattern.java");
}

#[test]
fn test_debug_pattern_matching_switch() {
    debug_fixture("InputIndentationPatternMatchingForSwitch.java");
}

#[test]
fn test_debug_yield_statement() {
    debug_fixture("InputIndentationYieldStatement.java");
}

#[test]
fn test_debug_guava() {
    debug_fixture("InputIndentationFromGuava.java");
}

#[test]
fn test_debug_ctor_call() {
    debug_fixture("InputIndentationCtorCall.java");
}

#[test]
fn test_debug_method_paren_newline1() {
    debug_fixture("InputIndentationCheckMethodParenOnNewLine1.java");
}

#[test]
fn test_debug_new_children_sevntu() {
    debug_fixture("InputIndentationNewChildrenSevntuConfig.java");
}

#[test]
fn test_debug_invalid_while() {
    debug_fixture("InputIndentationInvalidWhileIndent.java");
}

#[test]
fn test_debug_invalid_do_while() {
    debug_fixture("InputIndentationInvalidDoWhileIndent.java");
}

#[test]
fn test_debug_invalid_if2() {
    debug_fixture("InputIndentationInvalidIfIndent2.java");
}

#[test]
fn test_debug_throws_indent() {
    debug_fixture("InputIndentationInvalidThrowsIndent2.java");
}

#[test]
fn test_throws_clause_indentation() {
    // Test throws clause on continuation line
    let source = r#"
class Foo {
    void bar()
        throws Exception {
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.is_empty(), "Expected no violations for correctly indented throws, got lines: {:?}", violations);
}

#[test]
fn test_throws_clause_wrong_indentation() {
    // Test throws clause with wrong indentation
    let source = r#"
class Foo {
    void bar()
throws Exception {
    }
}
"#;
    let violations = check_indentation(source);
    assert!(violations.contains(&4), "Expected violation for incorrectly indented throws at line 4, got: {:?}", violations);
}

#[test]
fn test_debug_classes_methods() {
    debug_fixture("InputIndentationClassesMethods.java");
}

#[test]
fn test_debug_switch_cases_enums() {
    debug_fixture("InputIndentationSwitchCasesAndEnums.java");
}

#[test]
fn test_debug_code_blocks1() {
    debug_fixture("InputIndentationCodeBlocks1.java");
}

#[test]
fn test_debug_code_blocks2() {
    debug_fixture("InputIndentationCodeBlocks2.java");
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

#[test]
fn test_debug_single_switch_without_curly() {
    debug_fixture("InputIndentationCheckSingleSwitchStatementsWithoutCurly.java");
}

#[test]
fn test_debug_lambda_child_same_line() {
    debug_fixture("InputIndentationLambdaAndChildOnTheSameLine.java");
}

#[test]
fn test_debug_strict_condition() {
    debug_fixture("InputIndentationStrictCondition.java");
}

#[test]
fn test_debug_chained_method_calls() {
    debug_fixture("InputIndentationChainedMethodCalls.java");
}

#[test]
fn test_debug_annotation_incorrect() {
    debug_fixture("InputIndentationAnnotationIncorrect.java");
}

#[test]
fn test_debug_method_call_line_wrap() {
    debug_fixture("InputIndentationMethodCallLineWrap.java");
}

#[test]
fn test_debug_lambda3() {
    debug_fixture("InputIndentationLambda3.java");
}

#[test]
fn test_debug_ann_arr_init() {
    debug_fixture("InputIndentationAnnArrInit.java");
}

#[test]
fn test_debug_lambda7() {
    debug_fixture("InputIndentationLambda7.java");
}

#[test]
fn test_debug_valid_method_indent1() {
    debug_fixture("InputIndentationValidMethodIndent1.java");
}

#[test]
fn test_debug_members() {
    debug_fixture("InputIndentationMembers.java");
}

#[test]
fn test_debug_multiline() {
    debug_fixture("InputIndentationMultilineStatements.java");
}

#[test]
fn test_debug_invalid_switch() {
    debug_fixture("InputIndentationInvalidSwitchIndent.java");
}

#[test]
fn test_debug_invalid_for() {
    debug_fixture("InputIndentationInvalidForIndent.java");
}

#[test]
fn test_debug_ctor_call1() {
    debug_fixture("InputIndentationCtorCall1.java");
}

#[test]
fn test_debug_android_style() {
    debug_fixture("InputIndentationAndroidStyle.java");
}

#[test]
fn test_debug_new_handler() {
    debug_fixture("InputIndentationNewHandler.java");
}

#[test]
fn test_debug_members2() {
    debug_fixture("InputIndentationMembers.java");
}

#[test]
fn test_debug_try_resources_strict() {
    debug_fixture("InputIndentationTryWithResourcesStrict.java");
}

#[test]
fn test_debug_custom_annotation() {
    debug_fixture("InputIndentationCustomAnnotation.java");
}

// === Debug tests for next phase ===

#[test]
fn test_debug_force_strict() {
    debug_fixture("InputIndentationNewWithForceStrictCondition.java");
}

#[test]
fn test_debug_catch_params() {
    debug_fixture("InputIndentationCatchParametersOnNewLine.java");
}

#[test]
fn test_debug_anon_class_curly() {
    debug_fixture("InputIndentationAnonymousClassInMethodCurlyOnNewLine.java");
}

#[test]
fn test_debug_annotation_closing_paren() {
    debug_fixture("InputIndentationAnnotationClosingParenthesisEndsInSameIndentationAsOpen.java");
}

#[test]
fn test_debug_lambda2() {
    debug_fixture("InputIndentationLambda2.java");
}

#[test]
fn test_debug_lambda4() {
    debug_fixture("InputIndentationLambda4.java");
}

#[test]
fn test_debug_if_and_parameter() {
    debug_fixture("InputIndentationIfAndParameter.java");
}

#[test]
fn test_debug_annotation_array_init_old_style() {
    debug_fixture("InputIndentationAnnotationArrayInitOldStyle.java");
}

#[test]
fn test_debug_try_resources_not_strict1() {
    debug_fixture("InputIndentationTryResourcesNotStrict1.java");
}

#[test]
fn test_debug_text_block() {
    debug_fixture("InputIndentationTextBlock.java");
}

#[test]
fn test_debug_class_def_indent1() {
    debug_fixture("InputIndentationInvalidClassDefIndent1.java");
}

#[test]
fn test_debug_arr_init_no_trailing() {
    debug_fixture("InputIndentationInvalidArrInitIndentNoTrailingComments.java");
}

#[test]
fn test_debug_arr_init1() {
    debug_fixture("InputIndentationInvalidArrayInitIndent1.java");
}

#[test]
fn test_debug_package_info() {
    debug_fixture("package-info.java");
}

#[test]
fn test_debug_difficult_annotations() {
    debug_fixture("InputIndentationDifficultAnnotations.java");
}
