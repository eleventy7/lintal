//! WhitespaceAfter checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::WhitespaceAfter;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::{HashMap, HashSet};

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    token: String,
}

impl Violation {
    #[allow(dead_code)]
    fn not_followed(line: usize, column: usize, token: &str) -> Self {
        Self {
            line,
            column,
            token: token.to_string(),
        }
    }
}

/// Configuration for WhitespaceAfter rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct WhitespaceAfterConfig {
    tokens: Option<Vec<String>>,
}

impl WhitespaceAfterConfig {
    fn default_config() -> Self {
        Self { tokens: None }
    }

    fn to_rule(&self) -> WhitespaceAfter {
        if let Some(ref tokens) = self.tokens {
            let mut token_set = HashSet::new();
            for token in tokens {
                use lintal_linter::rules::whitespace::whitespace_after::WhitespaceAfterToken;
                match token.as_str() {
                    "COMMA" => {
                        token_set.insert(WhitespaceAfterToken::Comma);
                    }
                    "SEMI" => {
                        token_set.insert(WhitespaceAfterToken::Semi);
                    }
                    "TYPECAST" => {
                        token_set.insert(WhitespaceAfterToken::Typecast);
                    }
                    "LITERAL_IF" => {
                        token_set.insert(WhitespaceAfterToken::LiteralIf);
                    }
                    "LITERAL_ELSE" => {
                        token_set.insert(WhitespaceAfterToken::LiteralElse);
                    }
                    "LITERAL_WHILE" => {
                        token_set.insert(WhitespaceAfterToken::LiteralWhile);
                    }
                    "LITERAL_DO" => {
                        token_set.insert(WhitespaceAfterToken::LiteralDo);
                    }
                    "LITERAL_FOR" => {
                        token_set.insert(WhitespaceAfterToken::LiteralFor);
                    }
                    "DO_WHILE" => {
                        token_set.insert(WhitespaceAfterToken::DoWhile);
                    }
                    _ => {}
                }
            }
            WhitespaceAfter { tokens: token_set }
        } else {
            WhitespaceAfter::default()
        }
    }
}

/// Run WhitespaceAfter rule on source and collect violations.
fn check_whitespace_after(source: &str) -> Vec<Violation> {
    check_whitespace_after_with_config(source, &WhitespaceAfterConfig::default_config())
}

/// Run WhitespaceAfter rule with custom config on source and collect violations.
fn check_whitespace_after_with_config(
    source: &str,
    config: &WhitespaceAfterConfig,
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
            let token = extract_token(&diagnostic.kind.body);

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                token,
            });
        }
    }

    violations
}

/// Extract token from message like "',' is not followed by whitespace"
fn extract_token(message: &str) -> String {
    // Look for pattern: 'X' is not followed
    if let Some(start) = message.find('\'')
        && let Some(end) = message[start + 1..].find('\'')
    {
        return message[start + 1..start + 1 + end].to_string();
    }
    message.to_string()
}

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("whitespaceafter", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Extract expected violations from inline comments in test file.
/// Format: // violation '',' is not followed by whitespace'
fn extract_expected_violations(source: &str) -> Vec<(usize, String)> {
    let mut violations = vec![];
    for (line_num, line) in source.lines().enumerate() {
        if let Some(comment_start) = line.find("// violation") {
            let comment = &line[comment_start..];
            // Extract token from pattern: ''X' is not followed'
            if let Some(start) = comment.find("''") {
                let after_quote = &comment[start + 2..];
                if let Some(end) = after_quote.find("'") {
                    let token = after_quote[..end].to_string();
                    violations.push((line_num + 1, token)); // 1-indexed
                }
            }
        }
    }
    violations
}

/// Helper to verify violations match expected.
#[allow(dead_code)]
fn verify_violations(violations: &[Violation], expected_lines: &[usize], token: &str) -> bool {
    for line in expected_lines {
        if !violations
            .iter()
            .any(|v| v.line == *line && v.token == token)
        {
            return false;
        }
    }
    true
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}: `{}`", v.line, v.column, v.token);
    }
}

/// Group violations by line for analysis.
#[allow(dead_code)]
fn violations_by_line(violations: &[Violation]) -> HashMap<usize, Vec<&Violation>> {
    let mut by_line: HashMap<usize, Vec<&Violation>> = HashMap::new();
    for v in violations {
        by_line.entry(v.line).or_default().push(v);
    }
    by_line
}

// =============================================================================
// Test: testDefaultConfig
// File: InputWhitespaceAfterDefaultConfig.java
// Expected: violations on lines 45, 74 for comma
// =============================================================================

#[test]
fn test_whitespace_after_default_config() {
    let Some(source) = load_fixture("InputWhitespaceAfterDefaultConfig.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = extract_expected_violations(&source);
    println!("Expected violations from comments: {:?}", expected);

    let violations = check_whitespace_after(&source);
    print_violations("Actual violations", &violations);

    // Verify we found violations on expected lines
    let expected_lines = vec![45, 74];
    for line in &expected_lines {
        assert!(
            violations.iter().any(|v| v.line == *line && v.token == ","),
            "Missing comma violation on line {}",
            line
        );
    }

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralFor
// File: InputWhitespaceAfterFor.java
// Expected: violations on lines 18, 21 for semicolon
// =============================================================================

#[test]
fn test_whitespace_after_for() {
    let Some(source) = load_fixture("InputWhitespaceAfterFor.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_after(&source);
    print_violations("Actual violations", &violations);

    // Expected violations on lines 18, 21 for semicolon
    let expected_lines = vec![18, 21];
    for line in &expected_lines {
        assert!(
            violations.iter().any(|v| v.line == *line && v.token == ";"),
            "Missing semicolon violation on line {}",
            line
        );
    }

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralDoWhile
// File: InputWhitespaceAfterDoWhile.java
// Expected: violation on line 25 for 'while' keyword
// =============================================================================

#[test]
fn test_whitespace_after_do_while() {
    let Some(source) = load_fixture("InputWhitespaceAfterDoWhile.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check DO_WHILE token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "LITERAL_DO".to_string(),
            "DO_WHILE".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: while on line 25
    assert!(
        violations
            .iter()
            .any(|v| v.line == 25 && v.token == "while"),
        "Missing 'while' violation on line 25"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testTypeCast
// File: InputWhitespaceAfterTypeCast.java
// Expected: violation on line 91 for typecast
// =============================================================================

#[test]
fn test_whitespace_after_typecast() {
    let Some(source) = load_fixture("InputWhitespaceAfterTypeCast.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check TYPECAST token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "TYPECAST".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: typecast on line 91
    assert!(
        violations.iter().any(|v| v.line == 91 && v.token == ")"),
        "Missing typecast violation on line 91"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralIf
// File: InputWhitespaceAfterLiteralIf.java
// Expected: violation on line 25 for 'if' keyword
// =============================================================================

#[test]
fn test_whitespace_after_literal_if() {
    let Some(source) = load_fixture("InputWhitespaceAfterLiteralIf.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check LITERAL_IF token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "LITERAL_IF".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: 'if' on line 25
    assert!(
        violations.iter().any(|v| v.line == 25 && v.token == "if"),
        "Missing 'if' violation on line 25"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralWhile
// File: InputWhitespaceAfterLiteralWhile.java
// Expected: violation on line 46 for 'while' keyword
// =============================================================================

#[test]
fn test_whitespace_after_literal_while() {
    let Some(source) = load_fixture("InputWhitespaceAfterLiteralWhile.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check LITERAL_WHILE token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "LITERAL_WHILE".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: 'while' on line 46
    assert!(
        violations
            .iter()
            .any(|v| v.line == 46 && v.token == "while"),
        "Missing 'while' violation on line 46"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralElse
// File: InputWhitespaceAfterLiteralElse.java
// Expected: violation on line 34 for 'else' keyword
// =============================================================================

#[test]
fn test_whitespace_after_literal_else() {
    let Some(source) = load_fixture("InputWhitespaceAfterLiteralElse.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check LITERAL_ELSE token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "LITERAL_ELSE".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: 'else' on line 34
    assert!(
        violations.iter().any(|v| v.line == 34 && v.token == "else"),
        "Missing 'else' violation on line 34"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testLiteralDo
// File: InputWhitespaceAfterLiteralDo.java
// Expected: violation on line 70 for 'do' keyword
// =============================================================================

#[test]
fn test_whitespace_after_literal_do() {
    let Some(source) = load_fixture("InputWhitespaceAfterLiteralDo.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Configure to check LITERAL_DO token
    let config = WhitespaceAfterConfig {
        tokens: Some(vec![
            "COMMA".to_string(),
            "SEMI".to_string(),
            "LITERAL_DO".to_string(),
        ]),
    };
    let violations = check_whitespace_after_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected: 'do' on line 70
    assert!(
        violations.iter().any(|v| v.line == 70 && v.token == "do"),
        "Missing 'do' violation on line 70"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: inline basic tests
// =============================================================================

#[test]
fn test_comma_without_space() {
    let violations = check_whitespace_after("class Foo { int[] a = {1,2}; }");
    assert!(
        violations.iter().any(|v| v.token == ","),
        "Should detect comma without space: {:?}",
        violations
    );
}

#[test]
fn test_comma_with_space() {
    let violations = check_whitespace_after("class Foo { int[] a = {1, 2}; }");
    let comma_violations: Vec<_> = violations.iter().filter(|v| v.token == ",").collect();
    assert!(
        comma_violations.is_empty(),
        "Should not flag comma with space"
    );
}

#[test]
fn test_semicolon_end_of_line() {
    let violations = check_whitespace_after("class Foo { int x = 1;\n}");
    let semi_violations: Vec<_> = violations.iter().filter(|v| v.token == ";").collect();
    assert!(
        semi_violations.is_empty(),
        "Should not flag semicolon at EOL"
    );
}
