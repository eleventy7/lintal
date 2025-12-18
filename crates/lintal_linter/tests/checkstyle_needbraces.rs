//! Checkstyle compatibility tests for NeedBraces rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the NeedBraces check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::NeedBraces;
use lintal_linter::{CheckContext, FromConfig, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    construct: String,
}

impl Violation {
    fn new(line: usize, column: usize, construct: &str) -> Self {
        Self {
            line,
            column,
            construct: construct.to_string(),
        }
    }
}

/// Run NeedBraces rule with custom config on source and collect violations.
fn check_need_braces(
    source: &str,
    allow_single_line_statement: bool,
    allow_empty_loop_body: bool,
) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    properties.insert(
        "allowSingleLineStatement",
        if allow_single_line_statement {
            "true"
        } else {
            "false"
        },
    );
    properties.insert(
        "allowEmptyLoopBody",
        if allow_empty_loop_body {
            "true"
        } else {
            "false"
        },
    );

    let rule = NeedBraces::from_config(&properties);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            let message = diagnostic.kind.body.clone();

            // Parse construct from message: "'X' construct must use '{}'s"
            let construct = if let Some(start) = message.find('\'') {
                if let Some(end) = message[start + 1..].find('\'') {
                    message[start + 1..start + 1 + end].to_string()
                } else {
                    "unknown".to_string()
                }
            } else {
                "unknown".to_string()
            };

            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                construct,
            });
        }
    }

    violations.sort_by_key(|v| (v.line, v.column));
    violations
}

/// Load a checkstyle test input file for NeedBraces.
/// Returns None if the checkstyle repo is not available.
fn load_needbraces_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let fixture_path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/blocks/needbraces")
        .join(file_name);

    std::fs::read_to_string(fixture_path).ok()
}

#[test]
fn test_it() {
    let Some(source) = load_needbraces_fixture("InputNeedBracesTestIt.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_need_braces(&source, false, false);

    let expected = vec![
        Violation::new(30, 9, "do"),
        Violation::new(42, 9, "while"),
        Violation::new(43, 9, "while"),
        Violation::new(45, 9, "while"),
        Violation::new(46, 13, "if"),
        Violation::new(59, 9, "for"),
        Violation::new(60, 9, "for"),
        Violation::new(62, 9, "for"),
        Violation::new(64, 13, "if"),
        Violation::new(83, 9, "if"),
        Violation::new(84, 9, "if"),
        Violation::new(86, 9, "if"),
        Violation::new(88, 9, "else"),
        Violation::new(90, 9, "if"),
        Violation::new(98, 9, "else"),
        Violation::new(100, 9, "if"),
        Violation::new(101, 13, "if"),
        Violation::new(104, 9, "if"),
        Violation::new(105, 13, "while"),
        Violation::new(106, 9, "if"),
        Violation::new(107, 13, "do"),
        Violation::new(108, 9, "if"),
        Violation::new(109, 13, "for"),
    ];

    assert_eq!(violations, expected);
}

#[test]
fn test_allow_empty_loop_body_true() {
    let Some(source) = load_needbraces_fixture("InputNeedBracesLoopBodyTrue.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_need_braces(&source, false, true);

    let expected = vec![Violation::new(106, 9, "if")];

    assert_eq!(violations, expected);
}

#[test]
fn test_single_line_statements() {
    let Some(source) = load_needbraces_fixture("InputNeedBracesSingleLineStatements.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_need_braces(&source, true, false);

    let expected = vec![
        Violation::new(32, 9, "if"),
        Violation::new(39, 43, "if"),
        Violation::new(48, 9, "if"),
        Violation::new(56, 9, "while"),
        Violation::new(63, 9, "do"),
        Violation::new(66, 9, "for"),
        Violation::new(72, 9, "for"),
        Violation::new(101, 9, "if"),
        Violation::new(105, 11, "else"),
        Violation::new(118, 47, "if"),
        Violation::new(125, 9, "for"),
    ];

    assert_eq!(violations, expected);
}
