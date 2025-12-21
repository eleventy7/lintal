//! OperatorWrap checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::OperatorWrap;
use lintal_linter::{CheckContext, Rule};

#[derive(Debug, Clone)]
struct Violation {
    line: usize,
}

fn check_operator_wrap_nl(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = OperatorWrap::default();
    let ctx = CheckContext::new(source);
    let source_code = ctx.source_code();

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
    let path = checkstyle_repo::whitespace_test_input("operatorwrap", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_simple_case_first() {
    // Test the exact pattern from line 23-24
    let source = r#"
class Test {
    void test() {
        int x = 1 +
            2;
    }
}
"#;
    let violations = check_operator_wrap_nl(source);
    println!("Simple test found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }
    assert!(!violations.is_empty(), "Should find violation for operator at end of line");
}

#[test]
fn test_operator_wrap_nl_option() {
    let Some(source) = load_fixture("InputOperatorWrap1.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_operator_wrap_nl(&source);

    // From checkstyle test file:
    // Line 23: 1 + (violation)
    // Line 24: 2 - (violation)
    // Line 32: true && (violation)
    // Line 54: Comparable & (violation)
    // Line 67: Foo & (violation)

    println!("Found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }

    // Should find violations on lines 23, 24, 32, 54, 67
    assert!(violations.iter().any(|v| v.line == 23), "Should find violation on line 23");
    assert!(violations.iter().any(|v| v.line == 24), "Should find violation on line 24");
}

#[test]
fn test_same_line_no_violation() {
    let source = r#"
class Test {
    void method() {
        int x = 1 + 2 + 3;
        boolean y = true && false;
    }
}
"#;
    let violations = check_operator_wrap_nl(source);
    assert!(violations.is_empty(), "Same line expressions should not cause violations");
}
