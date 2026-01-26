//! EmptyStatement checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::EmptyStatement;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

#[derive(Debug, Clone)]
struct Violation {
    line: usize,
}

fn check_empty_statement(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = EmptyStatement;
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
    let path = checkstyle_repo::coding_test_input("emptystatement", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_checkstyle_fixture() {
    let Some(source) = load_fixture("InputEmptyStatement.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_empty_statement(&source);

    println!("Found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }

    // From checkstyle test InputEmptyStatement.java:
    // The file has several empty statements like:
    // - Line 23: for (; i < 5; i++);  (empty for body - semicolon)
    // - etc.
    // We should find multiple violations
    assert!(
        !violations.is_empty(),
        "Should find violations in checkstyle test file"
    );
}

#[test]
fn test_empty_if_body() {
    let source = r#"
class Test {
    void test() {
        if (true);  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_empty_while_body() {
    let source = r#"
class Test {
    void test() {
        while (condition);  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_empty_for_body() {
    let source = r#"
class Test {
    void test() {
        for (int i = 0; i < 10; i++);  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_empty_enhanced_for_body() {
    let source = r#"
class Test {
    void test(int[] arr) {
        for (int x : arr);  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_empty_do_body() {
    let source = r#"
class Test {
    void test() {
        do; while (condition);  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_standalone_semicolon() {
    let source = r#"
class Test {
    void test() {
        ;  // violation
    }
}
"#;
    let violations = check_empty_statement(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 4);
}

#[test]
fn test_no_false_positives() {
    // Normal statements should not be flagged
    let source = r#"
class Test {
    void test() {
        int x = 5;
        if (true) {
            doSomething();
        }
        while (condition) {
            process();
        }
        for (int i = 0; i < 10; i++) {
            work(i);
        }
    }
}
"#;
    let violations = check_empty_statement(source);
    assert!(
        violations.is_empty(),
        "Normal statements should not be violations, got: {:?}",
        violations
    );
}

#[test]
fn test_for_loop_internal_semicolons_no_violation() {
    // The semicolons inside for loop syntax should not be flagged
    let source = r#"
class Test {
    void test() {
        for (;;) {
            break;
        }
        for (int i = 0;;) {
            if (i > 10) break;
        }
    }
}
"#;
    let violations = check_empty_statement(source);
    assert!(
        violations.is_empty(),
        "For loop internal semicolons should not be violations, got: {:?}",
        violations
    );
}
