//! ConstantName rule implementation.
//!
//! Checks that constant names conform to a specified pattern.
//! A constant is a static and final field or an interface/annotation field,
//! except serialVersionUID and serialPersistentFields.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for constant names: UPPER_CASE with underscores
const DEFAULT_FORMAT: &str = r"^[A-Z][A-Z0-9]*(_[A-Z0-9]+)*$";

/// Names that are always excluded from checking (serialization-related)
const EXCLUDED_NAMES: &[&str] = &["serialVersionUID", "serialPersistentFields"];

/// Node kinds that represent constants to check
const RELEVANT_KINDS: &[&str] = &["field_declaration", "constant_declaration"];

/// Configuration for ConstantName rule.
#[derive(Debug, Clone)]
pub struct ConstantName {
    /// Regex pattern for valid constant names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Apply to public members
    apply_to_public: bool,
    /// Apply to protected members
    apply_to_protected: bool,
    /// Apply to package-private members
    apply_to_package: bool,
    /// Apply to private members
    apply_to_private: bool,
}

impl Default for ConstantName {
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

impl FromConfig for ConstantName {
    const MODULE_NAME: &'static str = "ConstantName";

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

/// Violation for constant name not matching pattern.
#[derive(Debug, Clone)]
pub struct ConstantNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for ConstantNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for ConstantName {
    fn name(&self) -> &'static str {
        "ConstantName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "field_declaration" => self.check_field_declaration(ctx, node),
            "constant_declaration" => self.check_constant_declaration(ctx, node),
            _ => vec![],
        }
    }
}

impl ConstantName {
    /// Check a field declaration (class/enum field).
    /// Only checks static final fields.
    fn check_field_declaration(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Must have modifiers with both static and final
        let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") else {
            return vec![];
        };

        let has_static =
            crate::rules::modifier::common::has_modifier(&modifiers, "static");
        let has_final = crate::rules::modifier::common::has_modifier(&modifiers, "final");

        // Not a constant if not static final
        if !has_static || !has_final {
            return vec![];
        }

        // Check if this field is inside a code block (local constant in method)
        if self.is_in_code_block(node) {
            return vec![];
        }

        // Check access control
        if !self.should_check_access(&modifiers, ctx, node) {
            return vec![];
        }

        // Get the variable declarator(s) and check each name
        let mut diagnostics = vec![];
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.children().find(|c| c.kind() == "identifier") {
                    let name = &ctx.source()[name_node.range()];

                    // Skip excluded names
                    if EXCLUDED_NAMES.contains(&name) {
                        continue;
                    }

                    // Check against pattern
                    if !self.format.is_match(name) {
                        diagnostics.push(Diagnostic::new(
                            ConstantNameInvalid {
                                name: name.to_string(),
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

    /// Check a constant declaration (interface/annotation constant).
    /// These are implicitly public static final.
    fn check_constant_declaration(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Interface constants are implicitly public - check if we should apply
        if !self.apply_to_public {
            return vec![];
        }

        // Get the variable declarator(s) and check each name
        let mut diagnostics = vec![];
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.children().find(|c| c.kind() == "identifier") {
                    let name = &ctx.source()[name_node.range()];

                    // Skip excluded names
                    if EXCLUDED_NAMES.contains(&name) {
                        continue;
                    }

                    // Check against pattern
                    if !self.format.is_match(name) {
                        diagnostics.push(Diagnostic::new(
                            ConstantNameInvalid {
                                name: name.to_string(),
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

    /// Check if the field is inside a code block (method body, etc.)
    fn is_in_code_block(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                // Code block contexts where constants shouldn't be checked
                "block" | "constructor_body" | "static_initializer" | "instance_initializer" => {
                    return true;
                }
                // Stop at class/interface level
                "class_body" | "interface_body" | "enum_body" | "annotation_type_body" => {
                    return false;
                }
                _ => {}
            }
            current = parent.parent();
        }
        false
    }

    /// Determine if we should check this member based on access modifiers.
    fn should_check_access(
        &self,
        modifiers: &CstNode,
        _ctx: &CheckContext,
        node: &CstNode,
    ) -> bool {
        let has_public = crate::rules::modifier::common::has_modifier(modifiers, "public");
        let has_protected =
            crate::rules::modifier::common::has_modifier(modifiers, "protected");
        let has_private = crate::rules::modifier::common::has_modifier(modifiers, "private");

        // Check if in interface or annotation (implicitly public)
        let in_interface = self.is_in_interface_or_annotation(node);

        let is_public = has_public || (in_interface && !has_private);
        let is_protected = has_protected;
        let is_private = has_private;
        let is_package = !is_public && !is_protected && !is_private;

        (self.apply_to_public && is_public)
            || (self.apply_to_protected && is_protected)
            || (self.apply_to_package && is_package)
            || (self.apply_to_private && is_private)
    }

    /// Check if the node is inside an interface or annotation type.
    fn is_in_interface_or_annotation(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "interface_body" | "annotation_type_body" => return true,
                "class_body" | "enum_body" => return false,
                _ => {}
            }
            current = parent.parent();
        }
        false
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
        let rule = ConstantName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_constant_name() {
        let source = "class Foo { public static final int MAX_VALUE = 100; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_constant_name() {
        let source = "class Foo { public static final int badConstant = 100; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_serial_version_uid_ignored() {
        let source = "class Foo { private static final long serialVersionUID = 1L; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_interface_constant() {
        let source = "interface Foo { int BAD_NAME = 1; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // BAD_NAME matches the pattern

        let source2 = "interface Foo { int badName = 1; }";
        let diagnostics2 = check_source(source2, Properties::new());
        assert_eq!(diagnostics2.len(), 1);
    }

    #[test]
    fn test_non_constant_ignored() {
        // Not static - not a constant
        let source = "class Foo { final int notConstant = 100; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);

        // Not final - not a constant
        let source2 = "class Foo { static int notConstant = 100; }";
        let diagnostics2 = check_source(source2, Properties::new());
        assert_eq!(diagnostics2.len(), 0);
    }

    #[test]
    fn test_apply_to_private_false() {
        let source = "class Foo { private static final int bad = 100; }";
        let mut properties = Properties::new();
        properties.insert("applyToPrivate", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_custom_format() {
        let source = "class Foo { public static final int myConst = 100; }";
        let mut properties = Properties::new();
        properties.insert("format", "^[a-z][a-zA-Z0-9]*$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // myConst matches camelCase pattern
    }

    #[test]
    fn test_double_underscore_invalid() {
        let source = "class Foo { public static final int BAD__NAME = 100; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1); // Double underscore not allowed by default pattern
    }
}
