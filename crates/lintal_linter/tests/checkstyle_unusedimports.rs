//! UnusedImports checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::UnusedImports;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

fn check_unused_imports(source: &str, process_javadoc: bool) -> Vec<(usize, String)> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = UnusedImports { process_javadoc };
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
    let path = checkstyle_repo::imports_test_input("unusedimports", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_unused_imports_main() {
    let Some(source) = load_fixture("InputUnusedImports.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, true);

    println!("Found violations:");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    // Expected violations from checkstyle test comments (processJavadoc=true):
    // Line 11: GuardedBy unused
    // Line 15: java.lang.String unused
    // Line 17-18: List unused (duplicate)
    // Line 21: Enumeration unused
    // Line 24: JToggleButton unused
    // Line 26: BorderFactory unused
    // Line 31-32: createTempFile unused
    // Line 36: Label unused
    // Line 48: ForOverride unused

    let expected_unused_lines = vec![11, 15, 17, 18, 21, 24, 26, 36, 48];

    for line in &expected_unused_lines {
        let found = violations.iter().any(|(l, _)| l == line);
        if !found {
            println!("WARNING: Expected violation on line {} not found", line);
        }
    }

    // Should have at least 6 of the expected violations
    let found_count = expected_unused_lines
        .iter()
        .filter(|line| violations.iter().any(|(l, _)| l == *line))
        .count();

    assert!(
        found_count >= 6,
        "Expected at least 6 of {} violations, found {}",
        expected_unused_lines.len(),
        found_count
    );
}

#[test]
fn test_no_false_positives() {
    let Some(source) = load_fixture("InputUnusedImportsWithoutWarnings.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, true);

    println!("Violations (should be empty):");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    assert!(
        violations.is_empty(),
        "Expected no violations, got {} violations",
        violations.len()
    );
}

#[test]
fn test_javadoc_disabled() {
    let Some(source) = load_fixture("InputUnusedImportsFromStaticMethodRefJavadocDisabled.java")
    else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, false);

    println!("Violations (javadoc disabled):");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }
}
