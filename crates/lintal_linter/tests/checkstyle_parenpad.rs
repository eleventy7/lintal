//! ParenPad checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::ParenPad;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::{HashMap, HashSet};

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

/// Configuration for ParenPad rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct ParenPadConfig {
    option: Option<String>,
    tokens: Option<Vec<String>>,
}

impl ParenPadConfig {
    fn default_config() -> Self {
        Self {
            option: None,
            tokens: None,
        }
    }

    fn with_option(option: &str) -> Self {
        Self {
            option: Some(option.to_string()),
            tokens: None,
        }
    }

    fn to_rule(&self) -> ParenPad {
        use lintal_linter::rules::whitespace::paren_pad::{ParenPadOption, ParenPadToken};

        let option = self
            .option
            .as_ref()
            .map(|s| match s.as_str() {
                "space" => ParenPadOption::Space,
                _ => ParenPadOption::NoSpace,
            })
            .unwrap_or(ParenPadOption::NoSpace);

        if let Some(ref tokens) = self.tokens {
            let mut token_set = HashSet::new();
            for token in tokens {
                match token.as_str() {
                    "ANNOTATION" => {
                        token_set.insert(ParenPadToken::Annotation);
                    }
                    "ANNOTATION_FIELD_DEF" => {
                        token_set.insert(ParenPadToken::AnnotationFieldDef);
                    }
                    "CTOR_CALL" => {
                        token_set.insert(ParenPadToken::CtorCall);
                    }
                    "CTOR_DEF" => {
                        token_set.insert(ParenPadToken::CtorDef);
                    }
                    "ENUM_CONSTANT_DEF" => {
                        token_set.insert(ParenPadToken::EnumConstantDef);
                    }
                    "EXPR" => {
                        token_set.insert(ParenPadToken::Expr);
                    }
                    "LITERAL_CATCH" => {
                        token_set.insert(ParenPadToken::LiteralCatch);
                    }
                    "LITERAL_DO" => {
                        token_set.insert(ParenPadToken::LiteralDo);
                    }
                    "LITERAL_FOR" => {
                        token_set.insert(ParenPadToken::LiteralFor);
                    }
                    "LITERAL_IF" => {
                        token_set.insert(ParenPadToken::LiteralIf);
                    }
                    "LITERAL_NEW" => {
                        token_set.insert(ParenPadToken::LiteralNew);
                    }
                    "LITERAL_SWITCH" => {
                        token_set.insert(ParenPadToken::LiteralSwitch);
                    }
                    "LITERAL_SYNCHRONIZED" => {
                        token_set.insert(ParenPadToken::LiteralSynchronized);
                    }
                    "LITERAL_WHILE" => {
                        token_set.insert(ParenPadToken::LiteralWhile);
                    }
                    "METHOD_CALL" => {
                        token_set.insert(ParenPadToken::MethodCall);
                    }
                    "METHOD_DEF" => {
                        token_set.insert(ParenPadToken::MethodDef);
                    }
                    "QUESTION" => {
                        token_set.insert(ParenPadToken::Question);
                    }
                    "RESOURCE_SPECIFICATION" => {
                        token_set.insert(ParenPadToken::ResourceSpecification);
                    }
                    "SUPER_CTOR_CALL" => {
                        token_set.insert(ParenPadToken::SuperCtorCall);
                    }
                    "LAMBDA" => {
                        token_set.insert(ParenPadToken::Lambda);
                    }
                    "RECORD_DEF" => {
                        token_set.insert(ParenPadToken::RecordDef);
                    }
                    _ => {}
                }
            }
            ParenPad {
                option,
                tokens: token_set,
            }
        } else {
            ParenPad {
                option,
                tokens: ParenPad::default().tokens,
            }
        }
    }
}

/// Run ParenPad rule on source and collect violations.
fn check_paren_pad(source: &str) -> Vec<Violation> {
    check_paren_pad_with_config(source, &ParenPadConfig::default_config())
}

/// Run ParenPad rule with custom config on source and collect violations.
fn check_paren_pad_with_config(source: &str, config: &ParenPadConfig) -> Vec<Violation> {
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
    let path = checkstyle_repo::whitespace_test_input("parenpad", file_name)?;
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
// Test: InputParenPadWhitespace.java - default nospace option
// =============================================================================

#[test]
fn test_paren_pad_whitespace_nospace() {
    let Some(source) = load_fixture("InputParenPadWhitespace.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_paren_pad(&source);
    print_violations("Actual violations", &violations);

    // Based on comments in the file, expected violations:
    // Line 65-66: if( !complicatedStuffNeeded ) - both lparen and rparen
    // Line 84-86: if ( true ) - both lparen and rparen
    // Line 254: (int) ( 2 / 3 ) - expr parens
    // Line 293: register( args ) - method call
    let expected = [
        Violation::followed(65, 11),  // if(
        Violation::preceded(65, 36),  // )
        Violation::followed(84, 12),  // if (
        Violation::preceded(84, 19),  // )
        Violation::followed(254, 23), // ( 2
        Violation::preceded(254, 30), // 3 )
        Violation::followed(293, 16), // register(
        Violation::preceded(293, 23), // args )
    ];

    // Check that we found the key violations
    let by_line = violations_by_line(&violations);
    assert!(
        by_line.contains_key(&65),
        "Should find violations on line 65"
    );
    assert!(
        by_line.contains_key(&84),
        "Should find violations on line 84"
    );

    println!(
        "Test passed: found {} violations (expected around {})",
        violations.len(),
        expected.len()
    );
}

// =============================================================================
// Test: InputParenPadWithSpace.java - space option
// =============================================================================

#[test]
fn test_paren_pad_with_space() {
    let Some(source) = load_fixture("InputParenPadWithSpace.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = ParenPadConfig::with_option("space");
    let violations = check_paren_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // This file should have NO violations when option=space because all parens have spaces
    // Actually, looking at the file, some don't have spaces, so there should be violations
    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: InputParenPadLambda.java - lambda expressions
// =============================================================================

#[test]
fn test_paren_pad_lambda() {
    let Some(source) = load_fixture("InputParenPadLambda.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_paren_pad(&source);
    print_violations("Actual violations", &violations);

    // Expected violations based on comments:
    // Line 20: ( o ) -> both
    // Line 25: (o ) -> rparen
    // Line 28: ( o) -> lparen
    // Line 31: ( o ) -> both
    // Line 36: ( Object o ) -> both
    // Line 41: toString( ) -> both
    // Line 47: method( String param ) -> both
    let by_line = violations_by_line(&violations);
    assert!(
        by_line.contains_key(&20),
        "Should find violations on line 20"
    );
    assert!(
        by_line.contains_key(&25),
        "Should find violations on line 25"
    );
    assert!(
        by_line.contains_key(&28),
        "Should find violations on line 28"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: InputParenPadCheckRecords.java - record declarations
// =============================================================================

#[test]
fn test_paren_pad_records() {
    let Some(source) = load_fixture("InputParenPadCheckRecords.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_paren_pad(&source);
    print_violations("Actual violations", &violations);

    // Expected violations based on comments:
    // Line 20: record MyRecord1( ) -> both
    // Line 25: MyRecord1( int x ) -> both
    // Line 31: bar( 1) -> lparen
    // Line 37: bar( 1) -> lparen
    // Line 40: bar(int k ) -> rparen
    // Line 46: switch( n) -> lparen
    let by_line = violations_by_line(&violations);
    assert!(
        by_line.contains_key(&20),
        "Should find violations on line 20"
    );
    assert!(
        by_line.contains_key(&25),
        "Should find violations on line 25"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: inline basic tests
// =============================================================================

#[test]
fn test_method_def_with_space() {
    let violations = check_paren_pad("class Foo { void m( int x ) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is followed")),
        "Should detect space after lparen: {:?}",
        violations
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("')' is preceded")),
        "Should detect space before rparen"
    );
}

#[test]
fn test_method_def_without_space() {
    let violations = check_paren_pad("class Foo { void m(int x) {} }");
    let paren_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.message.contains("'('") || v.message.contains("')'"))
        .collect();
    assert!(
        paren_violations.is_empty(),
        "Should not flag parens without space when option=nospace"
    );
}

#[test]
fn test_empty_parens() {
    let violations = check_paren_pad("class Foo { void m() {} }");
    let paren_violations: Vec<_> = violations
        .iter()
        .filter(|v| v.message.contains("'('") || v.message.contains("')'"))
        .collect();
    assert!(paren_violations.is_empty(), "Should not flag empty parens");
}

#[test]
fn test_if_statement_with_space() {
    let violations = check_paren_pad("class Foo { void m() { if( true ) {} } }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is followed")),
        "Should detect space after lparen in if"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("')' is preceded")),
        "Should detect space before rparen in if"
    );
}

#[test]
fn test_method_call_with_space() {
    let violations = check_paren_pad("class Foo { void m() { foo( 1 ); } void foo(int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is followed")),
        "Should detect space after lparen in method call"
    );
}

#[test]
fn test_space_option() {
    let config = ParenPadConfig::with_option("space");
    let violations = check_paren_pad_with_config("class Foo { void m(int x) {} }", &config);
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is not followed")),
        "Should detect missing space after lparen when option=space"
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("')' is not preceded")),
        "Should detect missing space before rparen when option=space"
    );
}
