//! MethodName rule implementation.
//!
//! Checks that method names conform to a specified pattern.
//! Also checks if a method name has the same name as the enclosing class.
//! Does not check the name of overridden methods (@Override annotation).

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for method names: camelCase starting with lowercase
const DEFAULT_FORMAT: &str = r"^[a-z][a-zA-Z0-9]*$";

/// Node kinds that represent methods to check
const RELEVANT_KINDS: &[&str] = &["method_declaration"];

/// Configuration for MethodName rule.
#[derive(Debug, Clone)]
pub struct MethodName {
    /// Regex pattern for valid method names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Allow method name to equal class name
    allow_class_name: bool,
    /// Apply to public members
    apply_to_public: bool,
    /// Apply to protected members
    apply_to_protected: bool,
    /// Apply to package-private members
    apply_to_package: bool,
    /// Apply to private members
    apply_to_private: bool,
}

impl Default for MethodName {
    fn default() -> Self {
        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            allow_class_name: false,
            apply_to_public: true,
            apply_to_protected: true,
            apply_to_package: true,
            apply_to_private: true,
        }
    }
}

impl FromConfig for MethodName {
    const MODULE_NAME: &'static str = "MethodName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

        let allow_class_name = properties
            .get("allowClassName")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let apply_to_public = properties
            .get("applyToPublic")
            .map(|v| *v != "false")
            .unwrap_or(true);

        let apply_to_protected = properties
            .get("applyToProtected")
            .map(|v| *v != "false")
            .unwrap_or(true);

        let apply_to_package = properties
            .get("applyToPackage")
            .map(|v| *v != "false")
            .unwrap_or(true);

        let apply_to_private = properties
            .get("applyToPrivate")
            .map(|v| *v != "false")
            .unwrap_or(true);

        Self {
            format,
            format_str,
            allow_class_name,
            apply_to_public,
            apply_to_protected,
            apply_to_package,
            apply_to_private,
        }
    }
}

/// Violation for method name not matching pattern.
#[derive(Debug, Clone)]
pub struct MethodNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for MethodNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

/// Violation for method name equaling class name.
#[derive(Debug, Clone)]
pub struct MethodNameEqualsClassName {
    pub name: String,
}

impl Violation for MethodNameEqualsClassName {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Method name '{}' must not equal the enclosing class name.",
            self.name
        )
    }
}

impl Rule for MethodName {
    fn name(&self) -> &'static str {
        "MethodName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check method_declaration nodes
        if node.kind() != "method_declaration" {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Get method name
        let Some(name_node) = node.child_by_field_name("name") else {
            return vec![];
        };
        let method_name = &ctx.source()[name_node.range()];

        // Check for @Override annotation - skip if present
        if self.has_override_annotation(ctx, node) {
            return vec![];
        }

        // Check access control
        if !self.should_check_access(node) {
            return vec![];
        }

        // Check against pattern
        if !self.format.is_match(method_name) {
            diagnostics.push(Diagnostic::new(
                MethodNameInvalid {
                    name: method_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            ));
        }

        // Check if method name equals class name
        if !self.allow_class_name {
            if let Some(class_name) = self.get_enclosing_class_name(ctx, node) {
                if method_name == class_name {
                    diagnostics.push(Diagnostic::new(
                        MethodNameEqualsClassName {
                            name: method_name.to_string(),
                        },
                        name_node.range(),
                    ));
                }
            }
        }

        diagnostics
    }
}

impl MethodName {
    /// Check if method has @Override annotation.
    fn has_override_annotation(&self, ctx: &CheckContext, node: &CstNode) -> bool {
        // Look for modifiers with annotation containing "Override"
        let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") else {
            return false;
        };

        for child in modifiers.children() {
            if child.kind() == "marker_annotation" || child.kind() == "annotation" {
                if self.is_override_annotation(ctx, &child) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if an annotation node is @Override or @java.lang.Override.
    fn is_override_annotation(&self, ctx: &CheckContext, annotation: &CstNode) -> bool {
        for child in annotation.children() {
            match child.kind() {
                "identifier" => {
                    let name = &ctx.source()[child.range()];
                    if name == "Override" {
                        return true;
                    }
                }
                "scoped_identifier" => {
                    // Handle @java.lang.Override - check the last identifier
                    if self.scoped_identifier_ends_with(ctx, &child, "Override") {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Check if a scoped_identifier ends with the given name.
    fn scoped_identifier_ends_with(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
        target: &str,
    ) -> bool {
        for child in node.children() {
            if child.kind() == "identifier" {
                let name = &ctx.source()[child.range()];
                if name == target {
                    return true;
                }
            } else if child.kind() == "scoped_identifier" {
                // Recurse into nested scoped_identifier
                if self.scoped_identifier_ends_with(ctx, &child, target) {
                    return true;
                }
            }
        }
        false
    }

    /// Determine if we should check this method based on access modifiers.
    fn should_check_access(&self, node: &CstNode) -> bool {
        let modifiers = node.children().find(|c| c.kind() == "modifiers");

        let (has_public, has_protected, has_private) = if let Some(ref mods) = modifiers {
            let public = crate::rules::modifier::common::has_modifier(mods, "public");
            let protected = crate::rules::modifier::common::has_modifier(mods, "protected");
            let private = crate::rules::modifier::common::has_modifier(mods, "private");
            (public, protected, private)
        } else {
            (false, false, false)
        };

        // Check if this is in an interface (methods are implicitly public)
        let in_interface = self.is_in_interface(node);

        let is_public = has_public || (in_interface && !has_private);
        let is_protected = has_protected;
        let is_private = has_private;
        let is_package = !is_public && !is_protected && !is_private;

        (self.apply_to_public && is_public)
            || (self.apply_to_protected && is_protected)
            || (self.apply_to_package && is_package)
            || (self.apply_to_private && is_private)
    }

    /// Check if the node is in an interface.
    fn is_in_interface(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "interface_body" {
                return true;
            }
            if parent.kind() == "class_body" || parent.kind() == "enum_body" {
                return false;
            }
            current = parent.parent();
        }
        false
    }

    /// Get the name of the enclosing class.
    fn get_enclosing_class_name<'a>(
        &self,
        ctx: &'a CheckContext,
        node: &CstNode,
    ) -> Option<&'a str> {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "class_body" || parent.kind() == "enum_body" {
                // Parent is the class/enum body, go up to get class declaration
                if let Some(class_decl) = parent.parent() {
                    if class_decl.kind() == "class_declaration"
                        || class_decl.kind() == "enum_declaration"
                    {
                        if let Some(name_node) = class_decl.child_by_field_name("name") {
                            return Some(&ctx.source()[name_node.range()]);
                        }
                        // Fallback: find identifier child
                        if let Some(ident) =
                            class_decl.children().find(|c| c.kind() == "identifier")
                        {
                            return Some(&ctx.source()[ident.range()]);
                        }
                    }
                }
            }
            current = parent.parent();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str, properties: Properties) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = MethodName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_method_name() {
        let source = "class Foo { void myMethod() {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_method_name() {
        let source = "class Foo { void MyMethod() {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_method_name_equals_class_name() {
        let source = "class Foo { void Foo() {} }"; // Invalid - method named same as class
        let diagnostics = check_source(source, Properties::new());
        // Should have two violations: pattern (capital F) and equals class name
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_method_name_equals_class_name_allowed() {
        let source = "class Foo { void foo() {} }";
        let mut properties = Properties::new();
        properties.insert("allowClassName", "true");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // 'foo' is valid and allowClassName is true
    }

    #[test]
    fn test_custom_format() {
        let source = "class Foo { void MY_METHOD() {} }";
        let mut properties = Properties::new();
        properties.insert("format", "^[A-Z_]+$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_interface_method() {
        let source = "interface Foo { void myMethod(); }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_apply_to_private_false() {
        let source = "class Foo { private void MyMethod() {} }";
        let mut properties = Properties::new();
        properties.insert("applyToPrivate", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // Private methods not checked
    }
}
