//! PackageDeclaration rule implementation.
//!
//! Checks that each source file has a package declaration.
//!
//! Checkstyle equivalent: PackageDeclarationCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: missing package declaration.
#[derive(Debug, Clone)]
pub struct PackageDeclarationViolation;

impl Violation for PackageDeclarationViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Missing package declaration.".to_string()
    }
}

/// Configuration for PackageDeclaration rule.
#[derive(Debug, Clone, Default)]
pub struct PackageDeclaration;

const RELEVANT_KINDS: &[&str] = &["program"];

impl FromConfig for PackageDeclaration {
    const MODULE_NAME: &'static str = "PackageDeclaration";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for PackageDeclaration {
    fn name(&self) -> &'static str {
        "PackageDeclaration"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at the root node
        if node.parent().is_some() {
            return vec![];
        }

        // Look for a package_declaration child
        let has_package = node
            .children()
            .any(|child| child.kind() == "package_declaration");

        if !has_package {
            return vec![Diagnostic::new(PackageDeclarationViolation, node.range())];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = PackageDeclaration;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_with_package_no_violation() {
        let source = r#"
package com.example;

class Foo {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_without_package_violation() {
        let source = r#"
class Foo {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_with_imports_but_no_package_violation() {
        let source = r#"
import java.util.List;

class Foo {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }
}
