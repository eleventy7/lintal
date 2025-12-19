//! RedundantModifier rule implementation.
//!
//! Checks for redundant modifiers in various contexts.
//! This is a port of the checkstyle RedundantModifierCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for RedundantModifier rule.
#[derive(Debug, Clone)]
pub struct RedundantModifier {
    /// JDK version for version-specific checks (e.g., strictfp in JDK 17+)
    #[allow(dead_code)] // Will be used in Task 9 for strictfp check
    jdk_version: u32,
}

impl Default for RedundantModifier {
    fn default() -> Self {
        Self { jdk_version: 22 }
    }
}

impl FromConfig for RedundantModifier {
    const MODULE_NAME: &'static str = "RedundantModifier";

    fn from_config(properties: &Properties) -> Self {
        let jdk_version = properties
            .get("jdkVersion")
            .and_then(|v| parse_jdk_version(v))
            .unwrap_or(22);
        Self { jdk_version }
    }
}

/// Parse JDK version string (supports "1.8" or "8" format).
fn parse_jdk_version(version_str: &str) -> Option<u32> {
    let version_str = version_str.trim();
    if let Some(stripped) = version_str.strip_prefix("1.") {
        stripped.parse().ok()
    } else {
        version_str.parse().ok()
    }
}

/// Violation for redundant modifier.
#[derive(Debug, Clone)]
pub struct RedundantModifierViolation {
    pub modifier: String,
}

impl Violation for RedundantModifierViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("Redundant '{}' modifier.", self.modifier)
    }
}

impl Rule for RedundantModifier {
    fn name(&self) -> &'static str {
        "RedundantModifier"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "interface_declaration" | "annotation_type_declaration" => {
                self.check_interface_modifiers(ctx, node)
            }
            "field_declaration" | "constant_declaration" => self.check_field_modifiers(ctx, node),
            "method_declaration" | "annotation_type_element_declaration" => {
                self.check_method_modifiers(ctx, node)
            }
            "class_declaration" => self.check_class_modifiers(ctx, node),
            "enum_declaration" => self.check_enum_modifiers(ctx, node),
            _ => vec![],
        }
    }
}

impl RedundantModifier {
    /// Check for redundant modifiers on interface/annotation declarations.
    fn check_interface_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let modifiers = node
            .children()
            .find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Interfaces/annotations are implicitly abstract
            if let Some(abstract_mod) = self.find_modifier(&modifiers, "abstract") {
                diagnostics.push(self.create_diagnostic(ctx, &abstract_mod, "abstract"));
            }

            // Interfaces/annotations are implicitly static when nested
            if let Some(static_mod) = self.find_modifier(&modifiers, "static") {
                diagnostics.push(self.create_diagnostic(ctx, &static_mod, "static"));
            }
        }

        diagnostics
    }

    /// Check for redundant modifiers on field declarations.
    fn check_field_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check if the field is in an interface or annotation
        if !self.is_in_interface_or_annotation(node) {
            return diagnostics;
        }

        // Find modifiers - it's the first child with kind "modifiers"
        let modifiers = node
            .children()
            .find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Interface/annotation fields are implicitly public, static, and final
            if let Some(public_mod) = self.find_modifier(&modifiers, "public") {
                diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
            }
            if let Some(static_mod) = self.find_modifier(&modifiers, "static") {
                diagnostics.push(self.create_diagnostic(ctx, &static_mod, "static"));
            }
            if let Some(final_mod) = self.find_modifier(&modifiers, "final") {
                diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
            }
        }

        diagnostics
    }

    /// Check for redundant modifiers on method declarations.
    fn check_method_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check if the method is in an interface or annotation
        if !self.is_in_interface_or_annotation(node) {
            return diagnostics;
        }

        // Find modifiers - it's the first child with kind "modifiers"
        let modifiers = node
            .children()
            .find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Interface/annotation methods are implicitly public and abstract (unless default/static)
            let has_default = self.find_modifier(&modifiers, "default").is_some();
            let has_static = self.find_modifier(&modifiers, "static").is_some();

            // Public is always redundant
            if let Some(public_mod) = self.find_modifier(&modifiers, "public") {
                diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
            }

            // Abstract is redundant for non-static, non-default methods
            if !has_default
                && !has_static
                && let Some(abstract_mod) = self.find_modifier(&modifiers, "abstract")
            {
                diagnostics.push(self.create_diagnostic(ctx, &abstract_mod, "abstract"));
            }
        }

        diagnostics
    }

    /// Check for redundant modifiers on class declarations inside interfaces.
    fn check_class_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check if the class is directly inside an interface or annotation
        if !self.is_direct_child_of_interface_or_annotation(node) {
            return diagnostics;
        }

        let modifiers = node
            .children()
            .find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Classes inside interfaces are implicitly public and static
            if let Some(public_mod) = self.find_modifier(&modifiers, "public") {
                diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
            }
            if let Some(static_mod) = self.find_modifier(&modifiers, "static") {
                diagnostics.push(self.create_diagnostic(ctx, &static_mod, "static"));
            }
            // Note: abstract is allowed for classes inside interfaces
        }

        diagnostics
    }

    /// Check for redundant modifiers on enum declarations.
    fn check_enum_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check if the enum is inside an interface/annotation or nested in another class
        let is_nested = self.is_nested(node);

        let modifiers = node
            .children()
            .find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Nested enums are implicitly static
            if is_nested
                && let Some(static_mod) = self.find_modifier(&modifiers, "static")
            {
                diagnostics.push(self.create_diagnostic(ctx, &static_mod, "static"));
            }

            // Enums inside interfaces are also implicitly public
            if self.is_direct_child_of_interface_or_annotation(node)
                && let Some(public_mod) = self.find_modifier(&modifiers, "public")
            {
                diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
            }
        }

        diagnostics
    }

    /// Find a modifier by name in a modifiers node.
    fn find_modifier<'a>(&self, modifiers: &CstNode<'a>, modifier_name: &str) -> Option<CstNode<'a>> {
        modifiers
            .children()
            .find(|child| child.kind() == modifier_name)
    }

    /// Check if a node is inside an interface or annotation.
    fn is_in_interface_or_annotation(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "interface_declaration" | "annotation_type_declaration" => return true,
                // Stop at class boundaries (regular or anonymous)
                "class_declaration" | "enum_declaration" => return false,
                // Anonymous class: class_body inside object_creation_expression
                "class_body" => {
                    if let Some(grandparent) = parent.parent()
                        && grandparent.kind() == "object_creation_expression"
                    {
                        return false; // Stop at anonymous class
                    }
                    current = parent.parent();
                }
                _ => current = parent.parent(),
            }
        }
        false
    }

    /// Check if a node is a direct child of an interface or annotation body.
    fn is_direct_child_of_interface_or_annotation(&self, node: &CstNode) -> bool {
        if let Some(parent) = node.parent()
            && matches!(parent.kind(), "interface_body" | "annotation_type_body")
        {
            return true;
        }
        false
    }

    /// Check if a node is nested (inside another type declaration).
    fn is_nested(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "annotation_type_declaration" => return true,
                _ => current = parent.parent(),
            }
        }
        false
    }

    /// Create a diagnostic for a redundant modifier.
    fn create_diagnostic(&self, _ctx: &CheckContext, node: &CstNode, modifier: &str) -> Diagnostic {
        Diagnostic::new(
            RedundantModifierViolation {
                modifier: modifier.to_string(),
            },
            node.range(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str, jdk_version: Option<u32>) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = if let Some(version) = jdk_version {
            RedundantModifier {
                jdk_version: version,
            }
        } else {
            RedundantModifier::default()
        };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_redundant_public_in_interface() {
        let source = "interface Foo { public void test(); }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'public' modifier"));
    }

    #[test]
    fn test_redundant_abstract_in_interface() {
        let source = "interface Foo { abstract void test(); }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'abstract' modifier"));
    }

    #[test]
    fn test_redundant_public_static_final_in_interface_field() {
        let source = "interface Foo { public static final int X = 1; }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 3); // public, static, final
    }

    #[test]
    fn test_no_error_for_static_method_in_interface() {
        let source = "interface Foo { static void test() {} }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_no_error_for_default_method_in_interface() {
        let source = "interface Foo { default void test() {} }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_redundant_static_class_in_interface() {
        let source = "interface Foo { static class Bar {} }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'static' modifier"));
    }

    #[test]
    fn test_redundant_public_class_in_interface() {
        let source = "interface Foo { public class Bar {} }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'public' modifier"));
    }

    #[test]
    fn test_abstract_class_in_interface_is_ok() {
        let source = "interface Foo { abstract class Bar {} }";
        let diagnostics = check_source(source, None);
        // Abstract is allowed for classes inside interfaces
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_redundant_static_enum_in_interface() {
        let source = "interface Foo { static enum Bar { A, B } }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'static' modifier"));
    }

    #[test]
    fn test_annotation_fields() {
        let source = "@interface Annotation { public String s1 = \"\"; public String blah(); }";
        let diagnostics = check_source(source, None);
        // Should find 2 violations: public on field and public on method
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0]
            .kind
            .body
            .contains("Redundant 'public' modifier"));
        assert!(diagnostics[1]
            .kind
            .body
            .contains("Redundant 'public' modifier"));
    }
}
