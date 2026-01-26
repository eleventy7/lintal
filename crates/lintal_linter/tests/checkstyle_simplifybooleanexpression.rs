//! SimplifyBooleanExpression checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::SimplifyBooleanExpression;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

#[derive(Debug, Clone)]
struct Violation {
    line: usize,
}

fn check_simplify_boolean_expression(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = SimplifyBooleanExpression;
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
            });
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::coding_test_input("simplifybooleanexpression", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_checkstyle_fixture() {
    let Some(source) = load_fixture("InputSimplifyBooleanExpression.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_simplify_boolean_expression(&source);

    println!("Found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }

    // Should find violations in the checkstyle test file
    assert!(
        !violations.is_empty(),
        "Should find violations in checkstyle test file"
    );
}

#[test]
fn test_equals_true() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b == true) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_equals_false() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b == false) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_not_equals_true() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b != true) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_not_equals_false() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b != false) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_not_true_literal() {
    let source = r#"
class Test {
    void test() {
        boolean x = !true;  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_not_false_literal() {
    let source = r#"
class Test {
    void test() {
        boolean x = !false;  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_or_true() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b || true) {}  // violation (always true)
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_and_false() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (b && false) {}  // violation (always false)
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_reversed_order_true_equals_b() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (true == b) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_reversed_order_false_equals_b() {
    let source = r#"
class Test {
    void test(boolean b) {
        if (false == b) {}  // violation
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_no_false_positives() {
    // Normal boolean expressions should not be flagged
    let source = r#"
class Test {
    void test(boolean a, boolean b) {
        if (a == b) {}
        if (a != b) {}
        if (a && b) {}
        if (a || b) {}
        boolean x = !a;
        if (a) {}
        if (!a) {}
    }
}
"#;
    let violations = check_simplify_boolean_expression(source);
    assert!(
        violations.is_empty(),
        "Normal expressions should not be violations, got: {:?}",
        violations
    );
}

#[test]
fn test_checkstyle_fixture_with_when() {
    let path = checkstyle_repo::coding_test_input(
        "simplifybooleanexpression",
        "InputSimplifyBooleanExpressionWithWhen.java",
    );
    let Some(path) = path else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };
    let source = std::fs::read_to_string(&path).unwrap();

    let violations = check_simplify_boolean_expression(&source);

    println!(
        "WithWhen: Found {} violations (expected 5):",
        violations.len()
    );
    for v in &violations {
        println!("  Line {}", v.line);
    }
}
