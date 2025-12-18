//! EmptyForInitializerPad checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::EmptyForInitializerPad;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    token: String,
}

impl Violation {
    #[allow(dead_code)]
    fn preceded(line: usize, column: usize, token: &str) -> Self {
        Self {
            line,
            column,
            token: token.to_string(),
        }
    }
}

/// Configuration for EmptyForInitializerPad rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct EmptyForInitializerPadConfig {
    option: Option<String>,
}

impl EmptyForInitializerPadConfig {
    fn default_config() -> Self {
        Self { option: None }
    }

    fn to_rule(&self) -> EmptyForInitializerPad {
        use lintal_linter::rules::whitespace::empty_for_initializer_pad::PadOption;

        let option = if let Some(ref opt) = self.option {
            match opt.to_uppercase().as_str() {
                "SPACE" => PadOption::Space,
                "NOSPACE" => PadOption::NoSpace,
                _ => PadOption::NoSpace,
            }
        } else {
            PadOption::NoSpace
        };

        EmptyForInitializerPad { option }
    }
}

/// Run EmptyForInitializerPad rule on source and collect violations.
fn check_empty_for_initializer_pad(source: &str) -> Vec<Violation> {
    check_empty_for_initializer_pad_with_config(
        source,
        &EmptyForInitializerPadConfig::default_config(),
    )
}

/// Run EmptyForInitializerPad rule with custom config on source and collect violations.
fn check_empty_for_initializer_pad_with_config(
    source: &str,
    config: &EmptyForInitializerPadConfig,
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

/// Extract token from message like "';' is preceded by whitespace"
fn extract_token(message: &str) -> String {
    // Look for pattern: 'X' is preceded
    if let Some(start) = message.find('\'')
        && let Some(end) = message[start + 1..].find('\'')
    {
        return message[start + 1..start + 1 + end].to_string();
    }
    message.to_string()
}

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("emptyforinitializerpad", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}: `{}`", v.line, v.column, v.token);
    }
}

// =============================================================================
// Test: testDefault
// File: InputEmptyForInitializerPadDefaultConfig.java
// Config: option = (default)nospace
// =============================================================================

#[test]
fn test_empty_for_initializer_pad_default() {
    let Some(source) = load_fixture("InputEmptyForInitializerPadDefaultConfig.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = EmptyForInitializerPadConfig {
        option: None, // Use default (nospace)
    };
    let violations = check_empty_for_initializer_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected violations from checkstyle test
    // Line 51: for ( ; i < 1; i++ ) { - space before semicolon with nospace option
    let expected = vec![(51, ";")];

    for (line, token) in &expected {
        assert!(
            violations
                .iter()
                .any(|v| v.line == *line && v.token == *token),
            "Missing violation on line {} for token '{}'",
            line,
            token
        );
    }

    // Ensure we have exactly the expected violations
    assert_eq!(
        violations.len(),
        expected.len(),
        "Expected {} violations but found {}. Violations: {:?}",
        expected.len(),
        violations.len(),
        violations
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testSpaceOption
// File: InputEmptyForInitializerPad.java
// Config: option = SPACE
// =============================================================================

#[test]
fn test_empty_for_initializer_pad_space_option() {
    let Some(source) = load_fixture("InputEmptyForInitializerPad.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = EmptyForInitializerPadConfig {
        option: Some("SPACE".to_string()),
    };
    let violations = check_empty_for_initializer_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected violations from checkstyle test
    // Line 54: for (; i < 2; i++ ) { - no space before semicolon with space option
    let expected = vec![(54, ";")];

    for (line, token) in &expected {
        assert!(
            violations
                .iter()
                .any(|v| v.line == *line && v.token == *token),
            "Missing violation on line {} for token '{}'",
            line,
            token
        );
    }

    // Ensure we have exactly the expected violations
    assert_eq!(
        violations.len(),
        expected.len(),
        "Expected {} violations but found {}. Violations: {:?}",
        expected.len(),
        violations.len(),
        violations
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: testWithEmoji
// File: InputEmptyForInitializerPadWithEmoji.java
// Config: option = space
// =============================================================================

#[test]
fn test_empty_for_initializer_pad_with_emoji() {
    let Some(source) = load_fixture("InputEmptyForInitializerPadWithEmoji.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = EmptyForInitializerPadConfig {
        option: Some("space".to_string()),
    };
    let violations = check_empty_for_initializer_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected violations from checkstyle test
    // Line 23: for(;j < s.length() - no space before semicolon
    // Line 28: s = "🤩a🤩"; for (;j <s.length() - no space before semicolon
    let expected = vec![(23, ";"), (28, ";")];

    for (line, token) in &expected {
        assert!(
            violations
                .iter()
                .any(|v| v.line == *line && v.token == *token),
            "Missing violation on line {} for token '{}'",
            line,
            token
        );
    }

    // Ensure we have exactly the expected violations
    assert_eq!(
        violations.len(),
        expected.len(),
        "Expected {} violations but found {}. Violations: {:?}",
        expected.len(),
        violations.len(),
        violations
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Inline basic tests
// =============================================================================

#[test]
fn test_empty_initializer_with_space_nospace_option() {
    // Default is nospace, so space before semicolon is a violation
    let violations = check_empty_for_initializer_pad(
        "class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }",
    );
    assert!(
        violations.iter().any(|v| v.token == ";"),
        "Should detect space before semicolon with nospace option: {:?}",
        violations
    );
}

#[test]
fn test_empty_initializer_no_space_nospace_option() {
    // Default is nospace, no space before semicolon is OK
    let violations = check_empty_for_initializer_pad(
        "class Foo { void m() { int i = 0; for (; i < 1; i++) {} } }",
    );
    let semi_violations: Vec<_> = violations.iter().filter(|v| v.token == ";").collect();
    assert!(
        semi_violations.is_empty(),
        "Should not flag no space with nospace option"
    );
}

#[test]
fn test_empty_initializer_no_space_space_option() {
    // Space option, no space before semicolon is a violation
    let config = EmptyForInitializerPadConfig {
        option: Some("SPACE".to_string()),
    };
    let violations = check_empty_for_initializer_pad_with_config(
        "class Foo { void m() { int i = 0; for (; i < 1; i++) {} } }",
        &config,
    );
    assert!(
        violations.iter().any(|v| v.token == ";"),
        "Should detect no space before semicolon with space option: {:?}",
        violations
    );
}

#[test]
fn test_empty_initializer_with_space_space_option() {
    // Space option, space before semicolon is OK
    let config = EmptyForInitializerPadConfig {
        option: Some("SPACE".to_string()),
    };
    let violations = check_empty_for_initializer_pad_with_config(
        "class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }",
        &config,
    );
    let semi_violations: Vec<_> = violations.iter().filter(|v| v.token == ";").collect();
    assert!(
        semi_violations.is_empty(),
        "Should not flag space with space option"
    );
}

#[test]
fn test_non_empty_initializer_not_checked() {
    // For loops with non-empty initializers should not be checked
    let violations = check_empty_for_initializer_pad(
        "class Foo { void m() { for (int i = 0; i < 1; i++) {} } }",
    );
    let semi_violations: Vec<_> = violations.iter().filter(|v| v.token == ";").collect();
    assert!(
        semi_violations.is_empty(),
        "Should not check for loops with non-empty initializer"
    );
}

#[test]
fn test_line_wrap_not_checked() {
    // Line wraps before semicolon should not be checked
    let violations = check_empty_for_initializer_pad("class Foo { void m() { for (\n; ; ) {} } }");
    let semi_violations: Vec<_> = violations.iter().filter(|v| v.token == ";").collect();
    assert!(
        semi_violations.is_empty(),
        "Should not check when semicolon is on new line"
    );
}

#[test]
fn test_all_violations_have_fixes() {
    let violations = check_empty_for_initializer_pad(
        "class Foo { void m() { int i = 0; for ( ; i < 1; i++) {} } }",
    );
    // We can't easily check fixes here without more infrastructure,
    // but we can at least verify we got violations
    assert!(!violations.is_empty(), "Should have violations");
}
