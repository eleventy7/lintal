//! Checkstyle compatibility tests for FinalParameters rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the FinalParameters check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::FinalParameters;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
}

impl Violation {
    fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Run FinalParameters rule on source and collect violations.
fn check_final_parameters(source: &str, properties: HashMap<&str, &str>) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = FinalParameters::from_config(&properties);
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
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
/// Returns None if the checkstyle repo is not available.
fn load_finalparameters_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/finalparameters")
        .join(file_name);
    std::fs::read_to_string(&path).ok()
}

/// Helper to verify violations match expected.
fn verify_violations(violations: &[Violation], expected: &[Violation]) {
    let mut missing = vec![];
    let mut unexpected = vec![];

    for exp in expected {
        let matched = violations
            .iter()
            .any(|v| v.line == exp.line && v.column == exp.column);

        if !matched {
            missing.push(exp.clone());
        }
    }

    for actual in violations {
        let matched = expected
            .iter()
            .any(|v| v.line == actual.line && v.column == actual.column);

        if !matched {
            unexpected.push(actual.clone());
        }
    }

    if !missing.is_empty() || !unexpected.is_empty() {
        println!("\n=== Violations Report ===");
        if !missing.is_empty() {
            println!("\nMissing violations:");
            for v in &missing {
                println!("  {}:{}", v.line, v.column);
            }
        }
        if !unexpected.is_empty() {
            println!("\nUnexpected violations:");
            for v in &unexpected {
                println!("  {}:{}", v.line, v.column);
            }
        }
        panic!("Violation mismatch detected");
    }
}

// =============================================================================
// Test: testDefaultTokens
// File: InputFinalParameters.java
// Config: tokens = (default)METHOD_DEF, CTOR_DEF
// Expected violations from checkstyle test:
//   28:26 - Parameter s should be final
//   43:26 - Parameter i should be final
//   48:26 - Parameter s should be final
//   58:17 - Parameter s should be final
//   74:17 - Parameter s should be final
//   80:17 - Parameter s should be final
//   95:45 - Parameter e should be final
//   98:36 - Parameter e should be final
//   115:18 - Parameter aParam should be final
//   118:18 - Parameter args should be final
//   121:18 - Parameter args should be final
// =============================================================================

#[test]
fn test_default_tokens() {
    let Some(source) = load_finalparameters_fixture("InputFinalParameters.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(28, 26),
        Violation::new(43, 26),
        Violation::new(48, 26),
        Violation::new(58, 17),
        Violation::new(74, 17),
        Violation::new(80, 17),
        Violation::new(95, 45),
        Violation::new(98, 36),
        Violation::new(115, 18),
        Violation::new(118, 18),
        Violation::new(121, 18),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testCtorToken
// File: InputFinalParameters2.java
// Config: tokens = CTOR_DEF
// Expected violations from checkstyle test:
//   29:27 - Parameter s should be final
//   44:27 - Parameter i should be final
//   49:27 - Parameter s should be final
// =============================================================================

#[test]
fn test_ctor_token() {
    let Some(source) = load_finalparameters_fixture("InputFinalParameters2.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("tokens", "CTOR_DEF");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(29, 27),
        Violation::new(44, 27),
        Violation::new(49, 27),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testMethodToken
// File: InputFinalParameters3.java
// Config: tokens = METHOD_DEF
// Expected violations from checkstyle test:
//   59:17 - Parameter s should be final
//   75:17 - Parameter s should be final
//   81:17 - Parameter s should be final
//   96:45 - Parameter e should be final
//   99:36 - Parameter e should be final
//   116:18 - Parameter aParam should be final
//   119:18 - Parameter args should be final
//   122:18 - Parameter args should be final
// =============================================================================

#[test]
fn test_method_token() {
    let Some(source) = load_finalparameters_fixture("InputFinalParameters3.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("tokens", "METHOD_DEF");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(59, 17),
        Violation::new(75, 17),
        Violation::new(81, 17),
        Violation::new(96, 45),
        Violation::new(99, 36),
        Violation::new(116, 18),
        Violation::new(119, 18),
        Violation::new(122, 18),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testCatchToken
// File: InputFinalParameters4.java
// Config: tokens = LITERAL_CATCH
// Expected violations from checkstyle test:
//   131:16 - Parameter npe should be final
//   137:16 - Parameter e should be final
//   140:16 - Parameter e should be final
// =============================================================================

#[test]
fn test_catch_token() {
    let Some(source) = load_finalparameters_fixture("InputFinalParameters4.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("tokens", "LITERAL_CATCH");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(131, 16),
        Violation::new(137, 16),
        Violation::new(140, 16),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testForEachClauseToken
// File: InputFinalParameters5.java
// Config: tokens = FOR_EACH_CLAUSE
// Expected violations from checkstyle test:
//   158:13 - Parameter s should be final
//   166:13 - Parameter s should be final
// =============================================================================

#[test]
fn test_for_each_clause_token() {
    let Some(source) = load_finalparameters_fixture("InputFinalParameters5.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("tokens", "FOR_EACH_CLAUSE");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![Violation::new(158, 13), Violation::new(166, 13)];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testIgnorePrimitiveTypesParameters
// File: InputFinalParametersPrimitiveTypes.java
// Config: ignorePrimitiveTypes = true
// Expected violations from checkstyle test:
//   15:22 - Parameter k should be final
//   16:15 - Parameter s should be final
//   16:25 - Parameter o should be final
//   20:15 - Parameter array should be final
//   21:31 - Parameter s should be final
//   22:22 - Parameter l should be final
//   22:32 - Parameter s should be final
// =============================================================================

#[test]
fn test_ignore_primitive_types_parameters() {
    let Some(source) = load_finalparameters_fixture("InputFinalParametersPrimitiveTypes.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("ignorePrimitiveTypes", "true");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(15, 22),
        Violation::new(16, 15),
        Violation::new(16, 25),
        Violation::new(20, 15),
        Violation::new(21, 31),
        Violation::new(22, 22),
        Violation::new(22, 32),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testPrimitiveTypesParameters
// File: InputFinalParametersPrimitiveTypes2.java
// Config: ignorePrimitiveTypes = (default)false
// Expected violations from checkstyle test:
//   14:14 - Parameter i should be final
//   15:15 - Parameter i should be final
//   15:22 - Parameter k should be final
//   15:32 - Parameter s should be final
//   20:15 - Parameter s should be final
//   20:25 - Parameter o should be final
//   20:35 - Parameter l should be final
//   25:15 - Parameter array should be final
//   26:15 - Parameter i should be final
//   26:22 - Parameter x should be final
//   26:31 - Parameter s should be final
//   31:15 - Parameter x should be final
//   31:22 - Parameter l should be final
//   31:32 - Parameter s should be final
// =============================================================================

#[test]
fn test_primitive_types_parameters() {
    let Some(source) = load_finalparameters_fixture("InputFinalParametersPrimitiveTypes2.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(14, 14),
        Violation::new(15, 15),
        Violation::new(15, 22),
        Violation::new(15, 32),
        Violation::new(20, 15),
        Violation::new(20, 25),
        Violation::new(20, 35),
        Violation::new(25, 15),
        Violation::new(26, 15),
        Violation::new(26, 22),
        Violation::new(26, 31),
        Violation::new(31, 15),
        Violation::new(31, 22),
        Violation::new(31, 32),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testReceiverParameters
// File: InputFinalParametersReceiver.java
// Config: default
// Expected violations: none (receiver parameters should be ignored)
// =============================================================================

#[test]
fn test_receiver_parameters() {
    let Some(source) = load_finalparameters_fixture("InputFinalParametersReceiver.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let properties = HashMap::new();
    let violations = check_final_parameters(&source, properties);

    let expected = vec![];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testUnnamedParametersPropertyTrue
// File: InputFinalParametersUnnamedPropertyTrue.java
// Config: ignoreUnnamedParameters = true (default)
// Expected violations: only non-underscore parameters
//   25:18 - Parameter __ should be final
//   30:18 - Parameter _e should be final
//   35:18 - Parameter e_ should be final
//   46:14 - Parameter __ should be final
//   49:14 - Parameter _i should be final
//   52:14 - Parameter i_ should be final
// =============================================================================

#[test]
fn test_unnamed_parameters_property_true() {
    let Some(source) =
        load_finalparameters_fixture("InputFinalParametersUnnamedPropertyTrue.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("ignoreUnnamedParameters", "true");
    properties.insert("tokens", "METHOD_DEF,CTOR_DEF,LITERAL_CATCH,FOR_EACH_CLAUSE");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(25, 18),
        Violation::new(30, 18),
        Violation::new(35, 18),
        Violation::new(46, 14),
        Violation::new(49, 14),
        Violation::new(52, 14),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testUnnamedParametersPropertyFalse
// File: InputFinalParametersUnnamedPropertyFalse.java
// Config: ignoreUnnamedParameters = false
// Expected violations: all parameters including underscore
//   20:18 - Parameter _ should be final
//   25:18 - Parameter __ should be final
//   30:18 - Parameter _e should be final
//   35:18 - Parameter e_ should be final
//   43:14 - Parameter _ should be final
//   46:14 - Parameter __ should be final
//   49:14 - Parameter _i should be final
//   52:14 - Parameter i_ should be final
// =============================================================================

#[test]
fn test_unnamed_parameters_property_false() {
    let Some(source) =
        load_finalparameters_fixture("InputFinalParametersUnnamedPropertyFalse.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("ignoreUnnamedParameters", "false");
    properties.insert("tokens", "METHOD_DEF,CTOR_DEF,LITERAL_CATCH,FOR_EACH_CLAUSE");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(20, 18),
        Violation::new(25, 18),
        Violation::new(30, 18),
        Violation::new(35, 18),
        Violation::new(43, 14),
        Violation::new(46, 14),
        Violation::new(49, 14),
        Violation::new(52, 14),
    ];

    verify_violations(&violations, &expected);
}

// =============================================================================
// Test: testMethodTokenInInterface
// File: InputFinalParametersInterfaceMethod.java
// Config: tokens = METHOD_DEF
// Expected violations: Only default/static interface methods (not abstract)
//   16:26 - Parameter param1 should be final
//   22:27 - Parameter param1 should be final
//   28:27 - Parameter param1 should be final
// =============================================================================

#[test]
fn test_method_token_in_interface() {
    let Some(source) = load_finalparameters_fixture("InputFinalParametersInterfaceMethod.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let mut properties = HashMap::new();
    properties.insert("tokens", "METHOD_DEF");
    let violations = check_final_parameters(&source, properties);

    let expected = vec![
        Violation::new(16, 26),
        Violation::new(22, 27),
        Violation::new(28, 27),
    ];

    verify_violations(&violations, &expected);
}
