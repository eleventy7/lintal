//! UpperEll checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::UpperEll;
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

/// Run UpperEll rule on source and collect violations.
fn check_upper_ell(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = UpperEll;
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
    let path = checkstyle_repo::misc_test_input("upperell", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_upper_ell_semantic() {
    let Some(source) = load_fixture("InputUpperEllSemantic.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_upper_ell(&source);

    // From the test file: line 29 has `666l` which should be flagged
    // Line 29: `private static final long IGNORE = 666l + 666L;`
    // The lowercase 'l' at column 44 (0-indexed would be different)

    assert_eq!(violations.len(), 1, "Expected 1 violation, got {:?}", violations);
    assert_eq!(violations[0].line, 29, "Violation should be on line 29");
}

#[test]
fn test_uppercase_l_ok() {
    let source = r#"
class Test {
    long a = 123L;
    long b = 0xABCL;
    long c = 0777L;
    long d = 0b1010L;
}
"#;
    let violations = check_upper_ell(source);
    assert!(violations.is_empty(), "Uppercase L should not cause violations");
}

#[test]
fn test_lowercase_l_violation() {
    let source = r#"
class Test {
    long a = 123l;
}
"#;
    let violations = check_upper_ell(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 3);
}

#[test]
fn test_multiple_violations() {
    let source = r#"
class Test {
    long a = 1l;
    long b = 2l;
    long c = 3L;
    long d = 4l;
}
"#;
    let violations = check_upper_ell(source);
    assert_eq!(violations.len(), 3, "Expected 3 violations for lowercase 'l'");
}

#[test]
fn test_hex_lowercase_l() {
    let source = r#"
class Test {
    long a = 0xFFl;
    long b = 0xABCDl;
}
"#;
    let violations = check_upper_ell(source);
    assert_eq!(violations.len(), 2, "Hex literals with lowercase 'l' should be flagged");
}

#[test]
fn test_octal_lowercase_l() {
    let source = r#"
class Test {
    long a = 0777l;
}
"#;
    let violations = check_upper_ell(source);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_binary_lowercase_l() {
    let source = r#"
class Test {
    long a = 0b1010l;
}
"#;
    let violations = check_upper_ell(source);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_fix_content() {
    let source = r#"
class Test {
    long a = 123l;
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();
    let rule = UpperEll;
    let ctx = CheckContext::new(source);

    let mut diagnostics = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        diagnostics.extend(rule.check(&ctx, &node));
    }

    assert_eq!(diagnostics.len(), 1);
    let fix = diagnostics[0].fix.as_ref().expect("Fix should be present");
    let edits = fix.edits();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].content().unwrap(), "L", "Fix should replace 'l' with 'L'");
}
