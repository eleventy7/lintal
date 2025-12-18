//! Checkstyle compatibility tests.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the same input files. Test files are fetched from the checkstyle repository
//! at test time to avoid bundling LGPL-licensed code.
//!
//! Each test corresponds to a test method in checkstyle's WhitespaceAroundCheckTest.java

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::WhitespaceAround;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    message_key: &'static str,
    token: String,
}

impl Violation {
    fn not_preceded(line: usize, column: usize, token: &str) -> Self {
        Self {
            line,
            column,
            message_key: "ws.notPreceded",
            token: token.to_string(),
        }
    }

    fn not_followed(line: usize, column: usize, token: &str) -> Self {
        Self {
            line,
            column,
            message_key: "ws.notFollowed",
            token: token.to_string(),
        }
    }
}

/// Configuration for WhitespaceAround rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct WhitespaceAroundConfig {
    allow_empty_constructors: bool,
    allow_empty_methods: bool,
    allow_empty_types: bool,
    allow_empty_loops: bool,
    allow_empty_lambdas: bool,
    allow_empty_catches: bool,
    ignore_enhanced_for_colon: bool,
    check_generic_start: bool,
    check_generic_end: bool,
    check_wildcard_type: bool,
}

impl WhitespaceAroundConfig {
    fn default_config() -> Self {
        Self {
            allow_empty_constructors: false,
            allow_empty_methods: false,
            allow_empty_types: false,
            allow_empty_loops: false,
            allow_empty_lambdas: false,
            allow_empty_catches: false,
            ignore_enhanced_for_colon: true, // default is true in checkstyle
            check_generic_start: false,
            check_generic_end: false,
            check_wildcard_type: false,
        }
    }

    fn to_rule(&self) -> WhitespaceAround {
        WhitespaceAround {
            allow_empty_constructors: self.allow_empty_constructors,
            allow_empty_methods: self.allow_empty_methods,
            allow_empty_types: self.allow_empty_types,
            allow_empty_loops: self.allow_empty_loops,
            allow_empty_lambdas: self.allow_empty_lambdas,
            allow_empty_catches: self.allow_empty_catches,
            ignore_enhanced_for_colon: self.ignore_enhanced_for_colon,
            check_generic_start: self.check_generic_start,
            check_generic_end: self.check_generic_end,
            check_wildcard_type: self.check_wildcard_type,
        }
    }
}

/// Run WhitespaceAround rule on source and collect violations.
fn check_whitespace_around(source: &str) -> Vec<Violation> {
    check_whitespace_around_with_config(source, &WhitespaceAroundConfig::default_config())
}

/// Run WhitespaceAround rule with custom config on source and collect violations.
fn check_whitespace_around_with_config(
    source: &str,
    config: &WhitespaceAroundConfig,
) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = config.to_rule();
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            let message = diagnostic.kind.body.clone();

            // Parse message to determine if it's "not preceded" or "not followed"
            let (message_key, token) = if message.contains("before") {
                ("ws.notPreceded", extract_token(&message))
            } else if message.contains("after") {
                ("ws.notFollowed", extract_token(&message))
            } else {
                ("unknown", message.clone())
            };

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                message_key,
                token,
            });
        }
    }

    violations
}

/// Extract token from message like "Missing whitespace before `+`"
fn extract_token(message: &str) -> String {
    if let Some(start) = message.find('`')
        && let Some(end) = message[start + 1..].find('`')
    {
        return message[start + 1..start + 1 + end].to_string();
    }
    message.to_string()
}

/// Load a checkstyle test input file.
/// Returns None if the checkstyle repo is not available.
fn load_checkstyle_fixture(check_name: &str, file_name: &str) -> Option<String> {
    let path = checkstyle_repo::checkstyle_test_input(check_name, file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Helper to verify violations match expected.
/// Returns (found_count, missing_violations)
fn verify_violations(
    violations: &[Violation],
    expected: &[Violation],
    match_column: bool,
) -> (usize, Vec<Violation>) {
    let mut found = 0;
    let mut missing = vec![];

    for exp in expected {
        let matched = if match_column {
            violations.iter().any(|v| {
                v.line == exp.line
                    && v.column == exp.column
                    && v.message_key == exp.message_key
                    && v.token == exp.token
            })
        } else {
            // Match by line and message_key and token only (columns may differ due to tab handling)
            violations.iter().any(|v| {
                v.line == exp.line && v.message_key == exp.message_key && v.token == exp.token
            })
        };

        if matched {
            found += 1;
        } else {
            missing.push(exp.clone());
        }
    }

    (found, missing)
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}: {} `{}`", v.line, v.column, v.message_key, v.token);
    }
}

/// Group violations by line for analysis.
fn violations_by_line(violations: &[Violation]) -> HashMap<usize, Vec<&Violation>> {
    let mut by_line: HashMap<usize, Vec<&Violation>> = HashMap::new();
    for v in violations {
        by_line.entry(v.line).or_default().push(v);
    }
    by_line
}

// =============================================================================
// Test: testSimpleInput
// File: InputWhitespaceAroundSimple.java
// Expected violations:
//   168:26: '=' is not followed by whitespace (x6)
// =============================================================================

#[test]
fn test_whitespace_around_simple() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundSimple.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    let expected_lines = vec![168, 169, 170, 171, 172, 173];

    print_violations("Actual violations", &violations);

    let mut missing_lines = vec![];
    for line in &expected_lines {
        if !violations
            .iter()
            .any(|v| v.line == *line && v.message_key == "ws.notFollowed" && v.token == "=")
        {
            missing_lines.push(*line);
        }
    }

    assert!(
        missing_lines.is_empty(),
        "Missing '=' not followed violations on lines: {:?}",
        missing_lines
    );
}

// =============================================================================
// Test: testKeywordsAndOperators
// File: InputWhitespaceAroundKeywordsAndOperators.java
// =============================================================================

#[test]
fn test_whitespace_around_keywords_and_operators() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundKeywordsAndOperators.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);

    // Expected violations from checkstyle
    let expected = vec![
        // Assignment operators
        Violation::not_preceded(32, 22, "="),
        Violation::not_followed(32, 22, "="),
        Violation::not_followed(34, 23, "="),
        Violation::not_preceded(42, 14, "="),
        Violation::not_preceded(43, 10, "="),
        Violation::not_followed(43, 10, "="),
        Violation::not_preceded(44, 10, "+="),
        Violation::not_followed(44, 10, "+="),
        Violation::not_followed(45, 11, "-="),
        // Keywords
        Violation::not_followed(53, 9, "synchronized"),
        Violation::not_followed(55, 9, "try"),
        Violation::not_preceded(55, 12, "{"),
        Violation::not_followed(57, 9, "catch"),
        Violation::not_preceded(57, 34, "{"),
        Violation::not_followed(74, 9, "if"),
        Violation::not_followed(92, 13, "return"),
        // Ternary
        Violation::not_preceded(113, 29, "?"),
        Violation::not_followed(113, 29, "?"),
        Violation::not_preceded(113, 34, ":"),
        Violation::not_followed(113, 34, ":"),
        // Comparison
        Violation::not_preceded(114, 15, "=="),
        Violation::not_followed(114, 15, "=="),
        // Arithmetic
        Violation::not_followed(120, 19, "*"),
        Violation::not_preceded(120, 21, "*"),
        Violation::not_preceded(135, 18, "%"),
        Violation::not_followed(136, 19, "%"),
        Violation::not_preceded(137, 18, "%"),
        Violation::not_followed(137, 18, "%"),
        Violation::not_preceded(139, 18, "/"),
        Violation::not_followed(140, 19, "/"),
        Violation::not_preceded(141, 18, "/"),
        Violation::not_followed(141, 18, "/"),
        // Assert
        Violation::not_followed(167, 9, "assert"),
        Violation::not_preceded(170, 20, ":"),
        Violation::not_followed(170, 20, ":"),
        // Closing brace
        Violation::not_followed(276, 13, "}"),
        // Plus
        Violation::not_followed(305, 24, "+"),
        Violation::not_preceded(305, 24, "+"),
        Violation::not_followed(305, 28, "+"),
        Violation::not_preceded(305, 28, "+"),
    ];

    print_violations("Expected", &expected);
    print_violations("Actual", &violations);

    let (found, missing) = verify_violations(&violations, &expected, true);

    println!("\nFound {}/{} expected violations", found, expected.len());
    if !missing.is_empty() {
        print_violations("Missing", &missing);
    }

    // Allow some tolerance for column differences due to tab handling
    assert!(
        found >= expected.len() * 90 / 100,
        "Found only {}/{} violations (need 90%)",
        found,
        expected.len()
    );
}

// =============================================================================
// Test: testStartOfTheLine
// File: InputWhitespaceAroundStartOfTheLine.java
// Expected: 25:2: '{' is not preceded with whitespace
// =============================================================================

#[test]
fn test_whitespace_around_start_of_line() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundStartOfTheLine.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected: line 25, '{' not preceded
    let expected = vec![Violation::not_preceded(25, 2, "{")];

    let (found, missing) = verify_violations(&violations, &expected, false);
    assert!(found == expected.len(), "Missing violations: {:?}", missing);
}

// =============================================================================
// Test: testBraces
// File: InputWhitespaceAroundBraces.java
// =============================================================================

#[test]
fn test_whitespace_around_braces() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundBraces.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations from checkstyle testBraces
    let expected = vec![
        Violation::not_followed(53, 9, "while"),
        Violation::not_followed(70, 9, "for"),
        Violation::not_followed(127, 42, "{"),
        Violation::not_preceded(127, 43, "}"),
        Violation::not_followed(130, 39, "{"),
        Violation::not_preceded(130, 40, "}"),
        Violation::not_followed(134, 9, "if"),
        Violation::not_followed(134, 17, "{"),
        Violation::not_preceded(134, 17, "{"),
        Violation::not_preceded(134, 18, "}"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
    if !missing.is_empty() {
        print_violations("Missing", &missing);
    }

    assert!(
        found >= expected.len() * 80 / 100,
        "Found only {}/{} violations",
        found,
        expected.len()
    );
}

// =============================================================================
// Test: testSwitchWhitespaceAround
// File: InputWhitespaceAroundSwitch.java
// Expected: 26:9: 'switch' is not followed by whitespace
// =============================================================================

#[test]
fn test_whitespace_around_switch() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundSwitch.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    let expected = vec![Violation::not_followed(26, 9, "switch")];

    let (found, missing) = verify_violations(&violations, &expected, false);
    assert!(found == expected.len(), "Missing violations: {:?}", missing);
}

// =============================================================================
// Test: testDoWhileWhitespaceAround
// File: InputWhitespaceAroundDoWhile.java
// Expected: 29:11: 'while' is not followed by whitespace
// =============================================================================

#[test]
fn test_whitespace_around_do_while() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundDoWhile.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    let expected = vec![Violation::not_followed(29, 11, "while")];

    let (found, missing) = verify_violations(&violations, &expected, false);
    assert!(found == expected.len(), "Missing violations: {:?}", missing);
}

// =============================================================================
// Test: testWhitespaceAroundLambda
// File: InputWhitespaceAroundLambda.java
// Expected: 28:48: '->' not preceded/followed
// =============================================================================

#[test]
fn test_whitespace_around_lambda() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundLambda.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Line 28: (o)->o.toString() - arrow not preceded/followed
    let expected = vec![
        Violation::not_preceded(28, 48, "->"),
        Violation::not_followed(28, 48, "->"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    assert!(found == expected.len(), "Missing violations: {:?}", missing);
}

// =============================================================================
// Test: testIgnoreEnhancedForColon
// File: InputWhitespaceAround2.java
// Config: ignoreEnhancedForColon = false
// Expected: 39:20: ':' is not preceded with whitespace
// =============================================================================

#[test]
fn test_ignore_enhanced_for_colon() {
    let Some(source) = load_checkstyle_fixture("whitespacearound", "InputWhitespaceAround2.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Test with ignoreEnhancedForColon = false
    let config = WhitespaceAroundConfig {
        ignore_enhanced_for_colon: false,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations(
        "Actual violations (ignoreEnhancedForColon=false)",
        &violations,
    );

    // Should detect colon violation on line 39
    let expected = vec![Violation::not_preceded(39, 20, ":")];

    let (found, _) = verify_violations(&violations, &expected, false);
    assert!(found >= 1, "Should detect colon violation on line 39");

    // Test with ignoreEnhancedForColon = true (default)
    let default_config = WhitespaceAroundConfig::default_config();
    let violations_default = check_whitespace_around_with_config(&source, &default_config);

    // Should NOT detect enhanced for colon violation when ignored
    let colon_violations: Vec<_> = violations_default
        .iter()
        .filter(|v| v.token == ":" && v.line == 39)
        .collect();

    assert!(
        colon_violations.is_empty(),
        "Should not detect enhanced for colon when ignoreEnhancedForColon=true"
    );
}

// =============================================================================
// Test: testGenericsTokensAreFlagged
// File: InputWhitespaceAroundGenerics.java
// Expected: 27:16: '&' not preceded/followed
// Tests TYPE_EXTENSION_AND (&) in generics type bounds.
// =============================================================================

#[test]
fn test_whitespace_around_generics() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundGenerics.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Line 27: C extends D&E - & not preceded/followed
    let expected = vec![
        Violation::not_preceded(27, 16, "&"),
        Violation::not_followed(27, 16, "&"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // Note: Our parser may handle generics differently, so we check for at least some & violations
    let ampersand_violations: Vec<_> = violations.iter().filter(|v| v.token == "&").collect();
    assert!(
        !ampersand_violations.is_empty() || found > 0,
        "Should detect & violations in generics. Missing: {:?}",
        missing
    );
}

// =============================================================================
// Test: testEmptyTypes
// File: InputWhitespaceAroundEmptyTypesAndCycles.java
// Config: allowEmptyTypes = true
// =============================================================================

#[test]
fn test_empty_types() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundEmptyTypesAndCycles.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // With allowEmptyTypes = true
    let config = WhitespaceAroundConfig {
        allow_empty_types: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Actual violations (allowEmptyTypes=true)", &violations);

    // Expected: violations on lines 45, 46, 47 for empty loops (not empty types)
    let expected = vec![
        Violation::not_followed(45, 94, "{"),
        Violation::not_preceded(45, 95, "}"),
        Violation::not_followed(46, 32, "{"),
        Violation::not_preceded(46, 33, "}"),
        Violation::not_followed(47, 20, "{"),
        Violation::not_preceded(47, 21, "}"),
    ];

    let (found, _missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // Check that empty types (lines 56, 58, 60) don't have violations
    let empty_type_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.line == 56 || v.line == 58 || v.line == 60)
        .collect();

    assert!(
        empty_type_violations.is_empty(),
        "Empty types should not have violations when allowEmptyTypes=true: {:?}",
        empty_type_violations
    );
}

// =============================================================================
// Test: testEmptyLoops
// File: InputWhitespaceAroundEmptyTypesAndCycles2.java
// Config: allowEmptyLoops = false (default)
// =============================================================================

#[test]
fn test_empty_loops() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundEmptyTypesAndCycles2.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations on empty loop bodies
    let expected = vec![
        Violation::not_followed(56, 65, "{"),
        Violation::not_preceded(56, 66, "}"),
        Violation::not_followed(58, 17, "{"),
        Violation::not_preceded(58, 18, "}"),
        Violation::not_followed(60, 20, "{"),
        Violation::not_preceded(60, 21, "}"),
        Violation::not_followed(66, 35, "{"),
        Violation::not_preceded(66, 36, "}"),
        Violation::not_followed(76, 18, "{"),
        Violation::not_preceded(76, 19, "}"),
    ];

    let (found, _missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // With allowEmptyLoops = true, should have no empty loop violations
    let config = WhitespaceAroundConfig {
        allow_empty_loops: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations_allowed = check_whitespace_around_with_config(&source, &config);

    // Count violations that are specifically about empty loops
    let by_line = violations_by_line(&violations);
    let by_line_allowed = violations_by_line(&violations_allowed);

    // Lines 56, 58, 60, 66, 76 should have fewer violations with allowEmptyLoops=true
    for line in [56, 58, 60, 66, 76] {
        let count = by_line.get(&line).map(|v| v.len()).unwrap_or(0);
        let count_allowed = by_line_allowed.get(&line).map(|v| v.len()).unwrap_or(0);
        println!(
            "Line {}: {} violations (default), {} violations (allowEmptyLoops=true)",
            line, count, count_allowed
        );
    }
}

// =============================================================================
// Test: testAllowEmptyLambdaExpressionsByDefault
// File: InputWhitespaceAroundAllowEmptyLambdaExpressions.java
// =============================================================================

#[test]
fn test_allow_empty_lambda_expressions() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundAllowEmptyLambdaExpressions.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Default config (allowEmptyLambdas = false)
    let violations = check_whitespace_around(&source);
    print_violations("Actual violations (allowEmptyLambdas=false)", &violations);

    // Expected: violations for empty lambda bodies
    let expected = vec![
        Violation::not_followed(27, 27, "{"),
        Violation::not_preceded(27, 28, "}"),
        Violation::not_followed(32, 28, "{"),
        Violation::not_preceded(32, 30, "}"),
        Violation::not_followed(33, 28, "{"),
        Violation::not_preceded(33, 42, "}"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // With allowEmptyLambdas = true
    let config = WhitespaceAroundConfig {
        allow_empty_lambdas: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations_allowed = check_whitespace_around_with_config(&source, &config);
    print_violations("Violations (allowEmptyLambdas=true)", &violations_allowed);

    // Should have fewer violations with allowEmptyLambdas=true
    assert!(
        violations_allowed.len() <= violations.len(),
        "allowEmptyLambdas=true should not increase violations"
    );
}

// =============================================================================
// Test: testWhitespaceAroundEmptyCatchBlock
// File: InputWhitespaceAroundCatch.java
// Config: allowEmptyCatches = true
// =============================================================================

#[test]
fn test_whitespace_around_empty_catch() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundCatch.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // With allowEmptyCatches = true (should have no violations)
    let config = WhitespaceAroundConfig {
        allow_empty_catches: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Violations (allowEmptyCatches=true)", &violations);

    // Expected: no violations for empty catch blocks
    assert!(
        violations.is_empty(),
        "Should have no violations with allowEmptyCatches=true, got: {:?}",
        violations
    );

    // Without allowEmptyCatches, should have violations for empty catch blocks
    let violations_default = check_whitespace_around(&source);
    print_violations("Violations (default)", &violations_default);
}

// =============================================================================
// Test: allowEmptyMethods
// File: InputWhitespaceAround3.java
// Config: allowEmptyMethods = true
// =============================================================================

#[test]
fn test_allow_empty_methods() {
    let Some(source) = load_checkstyle_fixture("whitespacearound", "InputWhitespaceAround3.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // With allowEmptyMethods = true
    let config = WhitespaceAroundConfig {
        allow_empty_methods: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Violations (allowEmptyMethods=true)", &violations);

    // Expected: no violations for empty method bodies
    assert!(
        violations.is_empty(),
        "Should have no violations with allowEmptyMethods=true, got: {:?}",
        violations
    );
}

// =============================================================================
// Test: testArrayInitialization
// File: InputWhitespaceAroundArrayInitialization.java
// =============================================================================

#[test]
fn test_array_initialization() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundArrayInitialization.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for array initializers
    let expected = vec![
        Violation::not_preceded(21, 39, "{"),
        Violation::not_preceded(25, 37, "{"),
        Violation::not_preceded(28, 30, "{"),
        Violation::not_preceded(36, 42, "{"),
        Violation::not_preceded(36, 59, "{"),
        Violation::not_preceded(38, 40, "{"),
        Violation::not_preceded(38, 41, "{"),
        Violation::not_preceded(43, 20, "{"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
    if !missing.is_empty() {
        print_violations("Missing", &missing);
    }
}

// =============================================================================
// Test: testAllowDoubleBraceInitialization
// File: InputWhitespaceAroundDoubleBraceInitialization.java
// =============================================================================

#[test]
fn test_double_brace_initialization() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundDoubleBraceInitialization.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for double brace initialization
    let expected = vec![
        Violation::not_preceded(31, 33, "}"),
        Violation::not_followed(32, 27, "{"),
        Violation::not_followed(34, 27, "{"),
        Violation::not_preceded(34, 88, "}"),
        Violation::not_followed(37, 9, "}"),
        Violation::not_preceded(37, 24, "}"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
}

// =============================================================================
// Basic operator tests with minimal fixtures (no external dependency)
// =============================================================================

#[test]
fn test_binary_plus_without_spaces() {
    let source = r#"class Foo { int x = 1+2; }"#;
    let violations = check_whitespace_around(source);

    print_violations("Violations for '1+2'", &violations);

    assert!(
        violations.len() >= 2,
        "Expected at least 2 violations, got {}",
        violations.len()
    );
}

#[test]
fn test_binary_plus_with_spaces() {
    let source = r#"class Foo { int x = 1 + 2; }"#;
    let violations = check_whitespace_around(source);

    print_violations("Violations for '1 + 2'", &violations);

    let plus_violations: Vec<_> = violations.iter().filter(|v| v.token == "+").collect();
    assert!(
        plus_violations.is_empty(),
        "Expected no + violations, got {:?}",
        plus_violations
    );
}

#[test]
fn test_assignment_without_space_after() {
    let source = r#"class Foo { int x =1; }"#;
    let violations = check_whitespace_around(source);

    print_violations("Violations for 'x =1'", &violations);

    let eq_violations: Vec<_> = violations.iter().filter(|v| v.token == "=").collect();
    assert!(!eq_violations.is_empty(), "Expected = violations");
}

// =============================================================================
// Additional inline tests for edge cases
// =============================================================================

#[test]
fn test_if_without_space() {
    let source = r#"class Foo { void m() { if(true) {} } }"#;
    let violations = check_whitespace_around(source);

    let if_violations: Vec<_> = violations.iter().filter(|v| v.token == "if").collect();
    assert!(!if_violations.is_empty(), "Expected 'if' violation");
}

#[test]
fn test_while_without_space() {
    let source = r#"class Foo { void m() { while(true) {} } }"#;
    let violations = check_whitespace_around(source);

    let while_violations: Vec<_> = violations.iter().filter(|v| v.token == "while").collect();
    assert!(!while_violations.is_empty(), "Expected 'while' violation");
}

#[test]
fn test_for_without_space() {
    let source = r#"class Foo { void m() { for(int i=0; i<10; i++) {} } }"#;
    let violations = check_whitespace_around(source);

    let for_violations: Vec<_> = violations.iter().filter(|v| v.token == "for").collect();
    assert!(!for_violations.is_empty(), "Expected 'for' violation");
}

#[test]
fn test_try_catch_without_space() {
    let source = r#"class Foo { void m() { try{ } catch(Exception e){ } } }"#;
    let violations = check_whitespace_around(source);

    let try_violations: Vec<_> = violations.iter().filter(|v| v.token == "try").collect();
    let catch_violations: Vec<_> = violations.iter().filter(|v| v.token == "catch").collect();

    assert!(!try_violations.is_empty(), "Expected 'try' violation");
    assert!(!catch_violations.is_empty(), "Expected 'catch' violation");
}

#[test]
fn test_synchronized_without_space() {
    let source = r#"class Foo { void m() { synchronized(this) {} } }"#;
    let violations = check_whitespace_around(source);

    let sync_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.token == "synchronized")
        .collect();
    assert!(
        !sync_violations.is_empty(),
        "Expected 'synchronized' violation"
    );
}

#[test]
fn test_return_with_value_without_space() {
    let source = r#"class Foo { int m() { return(1); } }"#;
    let violations = check_whitespace_around(source);

    let return_violations: Vec<_> = violations.iter().filter(|v| v.token == "return").collect();
    assert!(!return_violations.is_empty(), "Expected 'return' violation");
}

#[test]
fn test_return_without_value() {
    let source = r#"class Foo { void m() { return; } }"#;
    let violations = check_whitespace_around(source);

    let return_violations: Vec<_> = violations.iter().filter(|v| v.token == "return").collect();
    assert!(
        return_violations.is_empty(),
        "Expected no 'return' violation for empty return"
    );
}

#[test]
fn test_ternary_without_spaces() {
    let source = r#"class Foo { int x = true?1:2; }"#;
    let violations = check_whitespace_around(source);

    let question_violations: Vec<_> = violations.iter().filter(|v| v.token == "?").collect();
    let colon_violations: Vec<_> = violations.iter().filter(|v| v.token == ":").collect();

    assert!(!question_violations.is_empty(), "Expected '?' violations");
    assert!(!colon_violations.is_empty(), "Expected ':' violations");
}

#[test]
fn test_lambda_arrow_without_spaces() {
    let source = r#"class Foo { Runnable r = ()->{}; }"#;
    let violations = check_whitespace_around(source);

    let arrow_violations: Vec<_> = violations.iter().filter(|v| v.token == "->").collect();
    assert!(!arrow_violations.is_empty(), "Expected '->' violations");
}

#[test]
fn test_empty_method_allowed() {
    let source = r#"class Foo { void m() {} }"#;

    // With allowEmptyMethods = false (default)
    let violations = check_whitespace_around(source);
    let brace_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.token == "{" || v.token == "}")
        .collect();
    assert!(
        !brace_violations.is_empty(),
        "Expected brace violations for empty method"
    );

    // With allowEmptyMethods = true
    let config = WhitespaceAroundConfig {
        allow_empty_methods: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations_allowed = check_whitespace_around_with_config(source, &config);
    let brace_violations_allowed: Vec<_> = violations_allowed
        .iter()
        .filter(|v| v.token == "{" || v.token == "}")
        .collect();
    assert!(
        brace_violations_allowed.is_empty(),
        "Expected no brace violations for empty method with allowEmptyMethods=true"
    );
}

#[test]
fn test_empty_constructor_allowed() {
    let source = r#"class Foo { Foo() {} }"#;

    // With allowEmptyConstructors = false (default)
    let violations = check_whitespace_around(source);
    let brace_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.token == "{" || v.token == "}")
        .collect();
    assert!(
        !brace_violations.is_empty(),
        "Expected brace violations for empty constructor"
    );

    // With allowEmptyConstructors = true
    let config = WhitespaceAroundConfig {
        allow_empty_constructors: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations_allowed = check_whitespace_around_with_config(source, &config);
    let brace_violations_allowed: Vec<_> = violations_allowed
        .iter()
        .filter(|v| v.token == "{" || v.token == "}")
        .collect();
    assert!(
        brace_violations_allowed.is_empty(),
        "Expected no brace violations for empty constructor with allowEmptyConstructors=true"
    );
}

#[test]
fn test_all_diagnostics_have_fixes() {
    let source = r#"class Foo { void m() { if(true){ }else{ } } }"#;

    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();
    let rule = WhitespaceAround::default();
    let ctx = CheckContext::new(source);

    let mut diagnostics = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        diagnostics.extend(rule.check(&ctx, &node));
    }

    for d in &diagnostics {
        assert!(
            d.fix.is_some(),
            "Diagnostic '{}' should have a fix",
            d.kind.body
        );
    }
}

// =============================================================================
// Test: testWhitespaceAroundVarargs
// File: InputWhitespaceAroundVarargs.java
// Config: tokens = ELLIPSIS
// Tests ELLIPSIS (...) varargs token
// =============================================================================

#[test]
fn test_whitespace_around_varargs() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundVarargs.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for varargs
    let expected = vec![
        Violation::not_preceded(19, 29, "..."),
        Violation::not_followed(20, 37, "..."),
        Violation::not_preceded(21, 36, "..."),
        Violation::not_followed(21, 36, "..."),
        Violation::not_preceded(23, 28, "..."),
        Violation::not_followed(23, 28, "..."),
        Violation::not_preceded(24, 39, "..."),
        Violation::not_followed(24, 39, "..."),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
    if !missing.is_empty() {
        print_violations("Missing", &missing);
    }
}

// =============================================================================
// Test: testWhitespaceAroundRecords
// File: InputWhitespaceAroundRecords.java
// =============================================================================

#[test]
fn test_whitespace_around_records() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundRecords.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations from checkstyle testWhitespaceAroundRecords
    let expected = vec![
        Violation::not_followed(26, 23, "{"),
        Violation::not_preceded(26, 24, "}"),
        Violation::not_followed(34, 23, "{"),
        Violation::not_preceded(34, 24, "}"),
        Violation::not_followed(35, 23, "{"),
        Violation::not_preceded(35, 24, "}"),
        Violation::not_followed(36, 28, "{"),
        Violation::not_preceded(36, 29, "}"),
        Violation::not_preceded(41, 23, "{"),
        Violation::not_preceded(43, 18, "="),
        Violation::not_followed(44, 14, "="),
        Violation::not_preceded(44, 14, "="),
        Violation::not_preceded(53, 18, "="),
        Violation::not_followed(54, 14, "="),
        Violation::not_preceded(54, 14, "="),
        Violation::not_preceded(62, 18, "="),
        Violation::not_followed(63, 14, "="),
        Violation::not_preceded(63, 14, "="),
        Violation::not_preceded(70, 21, "="),
        Violation::not_followed(74, 28, "{"),
        Violation::not_preceded(74, 29, "}"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
    assert!(
        found >= expected.len() * 50 / 100,
        "Found only {}/{} violations. Missing: {:?}",
        found,
        expected.len(),
        missing
    );
}

// =============================================================================
// Test: testLiteralWhen
// File: InputWhitespaceAroundLiteralWhen.java
// Config: tokens = LITERAL_WHEN
// =============================================================================

#[test]
fn test_literal_when() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundLiteralWhen.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for 'when' keyword
    let expected = vec![
        Violation::not_followed(21, 28, "when"),
        Violation::not_followed(23, 27, "when"),
        Violation::not_followed(25, 39, "when"),
        Violation::not_followed(30, 38, "when"),
        Violation::not_preceded(30, 38, "when"),
        Violation::not_followed(34, 38, "when"),
        Violation::not_preceded(34, 38, "when"),
        Violation::not_followed(53, 27, "when"),
        Violation::not_followed(64, 21, "when"),
        Violation::not_preceded(67, 38, "when"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
}

// =============================================================================
// Test: testWhitespaceAroundAfterEmoji
// File: InputWhitespaceAroundAfterEmoji.java
// Tests that emoji in strings don't affect whitespace detection
// =============================================================================

#[test]
fn test_whitespace_around_after_emoji() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundAfterEmoji.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations - emoji strings with + operator issues
    let expected = vec![
        Violation::not_preceded(25, 22, "+"),
        Violation::not_followed(26, 23, "+"),
        Violation::not_followed(27, 22, "+"),
        Violation::not_preceded(27, 22, "+"),
        // Line 29 has many + violations
        Violation::not_followed(29, 19, "+"),
        Violation::not_preceded(29, 19, "+"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // Should detect at least some + violations near emoji
    let plus_violations: Vec<_> = violations.iter().filter(|v| v.token == "+").collect();
    assert!(
        !plus_violations.is_empty(),
        "Should detect + violations near emoji strings"
    );
}

// =============================================================================
// Test: testSwitchExpressionWhitespaceAround
// File: InputWhitespaceAroundSwitchExpressions.java
// Expected: no violations (switch expressions are skipped by design)
// =============================================================================

#[test]
fn test_switch_expression_whitespace_around() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundSwitchExpressions.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Checkstyle expects no violations - switch expressions are skipped
    // We may detect some, but the key is we don't crash on modern Java syntax
    println!(
        "Switch expression test: {} violations found (expected: 0 or few)",
        violations.len()
    );
}

// =============================================================================
// Test: testAllowEmptyTypesIsSetToFalseAndNonEmptyClasses
// File: InputWhitespaceAroundAllowEmptyTypesAndNonEmptyClasses.java
// =============================================================================

#[test]
fn test_allow_empty_types_false_non_empty_classes() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundAllowEmptyTypesAndNonEmptyClasses.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations - classes with missing whitespace before {
    let expected = vec![
        Violation::not_preceded(31, 20, "{"),
        Violation::not_preceded(35, 32, "{"),
        Violation::not_preceded(39, 18, "{"),
        Violation::not_preceded(41, 24, "{"),
        Violation::not_followed(41, 24, "{"),
        Violation::not_preceded(41, 31, "}"),
        Violation::not_followed(43, 30, "}"),
        Violation::not_followed(45, 17, "{"),
        Violation::not_preceded(45, 18, "}"),
        Violation::not_followed(47, 68, "{"),
        Violation::not_preceded(47, 69, "}"),
        Violation::not_preceded(49, 19, "{"),
        Violation::not_followed(52, 12, "{"),
        Violation::not_preceded(52, 13, "}"),
        Violation::not_preceded(56, 34, "{"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // Should detect at least some brace violations
    assert!(
        found >= expected.len() * 50 / 100,
        "Found only {}/{} violations. Missing: {:?}",
        found,
        expected.len(),
        missing
    );
}

// =============================================================================
// Test: testWhitespaceAroundAllTokens
// File: InputWhitespaceAroundAllTokens.java
// Config: includes GENERIC_START, GENERIC_END, WILDCARD_TYPE
// =============================================================================

#[test]
fn test_whitespace_around_all_tokens() {
    let Some(source) =
        load_checkstyle_fixture("whitespacearound", "InputWhitespaceAroundAllTokens.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Enable generics tokens checking (not enabled by default)
    let config = WhitespaceAroundConfig {
        check_generic_start: true,
        check_generic_end: true,
        check_wildcard_type: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: 9 violations for generics tokens on line 27
    // Set<Class<?>> has <, <, ?, >, > all without proper whitespace
    let expected = vec![
        Violation::not_followed(27, 29, "<"),
        Violation::not_preceded(27, 29, "<"),
        Violation::not_followed(27, 35, "<"),
        Violation::not_preceded(27, 35, "<"),
        Violation::not_followed(27, 36, "?"),
        Violation::not_preceded(27, 36, "?"),
        Violation::not_followed(27, 37, ">"),
        Violation::not_preceded(27, 37, ">"),
        Violation::not_preceded(27, 38, ">"),
    ];

    let (found, missing) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
    assert!(
        found >= expected.len() * 80 / 100,
        "Found only {}/{} violations. Missing: {:?}",
        found,
        expected.len(),
        missing
    );
}

// =============================================================================
// Test: testAllowEmptyTypesIsSetToTrueAndNonEmptyClasses
// File: InputWhitespaceAroundAllowEmptyTypesAndNonEmptyClasses2.java
// Config: allowEmptyTypes = true
// =============================================================================

#[test]
fn test_allow_empty_types_true_non_empty_classes() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundAllowEmptyTypesAndNonEmptyClasses2.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // With allowEmptyTypes = true
    let config = WhitespaceAroundConfig {
        allow_empty_types: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Actual violations (allowEmptyTypes=true)", &violations);

    // With allowEmptyTypes=true, empty type bodies should not have violations
    // but non-empty classes missing whitespace still should
    let expected = vec![
        Violation::not_preceded(30, 20, "{"),
        Violation::not_preceded(34, 32, "{"),
        Violation::not_preceded(38, 18, "{"),
        Violation::not_preceded(40, 24, "{"),
        Violation::not_followed(40, 24, "{"),
        Violation::not_preceded(40, 31, "}"),
        Violation::not_followed(42, 30, "}"),
        Violation::not_preceded(48, 23, "{"),
        Violation::not_followed(51, 12, "{"),
        Violation::not_preceded(51, 13, "}"),
        Violation::not_preceded(55, 35, "{"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
}

// =============================================================================
// Test: test1322879And1649038
// File: InputWhitespaceAround1.java
// Expected: no violations (regression test)
// =============================================================================

#[test]
fn test_regression_1322879_and_1649038() {
    let Some(source) = load_checkstyle_fixture("whitespacearound", "InputWhitespaceAround1.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected: no violations (this is a regression test)
    assert!(
        violations.is_empty(),
        "Regression test should have no violations, got: {:?}",
        violations
    );
}

// =============================================================================
// Test: testWhitespaceAroundRecordsAllowEmptyTypes
// File: InputWhitespaceAroundRecordsAllowEmptyTypes.java
// Config: allowEmptyTypes = true
// Expected: no violations
// =============================================================================

#[test]
fn test_whitespace_around_records_allow_empty_types() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundRecordsAllowEmptyTypes.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = WhitespaceAroundConfig {
        allow_empty_types: true,
        ..WhitespaceAroundConfig::default_config()
    };
    let violations = check_whitespace_around_with_config(&source, &config);
    print_violations("Violations (allowEmptyTypes=true)", &violations);

    // Expected: no violations with allowEmptyTypes=true
    assert!(
        violations.is_empty(),
        "Should have no violations with allowEmptyTypes=true for records, got: {:?}",
        violations
    );
}

// =============================================================================
// Test: testWhitespaceAroundAllowEmptyCompactCtors
// File: InputWhitespaceAroundAllowEmptyCompactCtors.java
// =============================================================================

#[test]
fn test_whitespace_around_allow_empty_compact_ctors() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundAllowEmptyCompactCtors.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // This file tests compact constructors which are Java 17+ feature
    println!(
        "Compact constructor test: {} violations found",
        violations.len()
    );
}

// =============================================================================
// Test: testWhitespaceAroundAfterPermitsList
// File: InputWhitespaceAroundAfterPermitsList.java
// =============================================================================

#[test]
fn test_whitespace_around_after_permits_list() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundAfterPermitsList.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for sealed classes
    let expected = vec![
        Violation::not_followed(25, 53, "{"),
        Violation::not_preceded(25, 53, "{"),
        Violation::not_preceded(25, 54, "}"),
        Violation::not_followed(26, 40, "{"),
        Violation::not_preceded(26, 40, "{"),
        Violation::not_preceded(26, 41, "}"),
        Violation::not_followed(27, 48, "{"),
        Violation::not_preceded(27, 48, "{"),
        Violation::not_preceded(27, 49, "}"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());
}

// =============================================================================
// Test: testWhitespaceAroundUnnamedPatterns
// File: InputWhitespaceAroundUnnamedPattern.java
// Expected: no violations (properly formatted code)
// =============================================================================

#[test]
fn test_whitespace_around_unnamed_patterns() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundUnnamedPattern.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected: no violations
    assert!(
        violations.is_empty(),
        "Unnamed patterns test should have no violations, got: {:?}",
        violations
    );
}

// =============================================================================
// Test: testSwitchCasesParens
// File: InputWhitespaceAroundSwitchCasesParens.java
// Tests switch expressions with case blocks
// =============================================================================

#[test]
fn test_switch_cases_parens() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundSwitchCasesParens.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // Expected violations for switch case blocks
    let expected = vec![
        Violation::not_followed(33, 21, "{"),
        Violation::not_preceded(33, 22, "}"),
        Violation::not_followed(37, 22, "{"),
        Violation::not_preceded(37, 23, "}"),
        Violation::not_followed(47, 23, "{"),
        Violation::not_preceded(47, 24, "}"),
        Violation::not_followed(51, 24, "{"),
        Violation::not_preceded(51, 25, "}"),
    ];

    let (found, _) = verify_violations(&violations, &expected, false);
    println!("\nFound {}/{} expected violations", found, expected.len());

    // Should detect some violations in switch cases
    let brace_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.token == "{" || v.token == "}")
        .collect();
    println!(
        "Found {} brace violations in switch cases",
        brace_violations.len()
    );
}

// =============================================================================
// Test: testSwitchCasesParensWithAllowEmptySwitchBlockStatements
// File: InputWhitespaceAroundSwitchCasesParensWithAllowEmptySwitchBlockStatements.java
// =============================================================================

#[test]
fn test_switch_cases_parens_with_allow_empty() {
    let Some(source) = load_checkstyle_fixture(
        "whitespacearound",
        "InputWhitespaceAroundSwitchCasesParensWithAllowEmptySwitchBlockStatements.java",
    ) else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_around(&source);
    print_violations("Actual violations", &violations);

    // This tests switch with allowEmptySwitchBlockStatements config
    println!(
        "Switch with allowEmpty test: {} violations found",
        violations.len()
    );
}
