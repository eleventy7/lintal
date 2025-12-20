//! RedundantImport checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::RedundantImport;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

fn check_redundant_import(source: &str) -> Vec<(usize, String)> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = RedundantImport;
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push((loc.line.get(), diagnostic.kind.body.clone()));
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::imports_test_input("redundantimport", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_redundant_import_with_checker() {
    let Some(source) = load_fixture("InputRedundantImportWithChecker.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_import(&source);

    // Expected from checkstyle test file comments:
    // Line 9: same package (wildcard)
    // Line 10: same package (explicit)
    // Line 12: java.lang.*
    // Line 13: java.lang.String
    // Line 16: duplicate of line 15
    // Line 28: duplicate static import of line 27

    let expected_lines = vec![9, 10, 12, 13, 16, 28];

    println!("Found violations:");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    for expected_line in &expected_lines {
        assert!(
            violations.iter().any(|(line, _)| line == expected_line),
            "Expected violation on line {}",
            expected_line
        );
    }
}

#[test]
fn test_no_false_positives() {
    let Some(source) = load_fixture("InputRedundantImportWithoutWarnings.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_import(&source);

    assert!(
        violations.is_empty(),
        "Expected no violations, got: {:?}",
        violations
    );
}
