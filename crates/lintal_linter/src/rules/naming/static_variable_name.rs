//! StaticVariableName rule implementation.
//!
//! Checks that static, non-final variable names conform to a specified pattern.
//! Does not check:
//! - Instance fields (non-static) - use MemberName for those
//! - Final static fields - use ConstantName for those
//! - Interface fields (implicitly public static final)
//! - Annotation fields (implicitly public static final)

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for static variable names: camelCase starting with lowercase
const DEFAULT_FORMAT: &str = r"^[a-z][a-zA-Z0-9]*$";

/// Node kinds that represent field declarations
const RELEVANT_KINDS: &[&str] = &["field_declaration"];

/// Configuration for StaticVariableName rule.
#[derive(Debug, Clone)]
pub struct StaticVariableName {
    /// Regex pattern for valid static variable names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Apply to public fields
    apply_to_public: bool,
    /// Apply to protected fields
    apply_to_protected: bool,
    /// Apply to package-private fields
    apply_to_package: bool,
    /// Apply to private fields
    apply_to_private: bool,
}

impl Default for StaticVariableName {
    fn default() -> Self {
        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            apply_to_public: true,
            apply_to_protected: true,
            apply_to_package: true,
            apply_to_private: true,
        }
    }
}

impl FromConfig for StaticVariableName {
    const MODULE_NAME: &'static str = "StaticVariableName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

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
            apply_to_public,
            apply_to_protected,
            apply_to_package,
            apply_to_private,
        }
    }
}

/// Violation for static variable name not matching pattern.
#[derive(Debug, Clone)]
pub struct StaticVariableNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for StaticVariableNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for StaticVariableName {
    fn name(&self) -> &'static str {
        "StaticVariableName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check field_declaration nodes
        if node.kind() != "field_declaration" {
            return vec![];
        }

        // Must be a static field
        if !self.has_static_modifier(node) {
            return vec![];
        }

        // Skip final fields - those are checked by ConstantName
        if self.has_final_modifier(node) {
            return vec![];
        }

        // Skip fields in interfaces and annotations (implicitly public static final)
        if self.is_in_interface_or_annotation(node) {
            return vec![];
        }

        // Check access control
        if !self.should_check_access(node) {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Find variable declarators within the field declaration
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let var_name = &ctx.source()[name_node.range()];

                    // Check against pattern
                    if !self.format.is_match(var_name) {
                        diagnostics.push(Diagnostic::new(
                            StaticVariableNameInvalid {
                                name: var_name.to_string(),
                                pattern: self.format_str.clone(),
                            },
                            name_node.range(),
                        ));
                    }
                }
            }
        }

        diagnostics
    }
}

impl StaticVariableName {
    /// Check if the field has a static modifier.
    fn has_static_modifier(&self, node: &CstNode) -> bool {
        let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") else {
            return false;
        };

        crate::rules::modifier::common::has_modifier(&modifiers, "static")
    }

    /// Check if the field has a final modifier.
    fn has_final_modifier(&self, node: &CstNode) -> bool {
        let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") else {
            return false;
        };

        crate::rules::modifier::common::has_modifier(&modifiers, "final")
    }

    /// Check if the field is inside an interface or annotation declaration.
    fn is_in_interface_or_annotation(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "interface_declaration" | "annotation_type_declaration" => return true,
                "class_declaration" | "enum_declaration" => return false,
                _ => current = parent.parent(),
            }
        }
        false
    }

    /// Determine if we should check this field based on access modifiers.
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

        let is_public = has_public;
        let is_protected = has_protected;
        let is_private = has_private;
        let is_package = !is_public && !is_protected && !is_private;

        (self.apply_to_public && is_public)
            || (self.apply_to_protected && is_protected)
            || (self.apply_to_package && is_package)
            || (self.apply_to_private && is_private)
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
        let rule = StaticVariableName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_static_name() {
        let source = "class Foo { static int myStatic; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_static_name() {
        let source = "class Foo { static int MyStatic; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_final_static_not_checked() {
        let source = "class Foo { static final int MY_CONSTANT = 1; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Final statics are checked by ConstantName
    }

    #[test]
    fn test_instance_field_not_checked() {
        let source = "class Foo { int MyField; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Instance fields are checked by MemberName
    }

    #[test]
    fn test_interface_field_not_checked() {
        let source = "interface Foo { int MY_VAL = 1; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Interface fields are implicitly public static final
    }

    #[test]
    fn test_annotation_field_not_checked() {
        let source = "@interface Foo { int MY_VAL = 1; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Annotation fields are implicitly public static final
    }

    #[test]
    fn test_custom_format() {
        let source = "class Foo { static int sMyVar; }";
        let mut properties = Properties::new();
        properties.insert("format", r"^s[A-Z][a-zA-Z0-9]*$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_custom_format_violation() {
        let source = "class Foo { static int badStatic; }";
        let mut properties = Properties::new();
        properties.insert("format", r"^s[A-Z][a-zA-Z0-9]*$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_apply_to_private_false() {
        let source = "class Foo { private static int BadName; }";
        let mut properties = Properties::new();
        properties.insert("applyToPrivate", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // Private statics not checked
    }
}
