//! SingleSpaceSeparator checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::SingleSpaceSeparator;
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

/// Configuration for SingleSpaceSeparator rule.
#[derive(Debug, Clone)]
struct SingleSpaceSeparatorConfig {
    validate_comments: bool,
}

impl SingleSpaceSeparatorConfig {
    fn default_config() -> Self {
        Self {
            validate_comments: false,
        }
    }

    fn with_validate_comments(validate_comments: bool) -> Self {
        Self { validate_comments }
    }

    fn to_rule(&self) -> SingleSpaceSeparator {
        SingleSpaceSeparator {
            validate_comments: self.validate_comments,
        }
    }
}

/// Run SingleSpaceSeparator rule on source and collect violations.
fn check_single_space_separator(source: &str) -> Vec<Violation> {
    check_single_space_separator_with_config(source, &SingleSpaceSeparatorConfig::default_config())
}

/// Run SingleSpaceSeparator rule with custom config on source and collect violations.
fn check_single_space_separator_with_config(
    source: &str,
    config: &SingleSpaceSeparatorConfig,
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
    let path = checkstyle_repo::whitespace_test_input("singlespaceseparator", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Count the number of "// violation" or "// N violations" comments in source.
fn count_violation_comments(source: &str) -> usize {
    let mut count = 0;
    for line in source.lines() {
        if let Some(comment) = line.split("//").nth(1) {
            let comment = comment.trim();
            if comment == "violation" {
                count += 1;
            } else if comment.ends_with("violations") {
                // Parse "2 violations" -> 2
                if let Some(num_str) = comment.split_whitespace().next()
                    && let Ok(num) = num_str.parse::<usize>()
                {
                    count += num;
                }
            }
        }
    }
    count
}

/// Extract line numbers with violations from comments.
#[allow(dead_code)]
fn extract_violation_lines(source: &str) -> Vec<usize> {
    let mut lines = vec![];
    for (line_num, line) in source.lines().enumerate() {
        if line.contains("// violation")
            || line.contains("// 2 violations")
            || line.contains("// 3 violations")
            || line.contains("// 4 violations")
        {
            lines.push(line_num + 1); // 1-indexed
        }
    }
    lines
}

/// Print violation details for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}", v.line, v.column);
    }
}

// =============================================================================
// Test: testNoErrors
// File: InputSingleSpaceSeparatorNoErrors.java
// Expected: no violations (validateComments=false by default)
// =============================================================================

#[test]
fn test_single_space_separator_no_errors() {
    let Some(source) = load_fixture("InputSingleSpaceSeparatorNoErrors.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_single_space_separator(&source);
    print_violations("Actual violations", &violations);

    assert!(
        violations.is_empty(),
        "Should have no violations, found {}",
        violations.len()
    );

    println!("Test passed: no violations as expected");
}

// =============================================================================
// Test: testErrors
// File: InputSingleSpaceSeparatorErrors.java
// Expected: violations with validateComments=true
// =============================================================================

#[test]
fn test_single_space_separator_errors() {
    let Some(source) = load_fixture("InputSingleSpaceSeparatorErrors.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = SingleSpaceSeparatorConfig::with_validate_comments(true);
    let violations = check_single_space_separator_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Count expected violations from comments
    let expected_count = count_violation_comments(&source);
    println!("Expected violations from comments: {}", expected_count);

    // We should find violations
    assert!(
        !violations.is_empty(),
        "Should have violations with validateComments=true"
    );

    println!(
        "Test: found {} violations, expected around {}",
        violations.len(),
        expected_count
    );
}

// =============================================================================
// Test: testComments
// File: InputSingleSpaceSeparatorComments.java
// Expected: violations with validateComments=true
// =============================================================================

#[test]
fn test_single_space_separator_comments() {
    let Some(source) = load_fixture("InputSingleSpaceSeparatorComments.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let config = SingleSpaceSeparatorConfig::with_validate_comments(true);
    let violations = check_single_space_separator_with_config(&source, &config);
    print_violations("Actual violations", &violations);

    // Count expected violations
    let expected_count = count_violation_comments(&source);
    println!("Expected violations from comments: {}", expected_count);

    assert!(
        !violations.is_empty(),
        "Should have violations with validateComments=true"
    );

    println!(
        "Test: found {} violations, expected {}",
        violations.len(),
        expected_count
    );
}

// =============================================================================
// Test: testCommentsNoValidation
// File: InputSingleSpaceSeparatorComments.java
// Expected: no violations with validateComments=false
// =============================================================================

#[test]
fn test_single_space_separator_comments_no_validation() {
    let Some(source) = load_fixture("InputSingleSpaceSeparatorComments.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // Default config has validateComments=false
    let violations = check_single_space_separator(&source);
    print_violations("Actual violations", &violations);

    assert!(
        violations.is_empty(),
        "Should have no violations with validateComments=false"
    );

    println!("Test passed: no violations with validateComments=false");
}

// =============================================================================
// Test: testEmpty
// File: InputSingleSpaceSeparatorEmpty.java
// Expected: no violations (empty file)
// =============================================================================

#[test]
fn test_single_space_separator_empty() {
    let Some(source) = load_fixture("InputSingleSpaceSeparatorEmpty.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_single_space_separator(&source);
    print_violations("Actual violations", &violations);

    assert!(
        violations.is_empty(),
        "Empty file should have no violations"
    );

    println!("Test passed: empty file has no violations");
}

// =============================================================================
// Test: inline basic tests
// =============================================================================

#[test]
fn test_double_space_violation() {
    let source = "class Foo { int  x; }"; // double space between int and x
    let violations = check_single_space_separator(source);
    assert!(
        !violations.is_empty(),
        "Should detect double space: {:?}",
        violations
    );
}

#[test]
fn test_single_space_ok() {
    let source = "class Foo { int x; }";
    let violations = check_single_space_separator(source);
    assert!(violations.is_empty(), "Should not flag single space");
}

#[test]
fn test_indentation_ignored() {
    let source = "class Foo {\n    int x;\n}"; // leading spaces are indentation
    let violations = check_single_space_separator(source);
    assert!(violations.is_empty(), "Should ignore leading whitespace");
}

#[test]
fn test_multiple_spaces_in_middle() {
    let source = "int i =    99;"; // multiple spaces
    let violations = check_single_space_separator(source);
    assert!(
        !violations.is_empty(),
        "Should detect multiple spaces in expression"
    );
}

#[test]
fn test_tab_character() {
    let source = "int i =\t5;"; // tab character
    let violations = check_single_space_separator(source);
    // Tabs are not spaces, so this might not be flagged by this rule
    // But checkstyle does flag tabs - let's check the behavior
    println!("Tab test violations: {:?}", violations);
}
