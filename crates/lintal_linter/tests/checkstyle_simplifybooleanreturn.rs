//! SimplifyBooleanReturn checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::SimplifyBooleanReturn;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

#[derive(Debug, Clone)]
struct Violation {
    line: usize,
}

fn check_simplify_boolean_return(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = SimplifyBooleanReturn;
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
    let path = checkstyle_repo::coding_test_input("simplifybooleanreturn", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_checkstyle_fixture() {
    let Some(source) = load_fixture("InputSimplifyBooleanReturn.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_simplify_boolean_return(&source);

    // From checkstyle test file:
    // Line 22: if (even == true) { return false; } else { return true; }
    // Line 35: if (!even) return true; else return false;

    println!("Found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }

    assert!(violations.iter().any(|v| v.line == 22), "Should find violation on line 22");
    assert!(violations.iter().any(|v| v.line == 35), "Should find violation on line 35");
}

#[test]
fn test_no_false_positives() {
    // Cases that should NOT be violations
    let source = r#"
class Test {
    // No else - not a violation
    boolean noElse(boolean cond) {
        if (cond) {
            return true;
        }
        return false;
    }

    // Non-literal return - not a violation
    boolean nonLiteral(boolean cond, boolean other) {
        if (cond) {
            return true;
        } else {
            return other;
        }
    }

    // Both same literal - not a violation (weird but valid)
    boolean sameLiteral(boolean cond) {
        if (cond) {
            return true;
        } else {
            return true;
        }
    }
}
"#;
    let violations = check_simplify_boolean_return(source);
    assert!(violations.is_empty(), "Should have no violations for valid code, got: {:?}", violations);
}
