//! MemberName rule implementation.
//!
//! Checks that instance variable names (non-static fields) conform to a specified pattern.
//! Does not check static fields - use StaticVariableName for those.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for member names: camelCase starting with lowercase
const DEFAULT_FORMAT: &str = r"^[a-z][a-zA-Z0-9]*$";

/// Node kinds that represent field declarations
const RELEVANT_KINDS: &[&str] = &["field_declaration"];

/// Configuration for MemberName rule.
#[derive(Debug, Clone)]
pub struct MemberName {
    /// Regex pattern for valid member names
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

impl Default for MemberName {
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

impl FromConfig for MemberName {
    const MODULE_NAME: &'static str = "MemberName";

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

/// Violation for member name not matching pattern.
#[derive(Debug, Clone)]
pub struct MemberNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for MemberNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for MemberName {
    fn name(&self) -> &'static str {
        "MemberName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check field_declaration nodes
        if node.kind() != "field_declaration" {
            return vec![];
        }

        // Skip static fields - those are checked by StaticVariableName/ConstantName
        if self.has_static_modifier(node) {
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
                    let member_name = &ctx.source()[name_node.range()];

                    // Check against pattern
                    if !self.format.is_match(member_name) {
                        diagnostics.push(Diagnostic::new(
                            MemberNameInvalid {
                                name: member_name.to_string(),
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

impl MemberName {
    /// Check if the field has a static modifier.
    fn has_static_modifier(&self, node: &CstNode) -> bool {
        let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers") else {
            return false;
        };

        crate::rules::modifier::common::has_modifier(&modifiers, "static")
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
        let rule = MemberName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_member_name() {
        let source = "class Foo { int myField; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_member_name() {
        let source = "class Foo { int MyField; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_static_field_not_checked() {
        let source = "class Foo { static int MyField; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Static fields are not checked by MemberName
    }

    #[test]
    fn test_underscore_prefix() {
        let source = "class Foo { int _field; }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_custom_format() {
        let source = "class Foo { int _myField; }";
        let mut properties = Properties::new();
        properties.insert("format", "^_[a-z][a-zA-Z0-9]*$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_apply_to_private_false() {
        let source = "class Foo { private int MyField; }";
        let mut properties = Properties::new();
        properties.insert("applyToPrivate", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // Private members not checked
    }
}
