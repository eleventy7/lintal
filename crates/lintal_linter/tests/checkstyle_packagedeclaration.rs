//! PackageDeclaration checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::PackageDeclaration;
use lintal_linter::{CheckContext, Rule};

/// Run the PackageDeclaration rule on source code and check for violations.
fn check_package_declaration(source: &str) -> usize {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = PackageDeclaration;
    let ctx = CheckContext::new(source);

    let mut count = 0;
    for node in TreeWalker::new(result.tree.root_node(), source) {
        count += rule.check(&ctx, &node).len();
    }
    count
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::coding_test_input("packagedeclaration", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_input_package_declaration() {
    let Some(source) = load_fixture("InputPackageDeclaration.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    // This file should have a package declaration
    let violations = check_package_declaration(&source);
    println!("InputPackageDeclaration.java: {} violations", violations);
}

#[test]
fn test_with_package_no_violation() {
    let source = r#"
package com.example;

class Foo {}
"#;
    assert_eq!(check_package_declaration(source), 0);
}

#[test]
fn test_without_package_violation() {
    let source = "class Foo {}\n";
    assert_eq!(check_package_declaration(source), 1);
}

#[test]
fn test_imports_but_no_package() {
    let source = r#"
import java.util.List;

class Foo {}
"#;
    assert_eq!(check_package_declaration(source), 1);
}
