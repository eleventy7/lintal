//! Checkstyle compatibility tests for EmptyBlock rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the EmptyBlock check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::EmptyBlock;
use lintal_linter::{CheckContext, FromConfig, Rule};
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
    fn new(line: usize, column: usize, message: &str) -> Self {
        Self {
            line,
            column,
            message: message.to_string(),
        }
    }
}

/// Run EmptyBlock rule with custom config on source and collect violations.
fn check_empty_block(source: &str, option: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    properties.insert("option", option);

    let rule = EmptyBlock::from_config(&properties);
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

    violations.sort_by_key(|v| (v.line, v.column));
    violations
}

/// Load a checkstyle test input file for EmptyBlock.
/// Returns None if the checkstyle repo is not available.
fn load_emptyblock_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let fixture_path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/blocks/emptyblock")
        .join(file_name);

    std::fs::read_to_string(fixture_path).ok()
}

#[test]
fn test_empty_block_default() {
    let Some(source) = load_emptyblock_fixture("InputEmptyBlockSemantic.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_empty_block(&source, "statement");

    let expected = vec![
        Violation::new(38, 13, "Must have at least one statement."),
        Violation::new(40, 17, "Must have at least one statement."),
        Violation::new(42, 13, "Must have at least one statement."),
        Violation::new(45, 17, "Must have at least one statement."),
        Violation::new(68, 5, "Must have at least one statement."),
        Violation::new(76, 29, "Must have at least one statement."),
        Violation::new(78, 41, "Must have at least one statement."),
        Violation::new(89, 12, "Must have at least one statement."),
    ];

    assert_eq!(violations, expected);
}

#[test]
fn test_empty_block_text() {
    let Some(source) = load_emptyblock_fixture("InputEmptyBlockSemanticText.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_empty_block(&source, "text");

    let expected = vec![
        Violation::new(38, 13, "Empty try block."),
        Violation::new(40, 17, "Empty finally block."),
        Violation::new(68, 5, "Empty INSTANCE_INIT block."),
        Violation::new(76, 29, "Empty synchronized block."),
        Violation::new(88, 12, "Empty STATIC_INIT block."),
    ];

    assert_eq!(violations, expected);
}

#[test]
fn test_empty_block_statement() {
    let Some(source) = load_emptyblock_fixture("InputEmptyBlockSemanticStatement.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_empty_block(&source, "statement");

    let expected = vec![
        Violation::new(38, 13, "Must have at least one statement."),
        Violation::new(40, 17, "Must have at least one statement."),
        Violation::new(42, 13, "Must have at least one statement."),
        Violation::new(45, 17, "Must have at least one statement."),
        Violation::new(68, 5, "Must have at least one statement."),
        Violation::new(76, 29, "Must have at least one statement."),
        Violation::new(78, 41, "Must have at least one statement."),
        Violation::new(89, 12, "Must have at least one statement."),
    ];

    assert_eq!(violations, expected);
}
