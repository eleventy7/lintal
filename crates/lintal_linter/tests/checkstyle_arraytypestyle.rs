//! ArrayTypeStyle checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::ArrayTypeStyle;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

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

/// Run ArrayTypeStyle rule on source and collect violations.
fn check_array_type_style(source: &str, java_style: bool) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = ArrayTypeStyle { java_style };
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
    let path = checkstyle_repo::misc_test_input("arraytypestyle", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_array_type_style_java_style() {
    let Some(source) = load_fixture("InputArrayTypeStyle.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_array_type_style(&source, true);

    // From the test file comments:
    // Line 13: cStyle[] - violation
    // Line 14: c[] - violation
    // Line 20: aCStyle[] - violation (parameter)
    // Line 44: getOldTest()[] - violation (method return type)
    // Line 49-52: getOldTests()[][] - 2 violations
    // Line 57-61: getMoreTests()[][] - 2 violations

    // We should have at least 7 violations for C-style array declarations
    assert!(
        violations.len() >= 3,
        "Expected at least 3 violations for C-style arrays, got {:?}",
        violations
    );

    // Check for specific violations
    let has_line_13 = violations.iter().any(|v| v.line == 13);
    let has_line_14 = violations.iter().any(|v| v.line == 14);
    let has_line_20 = violations.iter().any(|v| v.line == 20);

    assert!(has_line_13, "Should have violation on line 13 (cStyle[])");
    assert!(has_line_14, "Should have violation on line 14 (c[])");
    assert!(has_line_20, "Should have violation on line 20 (aCStyle[] parameter)");
}

#[test]
fn test_array_type_style_c_style() {
    let Some(source) = load_fixture("InputArrayTypeStyleOff.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_array_type_style(&source, false);

    // When javaStyle=false, Java-style declarations are violations
    // Line 12: int[] javaStyle - violation
    // Line 16-17: String[] aJavaStyle - violation (parameter)
    // Line 23: int[] blah - violation
    // Line 31: Test[] variable - violation
    // Line 45: getOldTest()[] - still violation (method return type always)

    assert!(
        violations.len() >= 3,
        "Expected at least 3 violations for Java-style arrays when javaStyle=false, got {:?}",
        violations
    );

    // Line 12 should have a violation
    let has_line_12 = violations.iter().any(|v| v.line == 12);
    assert!(has_line_12, "Should have violation on line 12 (int[] javaStyle)");
}

#[test]
fn test_java_style_ok() {
    let source = r#"
class Test {
    int[] nums;
    String[] args;
    void foo(int[] param) {}
}
"#;
    let violations = check_array_type_style(source, true);
    assert!(violations.is_empty(), "Java-style should not cause violations in java mode");
}

#[test]
fn test_c_style_violation() {
    let source = r#"
class Test {
    int nums[];
}
"#;
    let violations = check_array_type_style(source, true);
    assert_eq!(violations.len(), 1, "C-style should cause 1 violation");
    assert_eq!(violations[0].line, 3);
}

#[test]
fn test_method_return_type_c_style() {
    let source = r#"
class Test {
    byte getData()[] {
        return null;
    }
}
"#;
    let violations = check_array_type_style(source, true);
    assert_eq!(violations.len(), 1, "Method return type with C-style should be violation");
}

#[test]
fn test_method_return_type_java_style_ok() {
    let source = r#"
class Test {
    byte[] getData() {
        return null;
    }
}
"#;
    let violations = check_array_type_style(source, true);
    assert!(violations.is_empty(), "Java-style method return should not cause violations");
}

#[test]
fn test_multi_dimensional_c_style() {
    let source = r#"
class Test {
    int nums[][];
}
"#;
    let violations = check_array_type_style(source, true);
    // Multi-dimensional C-style should be flagged
    assert!(!violations.is_empty(), "Multi-dimensional C-style should cause violations");
}

#[test]
fn test_parameter_c_style() {
    let source = r#"
class Test {
    void foo(String args[]) {}
}
"#;
    let violations = check_array_type_style(source, true);
    assert_eq!(violations.len(), 1, "Parameter with C-style should be violation");
}

#[test]
fn test_local_variable_c_style() {
    let source = r#"
class Test {
    void foo() {
        int local[];
    }
}
"#;
    let violations = check_array_type_style(source, true);
    assert_eq!(violations.len(), 1, "Local variable with C-style should be violation");
}

#[test]
fn test_c_mode_flags_java_style() {
    let source = r#"
class Test {
    int[] nums;
}
"#;
    let violations = check_array_type_style(source, false);
    assert_eq!(violations.len(), 1, "Java-style should cause violation in C mode");
}

#[test]
fn test_c_mode_allows_c_style() {
    let source = r#"
class Test {
    int nums[];
}
"#;
    let violations = check_array_type_style(source, false);
    assert!(violations.is_empty(), "C-style should not cause violations in C mode");
}

#[test]
fn test_instanceof_not_flagged() {
    let source = r#"
class Test {
    void foo(String[] args) {
        boolean isOK = args instanceof String[];
    }
}
"#;
    let violations = check_array_type_style(source, true);
    assert!(violations.is_empty(), "instanceof checks should not be flagged");
}

#[test]
fn test_fix_content() {
    let source = r#"
class Test {
    int nums[];
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();
    let rule = ArrayTypeStyle { java_style: true };
    let ctx = CheckContext::new(source);

    let mut diagnostics = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        diagnostics.extend(rule.check(&ctx, &node));
    }

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("Fix should be present");
    let edits = fix.edits();
    assert!(edits.len() >= 1, "Fix should have edits");
}
