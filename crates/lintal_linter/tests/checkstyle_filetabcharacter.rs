//! FileTabCharacter checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::FileTabCharacter;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
}

impl Violation {
    #[allow(dead_code)]
    fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Configuration for FileTabCharacter rule.
#[derive(Debug, Clone)]
struct FileTabCharacterConfig {
    each_line: bool,
    tab_width: usize,
}

impl FileTabCharacterConfig {
    fn default_config() -> Self {
        Self {
            each_line: false,
            tab_width: 8,
        }
    }

    fn with_each_line(each_line: bool) -> Self {
        Self {
            each_line,
            tab_width: 8,
        }
    }

    fn to_rule(&self) -> FileTabCharacter {
        FileTabCharacter {
            each_line: self.each_line,
            tab_width: self.tab_width,
        }
    }
}

/// Run FileTabCharacter rule on source and collect violations.
fn check_file_tab_character(source: &str) -> Vec<Violation> {
    check_file_tab_character_with_config(source, &FileTabCharacterConfig::default_config())
}

/// Run FileTabCharacter rule with custom config on source and collect violations.
fn check_file_tab_character_with_config(
    source: &str,
    config: &FileTabCharacterConfig,
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
            });
        }
    }

    violations
}

/// Load a checkstyle test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("filetabcharacter", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Extract line and column info from violation comments in source.
/// Format: "// violation 'message'" or just "// violation"
#[allow(dead_code)]
fn extract_violation_locations(source: &str) -> Vec<Violation> {
    let mut violations = vec![];
    for (line_no, line) in source.lines().enumerate() {
        if line.contains("// violation") {
            // Find the position of the tab on this line
            if let Some(tab_pos) = line.find('\t') {
                violations.push(Violation::new(line_no + 1, tab_pos + 1)); // 1-indexed
            }
        }
    }
    violations
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}", v.line, v.column);
    }
}

// =============================================================================
// Test: default configuration (eachLine=false)
// File: InputFileTabCharacterSimple.java
// Expected: only first tab violation reported
// =============================================================================

#[test]
fn test_file_tab_character_default() {
    let Some(source) = load_fixture("InputFileTabCharacterSimple.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_file_tab_character(&source);
    print_violations("Actual violations", &violations);

    // With default config (eachLine=false), we should only get ONE violation
    // for the first tab in the file
    assert_eq!(
        violations.len(),
        1,
        "Should have exactly 1 violation with default config, found {}",
        violations.len()
    );

    // The first tab should be on line 22 based on the fixture
    assert_eq!(
        violations[0].line, 22,
        "First tab should be on line 22, found line {}",
        violations[0].line
    );

    println!("Test passed: default config reports only first tab");
}

// =============================================================================
// Test: eachLine=true configuration
// File: InputFileTabCharacterSimple.java
// Expected: all tab violations reported
// =============================================================================

#[test]
fn test_file_tab_character_each_line() {
    let Some(source) = load_fixture("InputFileTabCharacterSimple.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = FileTabCharacterConfig::with_each_line(true);
    let violations = check_file_tab_character_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // With eachLine=true, we should get multiple violations
    // The exact count depends on the test file
    assert!(
        violations.len() > 1,
        "Should have multiple violations with eachLine=true, found {}",
        violations.len()
    );

    println!(
        "Test passed: eachLine=true reports {} tabs",
        violations.len()
    );
}

// =============================================================================
// Test: file with no tabs
// Expected: no violations
// =============================================================================

#[test]
fn test_file_tab_character_no_tabs() {
    let source = r#"
package com.example;

class Foo {
    int x = 42;

    void method() {
        System.out.println("No tabs here!");
    }
}
"#;

    let violations = check_file_tab_character(source);
    print_violations("Actual violations", &violations);

    assert!(
        violations.is_empty(),
        "Should have no violations for file without tabs"
    );

    println!("Test passed: no violations for file without tabs");
}

// =============================================================================
// Test: simple tab case
// Expected: 1 violation at column where tab appears
// =============================================================================

#[test]
fn test_simple_tab() {
    let source = "class Foo {\tint x; }"; // Tab after '{'
    let violations = check_file_tab_character(source);

    assert_eq!(
        violations.len(),
        1,
        "Should detect single tab, found {}",
        violations.len()
    );

    // Tab is at position 11 (after "class Foo {")
    assert_eq!(violations[0].line, 1, "Tab should be on line 1");
    assert_eq!(violations[0].column, 12, "Tab should be at column 12");

    println!("Test passed: simple tab detected at correct position");
}

// =============================================================================
// Test: multiple tabs on same line
// Expected: only first tab reported (default config)
// =============================================================================

#[test]
fn test_multiple_tabs_same_line() {
    let source = "class Foo {\t\tint x; }"; // Two tabs
    let violations = check_file_tab_character(source);

    assert_eq!(
        violations.len(),
        1,
        "Should report only first tab on line, found {}",
        violations.len()
    );

    println!("Test passed: only first tab on line reported");
}

// =============================================================================
// Test: multiple tabs on different lines with eachLine=true
// Expected: all tabs reported
// =============================================================================

#[test]
fn test_multiple_lines_each_line() {
    let source = "class Foo {\n\tint x;\n\tint y;\n}";
    let config = FileTabCharacterConfig::with_each_line(true);
    let violations = check_file_tab_character_with_config(source, &config);

    assert_eq!(
        violations.len(),
        2,
        "Should report tabs on both lines, found {}",
        violations.len()
    );

    assert_eq!(violations[0].line, 2, "First tab should be on line 2");
    assert_eq!(violations[1].line, 3, "Second tab should be on line 3");

    println!("Test passed: all tabs on different lines reported");
}

// =============================================================================
// Test: tab in comment
// Expected: still reported (tabs are tabs, regardless of context)
// =============================================================================

#[test]
fn test_tab_in_comment() {
    let source = "class Foo {\n\t// A comment with a tab\n}";
    let violations = check_file_tab_character(source);

    assert_eq!(violations.len(), 1, "Should detect tab even in comment");

    println!("Test passed: tab in comment is detected");
}

// =============================================================================
// Test: tab in string literal
// Expected: still reported (file-level check doesn't parse semantics)
// =============================================================================

#[test]
fn test_tab_in_string() {
    // Note: \t in raw string is two chars: \ and t, not a tab
    // Let's use an actual tab character
    let source_with_tab = "class Foo { String s = \"text\there\"; }";

    let violations = check_file_tab_character(source_with_tab);

    assert_eq!(
        violations.len(),
        1,
        "Should detect tab even in string literal"
    );

    println!("Test passed: tab in string literal is detected");
}

// =============================================================================
// Test: empty file
// Expected: no violations
// =============================================================================

#[test]
fn test_empty_file() {
    let source = "";
    let violations = check_file_tab_character(source);

    assert!(
        violations.is_empty(),
        "Empty file should have no violations"
    );

    println!("Test passed: empty file has no violations");
}

// =============================================================================
// Test: verify fix replacements
// Expected: tabs should be replaced with appropriate number of spaces
// =============================================================================

#[test]
fn test_tab_fix() {
    let source = "class\tFoo"; // Tab after "class" (column 5)

    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = FileTabCharacter::default();
    let ctx = CheckContext::new(source);

    // Get diagnostics from root node
    let mut all_diagnostics = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        all_diagnostics.extend(diagnostics);
    }

    assert_eq!(all_diagnostics.len(), 1, "Should have one diagnostic");

    let diagnostic = &all_diagnostics[0];
    assert!(diagnostic.fix.is_some(), "Diagnostic should have a fix");

    let fix = diagnostic.fix.as_ref().unwrap();
    let edits = fix.edits();
    assert_eq!(edits.len(), 1, "Fix should have exactly one edit");

    let edit = &edits[0];

    // Tab at column 5, with tab_width=8, next tab stop is 8
    // So we need 8-5=3 spaces
    assert_eq!(
        edit.content().expect("Edit should have content"),
        "   ", // 3 spaces
        "Fix should replace tab with 3 spaces"
    );

    println!("Test passed: tab fix generates correct replacement");
}
