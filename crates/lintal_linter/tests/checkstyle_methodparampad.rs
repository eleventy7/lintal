//! MethodParamPad checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::MethodParamPad;
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
    #[allow(dead_code)]
    fn ws_preceded(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message: "'(' is preceded by whitespace".to_string(),
        }
    }

    #[allow(dead_code)]
    fn ws_not_preceded(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message: "'(' is not preceded by whitespace".to_string(),
        }
    }

    #[allow(dead_code)]
    fn line_previous(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            message: "'(' should be on the previous line".to_string(),
        }
    }
}

/// Configuration for MethodParamPad rule matching checkstyle options.
#[derive(Debug, Clone, Default)]
struct MethodParamPadConfig {
    option: Option<String>,
    allow_line_breaks: Option<bool>,
    tokens: Option<Vec<String>>,
}

impl MethodParamPadConfig {
    fn default_config() -> Self {
        Self {
            option: None,
            allow_line_breaks: None,
            tokens: None,
        }
    }

    fn with_option(option: &str) -> Self {
        Self {
            option: Some(option.to_string()),
            allow_line_breaks: None,
            tokens: None,
        }
    }

    fn with_allow_line_breaks(allow: bool) -> Self {
        Self {
            option: None,
            allow_line_breaks: Some(allow),
            tokens: None,
        }
    }

    fn to_rule(&self) -> MethodParamPad {
        use lintal_linter::rules::whitespace::method_param_pad::{
            MethodParamPadOption, MethodParamPadToken,
        };

        let option = self
            .option
            .as_ref()
            .map(|s| match s.as_str() {
                "space" => MethodParamPadOption::Space,
                _ => MethodParamPadOption::NoSpace,
            })
            .unwrap_or(MethodParamPadOption::NoSpace);

        let allow_line_breaks = self.allow_line_breaks.unwrap_or(false);

        if let Some(ref tokens) = self.tokens {
            let mut token_set = HashSet::new();
            for token in tokens {
                match token.as_str() {
                    "CTOR_DEF" => {
                        token_set.insert(MethodParamPadToken::CtorDef);
                    }
                    "LITERAL_NEW" => {
                        token_set.insert(MethodParamPadToken::LiteralNew);
                    }
                    "METHOD_CALL" => {
                        token_set.insert(MethodParamPadToken::MethodCall);
                    }
                    "METHOD_DEF" => {
                        token_set.insert(MethodParamPadToken::MethodDef);
                    }
                    "SUPER_CTOR_CALL" => {
                        token_set.insert(MethodParamPadToken::SuperCtorCall);
                    }
                    "ENUM_CONSTANT_DEF" => {
                        token_set.insert(MethodParamPadToken::EnumConstantDef);
                    }
                    "RECORD_DEF" => {
                        token_set.insert(MethodParamPadToken::RecordDef);
                    }
                    _ => {}
                }
            }
            MethodParamPad {
                option,
                allow_line_breaks,
                tokens: token_set,
            }
        } else {
            MethodParamPad {
                option,
                allow_line_breaks,
                tokens: MethodParamPad::default().tokens,
            }
        }
    }
}

/// Run MethodParamPad rule on source and collect violations.
fn check_method_param_pad(source: &str) -> Vec<Violation> {
    check_method_param_pad_with_config(source, &MethodParamPadConfig::default_config())
}

/// Run MethodParamPad rule with custom config on source and collect violations.
fn check_method_param_pad_with_config(
    source: &str,
    config: &MethodParamPadConfig,
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
    let path = checkstyle_repo::whitespace_test_input("methodparampad", file_name)?;
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
// Test: InputMethodParamPad.java - default nospace option, no line breaks
// =============================================================================

#[test]
fn test_method_param_pad_default() {
    let Some(source) = load_fixture("InputMethodParamPad.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_method_param_pad(&source);
    print_violations("Actual violations", &violations);

    // Based on comments in the file, expected violations:
    // Line 21: InputMethodParamPad (int aParam) - ws preceded
    // Line 23: super (); - ws preceded
    // Line 27: (double aParam) - line previous
    // Line 30: (); - line previous
    // Line 37: method (int aParam) - ws preceded
    // Line 42: (double aParam) - line previous
    // Line 46: new InputMethodParamPad (); - ws preceded
    // Line 48: (); - line previous
    // Line 52: method (); - ws preceded
    // Line 54: (); - line previous
    // Line 60: method (); - ws preceded
    // Line 62: (); - line previous
    // Line 66: method (); - ws preceded
    // Line 68: (); - line previous
    // Line 71: parseInt ("0"); - ws preceded
    // Line 73: ("0"); - line previous
    // Line 84: FIRST () - ws preceded
    // Line 89: () - line previous

    let by_line = violations_by_line(&violations);

    // Check some key violations
    assert!(
        by_line.contains_key(&21),
        "Should find violation on line 21 (constructor with space)"
    );
    assert!(
        by_line.contains_key(&23),
        "Should find violation on line 23 (super with space)"
    );
    assert!(
        by_line.contains_key(&27),
        "Should find violation on line 27 (line break)"
    );
    assert!(
        by_line.contains_key(&37),
        "Should find violation on line 37 (method with space)"
    );
    assert!(
        by_line.contains_key(&84),
        "Should find violation on line 84 (enum constant with space)"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: InputMethodParamPad2.java - allowLineBreaks = true
// =============================================================================

#[test]
fn test_method_param_pad_allow_line_breaks() {
    let Some(source) = load_fixture("InputMethodParamPad2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = MethodParamPadConfig::with_allow_line_breaks(true);
    let violations = check_method_param_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // With allowLineBreaks=true, we should NOT see "line previous" violations
    // But we should still see "ws preceded" violations
    let by_line = violations_by_line(&violations);

    // Check that we have ws preceded violations
    assert!(
        by_line.contains_key(&21),
        "Should find violation on line 21 (constructor with space)"
    );
    assert!(
        by_line.contains_key(&23),
        "Should find violation on line 23 (super with space)"
    );

    // Check that we DON'T have line previous violations
    for v in &violations {
        assert!(
            !v.message.contains("should be on the previous line"),
            "Should not have 'line previous' violations when allowLineBreaks=true"
        );
    }

    println!(
        "Test passed: found {} violations (no line break violations)",
        violations.len()
    );
}

// =============================================================================
// Test: InputMethodParamPadRecords.java - record declarations
// =============================================================================

#[test]
fn test_method_param_pad_records() {
    let Some(source) = load_fixture("InputMethodParamPadRecords.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = MethodParamPadConfig::with_allow_line_breaks(true);
    let violations = check_method_param_pad_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Expected violations based on comments:
    // Line 19: record Mtr (String string, Record rec) - ws preceded
    // Line 20: private boolean inRecord (Object obj) - ws preceded
    // Line 31: record Mtr2 () - ws preceded
    // Line 32: Mtr2 (String s1, String s2, String s3) - ws preceded
    // Line 33: this (); - ws preceded
    // Line 37: record Mtr3 (Integer i, Node node) - ws preceded
    // Line 38: public static void main (String... args) - ws preceded
    // Line 44: record Mtr4 () - ws preceded
    // Line 45: void foo (){} - ws preceded
    // Line 51: new Mtr ("my string", new Mtr4()) - ws preceded
    // Line 57: new Mtr ("my string", new Mtr4()) - ws preceded

    let by_line = violations_by_line(&violations);

    assert!(
        by_line.contains_key(&19),
        "Should find violation on line 19 (record with space)"
    );
    assert!(
        by_line.contains_key(&20),
        "Should find violation on line 20 (method in record with space)"
    );
    assert!(
        by_line.contains_key(&31),
        "Should find violation on line 31 (record with space)"
    );

    println!("Test passed: found {} violations", violations.len());
}

// =============================================================================
// Test: inline basic tests
// =============================================================================

#[test]
fn test_method_def_with_space() {
    let violations = check_method_param_pad("class Foo { void m (int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen: {:?}",
        violations
    );
}

#[test]
fn test_method_def_without_space() {
    let violations = check_method_param_pad("class Foo { void m(int x) {} }");
    assert!(
        violations.is_empty(),
        "Should not flag lparen without space when option=nospace"
    );
}

#[test]
fn test_constructor_with_space() {
    let violations = check_method_param_pad("class Foo { Foo (int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen in constructor"
    );
}

#[test]
fn test_method_call_with_space() {
    let violations =
        check_method_param_pad("class Foo { void m() { foo (1); } void foo(int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen in method call"
    );
}

#[test]
fn test_new_with_space() {
    let violations =
        check_method_param_pad("class Foo { void m() { new Foo (1); } Foo(int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen in new expression"
    );
}

#[test]
fn test_super_call_with_space() {
    let violations = check_method_param_pad("class Foo { Foo() { super (); } }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen in super call"
    );
}

#[test]
fn test_line_break_not_allowed() {
    let violations = check_method_param_pad("class Foo { void m\n(int x) {} }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("should be on the previous line")),
        "Should detect line break before lparen when not allowed"
    );
}

#[test]
fn test_line_break_allowed() {
    let config = MethodParamPadConfig::with_allow_line_breaks(true);
    let violations =
        check_method_param_pad_with_config("class Foo { void m\n(int x) {} }", &config);
    assert!(
        violations.is_empty(),
        "Should not flag line break when allowed"
    );
}

#[test]
fn test_space_option() {
    let config = MethodParamPadConfig::with_option("space");
    let violations = check_method_param_pad_with_config("class Foo { void m(int x) {} }", &config);
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is not preceded")),
        "Should detect missing space before lparen when option=space"
    );
}

#[test]
fn test_space_option_with_space() {
    let config = MethodParamPadConfig::with_option("space");
    let violations = check_method_param_pad_with_config("class Foo { void m (int x) {} }", &config);
    assert!(
        violations.is_empty(),
        "Should not flag lparen with space when option=space"
    );
}

#[test]
fn test_enum_constant() {
    let violations = check_method_param_pad("enum E { A (), B() }");
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("'(' is preceded")),
        "Should detect space before lparen in enum constant"
    );
}
