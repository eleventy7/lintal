//! Checkstyle compatibility tests for AvoidNestedBlocks rule.
//!
//! These tests verify that lintal produces the same violations as checkstyle
//! for the AvoidNestedBlocks check.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::AvoidNestedBlocks;
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

/// Run AvoidNestedBlocks rule with custom config on source and collect violations.
fn check_avoid_nested_blocks(source: &str, allow_in_switch_case: bool) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let mut properties = HashMap::new();
    if allow_in_switch_case {
        properties.insert("allowInSwitchCase", "true");
    } else {
        properties.insert("allowInSwitchCase", "false");
    }

    let rule = AvoidNestedBlocks::from_config(&properties);
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

/// Load a checkstyle test input file for AvoidNestedBlocks.
/// Returns None if the checkstyle repo is not available.
fn load_avoidnestedblocks_fixture(file_name: &str) -> Option<String> {
    let checkstyle_root = checkstyle_repo::checkstyle_repo()?;
    let fixture_path = checkstyle_root
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/blocks/avoidnestedblocks")
        .join(file_name);

    std::fs::read_to_string(fixture_path).ok()
}

#[test]
fn test_avoid_nested_blocks_default() {
    let Some(source) = load_avoidnestedblocks_fixture("InputAvoidNestedBlocksDefault.java") else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_avoid_nested_blocks(&source, false);

    // From AvoidNestedBlocksCheckTest.java testStrictSettings:
    // "25:9: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    // "47:17: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    // "53:17: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    // "61:17: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    let expected = vec![
        Violation::new(25, 9, "Avoid nested blocks."),
        Violation::new(47, 17, "Avoid nested blocks."),
        Violation::new(53, 17, "Avoid nested blocks."),
        Violation::new(61, 17, "Avoid nested blocks."),
    ];

    assert_eq!(violations, expected);
}

#[test]
fn test_avoid_nested_blocks_allow_in_switch_case() {
    let Some(source) =
        load_avoidnestedblocks_fixture("InputAvoidNestedBlocksAllowInSwitchCase.java")
    else {
        eprintln!("Skipping test - checkstyle repo not available");
        return;
    };

    let violations = check_avoid_nested_blocks(&source, true);

    // From AvoidNestedBlocksCheckTest.java testAllowSwitchInCase:
    // "21:9: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    // "43:17: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    // "57:17: " + getCheckMessage(MSG_KEY_BLOCK_NESTED),
    let expected = vec![
        Violation::new(21, 9, "Avoid nested blocks."),
        Violation::new(43, 17, "Avoid nested blocks."),
        Violation::new(57, 17, "Avoid nested blocks."),
    ];

    assert_eq!(violations, expected);
}
