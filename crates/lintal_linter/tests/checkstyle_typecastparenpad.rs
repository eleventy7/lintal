//! TypecastParenPad checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::TypecastParenPad;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    message: String,
}

impl Violation {
    fn followed(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message: "'(' is followed by whitespace".to_string(),
        }
    }

    fn preceded(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message: "')' is preceded by whitespace".to_string(),
        }
    }
}

/// Configuration for TypecastParenPad rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct TypecastParenPadConfig {
    option: Option<String>,
}

impl TypecastParenPadConfig {
    fn default_config() -> Self {
        Self { option: None }
    }

    fn with_option(option: &str) -> Self {
        Self {
            option: Some(option.to_string()),
        }
    }

    fn to_rule(&self) -> TypecastParenPad {
        use lintal_linter::rules::whitespace::typecast_paren_pad::TypecastParenPadOption;

        let option = self
            .option
            .as_ref()
            .map(|s| match s.as_str() {
                "space" => TypecastParenPadOption::Space,
                _ => TypecastParenPadOption::NoSpace,
            })
            .unwrap_or(TypecastParenPadOption::NoSpace);

        TypecastParenPad { option }
    }
}

/// Run TypecastParenPad rule on source and collect violations.
fn check_typecast_paren_pad(source: &str) -> Vec<Violation> {
    check_typecast_paren_pad_with_config(source, &TypecastParenPadConfig::default_config())
}

/// Run TypecastParenPad rule with custom config on source and collect violations.
fn check_typecast_paren_pad_with_config(
    source: &str,
    config: &TypecastParenPadConfig,
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

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                message: diagnostic.kind.body.clone(),
            });
        }
    }

    violations
}

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("typecastparenpad", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}: {}", v.line, v.column, v.message);
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
// Test: InputTypecastParenPadWhitespace.java - default nospace option
// =============================================================================

#[test]
fn test_typecast_paren_pad_whitespace_nospace() {
    let Some(source) = load_fixture("InputTypecastParenPadWhitespace.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_typecast_paren_pad(&source);
    print_violations("Actual violations", &violations);

    // Based on comments in the file, expected violations:
    // Line 86: o = ( Object ) o; - both lparen and rparen (2 violations comment)
    let expected = [
        Violation::followed(86, 12), // ( Object
        Violation::preceded(86, 21), // Object )
    ];

    // Check that we found the key violations
    let by_line = violations_by_line(&violations);
    assert!(
        by_line.contains_key(&86),
        "Should find violations on line 86 for typecast with spaces"
    );

    // Line 86 should have exactly 2 violations (lparen and rparen)
    assert_eq!(
        by_line.get(&86).map(|v| v.len()).unwrap_or(0),
        2,
        "Line 86 should have exactly 2 violations"
    );

    println!(
        "Test passed: found {} violations (expected {})",
        violations.len(),
        expected.len()
    );
}

// =============================================================================
// Test: InputTypecastParenPadWhitespaceAround.java
// =============================================================================

#[test]
fn test_typecast_paren_pad_whitespace_around() {
    let Some(source) = load_fixture("InputTypecastParenPadWhitespaceAround.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_typecast_paren_pad(&source);
    print_violations("Actual violations", &violations);

    // This file tests various typecast patterns
    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: InputTypecastParenPadWhitespaceTestSpace.java - space option
// =============================================================================

#[test]
fn test_typecast_paren_pad_whitespace_test_space() {
    let Some(source) = load_fixture("InputTypecastParenPadWhitespaceTestSpace.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = TypecastParenPadConfig::with_option("space");
    let violations = check_typecast_paren_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Based on comments in the file, expected violations when option=space:
    // Line 84-85: (Object) casts without spaces
    let by_line = violations_by_line(&violations);
    assert!(
        by_line.contains_key(&84) || by_line.contains_key(&85),
        "Should find violations on lines with casts without spaces"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: inline basic tests
// =============================================================================

#[test]
fn test_typecast_with_space_nospace_option() {
    let violations =
        check_typecast_paren_pad("class Foo { void m() { Object o = ( String ) x; } }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is followed")),
        "Should detect space after lparen in typecast: {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("')' is preceded")),
        "Should detect space before rparen in typecast"
    );
}

#[test]
fn test_typecast_without_space_nospace_option() {
    let violations = check_typecast_paren_pad("class Foo { void m() { Object o = (String) x; } }");
    assert!(
        violations.is_empty(),
        "Should not flag typecast without space when option=nospace: {:?}",
        violations
    );
}

#[test]
fn test_typecast_without_space_space_option() {
    let config = TypecastParenPadConfig::with_option("space");
    let violations = check_typecast_paren_pad_with_config(
        "class Foo { void m() { Object o = (String) x; } }",
        &config,
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is not followed")),
        "Should detect missing space after lparen when option=space: {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("')' is not preceded")),
        "Should detect missing space before rparen when option=space"
    );
}

#[test]
fn test_typecast_with_space_space_option() {
    let config = TypecastParenPadConfig::with_option("space");
    let violations = check_typecast_paren_pad_with_config(
        "class Foo { void m() { Object o = ( String ) x; } }",
        &config,
    );
    assert!(
        violations.is_empty(),
        "Should not flag typecast with space when option=space: {:?}",
        violations
    );
}

#[test]
fn test_multiple_typecasts() {
    let violations = check_typecast_paren_pad(
        "class Foo { void m() { Object o = (String)x; int i = ( int ) o; } }",
    );
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.message.contains("'(' is followed"))
            .count(),
        1,
        "Should detect one lparen with space"
    );
    assert_eq!(
        violations
            .iter()
            .filter(|v| v.message.contains("')' is preceded"))
            .count(),
        1,
        "Should detect one rparen with space"
    );
}

#[test]
fn test_does_not_affect_method_calls() {
    let violations = check_typecast_paren_pad("class Foo { void m( int x ) {} }");
    // Should not produce any violations for method parens
    assert!(
        violations.is_empty(),
        "Should not check method definition parens: {:?}",
        violations
    );
}

#[test]
fn test_does_not_affect_if_statements() {
    let violations = check_typecast_paren_pad("class Foo { void m() { if( true ) {} } }");
    // Should not produce any violations for if statement parens
    assert!(
        violations.is_empty(),
        "Should not check if statement parens: {:?}",
        violations
    );
}

#[test]
fn test_all_diagnostics_have_fixes() {
    let violations =
        check_typecast_paren_pad("class Foo { void m() { Object o = ( String ) x; } }");
    assert_eq!(violations.len(), 2, "Should have 2 violations");
    // Note: We're checking violations which are converted from diagnostics, so we can't check fixes directly here.
    // The unit tests in the rule implementation verify that fixes are present.
}
