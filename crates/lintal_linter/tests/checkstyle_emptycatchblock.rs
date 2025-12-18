//! Checkstyle compatibility tests for EmptyCatchBlock rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the EmptyCatchBlock check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::EmptyCatchBlock;
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

/// Run EmptyCatchBlock rule with custom config on source and collect violations.
fn check_empty_catch_block(
    source: &str,
    exception_variable_name: &str,
    comment_format: &str,
) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    properties.insert("exceptionVariableName", exception_variable_name);
    properties.insert("commentFormat", comment_format);

    let rule = EmptyCatchBlock::from_config(&properties);
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

/// Load a checkstyle test input file for EmptyCatchBlock.
/// Returns None if the checkstyle repo is not available.
fn load_emptycatchblock_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let fixture_path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/blocks/emptycatchblock")
        .join(file_name);

    std::fs::read_to_string(fixture_path).ok()
}

#[test]
fn test_empty_catch_block_default() {
    let Some(source) = load_emptycatchblock_fixture("InputEmptyCatchBlockDefault.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    // Default: exceptionVariableName = ^$, commentFormat = .*
    let violations = check_empty_catch_block(&source, "^$", ".*");

    let expected = vec![
        Violation::new(25, 31, "Empty catch block."),
        Violation::new(32, 83, "Empty catch block."),
    ];

    assert_eq!(violations, expected);
}

#[test]
fn test_empty_catch_block_with_user_set_values() {
    let Some(source) = load_emptycatchblock_fixture("InputEmptyCatchBlockDefault2.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    // Custom: exceptionVariableName = expected|ignore|myException, commentFormat = This is expected
    let violations =
        check_empty_catch_block(&source, "expected|ignore|myException", "This is expected");

    let expected = vec![
        Violation::new(26, 31, "Empty catch block."),
        Violation::new(54, 78, "Empty catch block."),
        Violation::new(88, 29, "Empty catch block."),
        Violation::new(177, 33, "Empty catch block."),
        Violation::new(186, 33, "Empty catch block."),
        Violation::new(205, 33, "Empty catch block."),
        Violation::new(221, 33, "Empty catch block."),
        Violation::new(230, 33, "Empty catch block."),
    ];

    assert_eq!(violations, expected);
}
