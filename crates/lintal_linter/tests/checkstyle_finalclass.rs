//! FinalClass checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::FinalClass;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

#[derive(Debug, Clone)]
struct Violation {
    line: usize,
}

fn check_final_class(source: &str) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = FinalClass;
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
    let path = checkstyle_repo::design_test_input("finalclass", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_checkstyle_fixture() {
    let Some(source) = load_fixture("InputFinalClass.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_final_class(&source);

    println!("Found {} violations:", violations.len());
    for v in &violations {
        println!("  Line {}", v.line);
    }

    // The checkstyle test file should have violations
    assert!(
        !violations.is_empty(),
        "Should find violations in checkstyle test file"
    );
}

#[test]
fn test_class_with_private_constructor() {
    let source = r#"
class Singleton {
    private Singleton() {}
}
"#;
    let violations = check_final_class(source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].line, 2);
}

#[test]
fn test_class_already_final() {
    let source = r#"
final class Singleton {
    private Singleton() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_class_with_public_constructor() {
    let source = r#"
class Normal {
    public Normal() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_class_with_no_constructor() {
    let source = r#"
class Default {
    void method() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_abstract_class() {
    let source = r#"
abstract class Base {
    private Base() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_class_with_mixed_constructors() {
    let source = r#"
class Mixed {
    private Mixed(int x) {}
    public Mixed() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_class_with_all_private_constructors() {
    let source = r#"
class AllPrivate {
    private AllPrivate() {}
    private AllPrivate(int x) {}
}
"#;
    let violations = check_final_class(source);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_utility_class() {
    let source = r#"
public class Utils {
    private Utils() {
        throw new UnsupportedOperationException();
    }

    public static void helper() {}
}
"#;
    let violations = check_final_class(source);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_protected_constructor_not_flagged() {
    let source = r#"
class Base {
    protected Base() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_package_private_constructor_not_flagged() {
    let source = r#"
class PackageAccess {
    PackageAccess() {}
}
"#;
    let violations = check_final_class(source);
    assert!(violations.is_empty());
}

#[test]
fn test_no_false_positives() {
    // Various valid patterns that should not be flagged
    let source = r#"
// Normal class with public constructor
class Normal {
    public Normal() {}
}

// Abstract class with private constructor
abstract class AbstractBase {
    private AbstractBase() {}
}

// Class with protected constructor (for subclassing)
class ExtendableBase {
    protected ExtendableBase() {}
}

// Class with no explicit constructor
class ImplicitPublic {
    void doSomething() {}
}

// Class with mixed visibility constructors
class MixedAccess {
    private MixedAccess(int x) {}
    public MixedAccess() {}
}
"#;
    let violations = check_final_class(source);
    assert!(
        violations.is_empty(),
        "Valid patterns should not be violations, got: {:?}",
        violations
    );
}

#[test]
fn test_all_checkstyle_fixtures() {
    let files = [
        "InputFinalClass.java",
        "InputFinalClass2.java",
        "InputFinalClassAnnotation.java",
        "InputFinalClassAnonymousInnerClass.java",
        "InputFinalClassConstructorInRecord.java",
        "InputFinalClassEnum.java",
        "InputFinalClassInnerAndNestedClass.java",
        "InputFinalClassInterface.java",
        "InputFinalClassNestedInEnumWithAnonInnerClass.java",
        "InputFinalClassNestedInInterfaceWithAnonInnerClass.java",
        "InputFinalClassNestedInRecord.java",
        "InputFinalClassNestedStaticClassInsideInnerClass.java",
        "InputFinalClassPrivateCtor.java",
        "InputFinalClassPrivateCtor2.java",
        "InputFinalClassPrivateCtor3.java",
    ];

    let mut total = 0;

    for file in files {
        let Some(source) = load_fixture(file) else {
            eprintln!("Skipping {}: not found", file);
            continue;
        };

        let violations = check_final_class(&source);
        println!("{}: {} violations", file, violations.len());
        total += violations.len();
    }

    println!("\nTotal: {} violations found (expected ~60)", total);
}

#[test]
fn test_anonymous_inner_class_detailed() {
    let Some(source) = load_fixture("InputFinalClassAnonymousInnerClass.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_final_class(&source);
    let found_lines: Vec<usize> = violations.iter().map(|v| v.line).collect();

    // Expected violations per checkstyle comments
    let expected = vec![11, 27, 40, 52, 67, 71, 84, 91];

    println!("InputFinalClassAnonymousInnerClass.java");
    println!("Expected violations at lines: {:?}", expected);
    println!("Found violations at lines: {:?}", found_lines);

    let missing: Vec<_> = expected
        .iter()
        .filter(|e| !found_lines.contains(e))
        .collect();
    let extra: Vec<_> = found_lines
        .iter()
        .filter(|l| !expected.contains(l))
        .collect();

    if !missing.is_empty() {
        println!("Missing (false negatives): {:?}", missing);
    }
    if !extra.is_empty() {
        println!("Extra (false positives): {:?}", extra);
    }

    // Debug: check line 48 (jasper) specifically
    // The anonymous at lines 102-104 should exclude it
    if found_lines.contains(&48) {
        println!("\nDebug: jasper at line 48 was incorrectly flagged");
        println!("Source around line 102-104:");
        for (i, line) in source.lines().enumerate() {
            if (100..=106).contains(&i) {
                println!("  {}: {}", i + 1, line);
            }
        }
    }

    assert_eq!(
        found_lines.len(),
        expected.len(),
        "Should find {} violations, found {}. Missing: {:?}, Extra: {:?}",
        expected.len(),
        found_lines.len(),
        missing,
        extra
    );
}
