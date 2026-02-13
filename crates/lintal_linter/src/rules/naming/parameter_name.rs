//! ParameterName rule implementation.
//!
//! Checks that method/constructor parameter names conform to a specified pattern.
//! Optionally skips parameters of overridden methods.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for parameter names: camelCase starting with lowercase
const DEFAULT_FORMAT: &str = r"^[a-z][a-zA-Z0-9]*$";

/// Node kinds that represent parameters
const RELEVANT_KINDS: &[&str] = &["formal_parameter"];

/// Access modifier flags
#[derive(Debug, Clone, Default)]
struct AccessModifiers {
    public: bool,
    protected: bool,
    package: bool,
    private: bool,
}

impl AccessModifiers {
    fn all() -> Self {
        Self {
            public: true,
            protected: true,
            package: true,
            private: true,
        }
    }

    fn from_str(s: &str) -> Self {
        let mut mods = AccessModifiers::default();
        // Handle escaped characters in checkstyle config (e.g., \t for tab)
        let s = s.replace("\\t", "\t").replace("\\n", "\n");
        for part in s.split(',') {
            // Trim all whitespace including tabs
            let part = part.trim();
            match part {
                "public" => mods.public = true,
                "protected" => mods.protected = true,
                "package" => mods.package = true,
                "private" => mods.private = true,
                _ => {}
            }
        }
        mods
    }
}

/// Configuration for ParameterName rule.
#[derive(Debug, Clone)]
pub struct ParameterName {
    /// Regex pattern for valid parameter names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Whether to ignore parameters in overridden methods
    ignore_overridden: bool,
    /// Which access modifiers to check
    access_modifiers: AccessModifiers,
}

impl Default for ParameterName {
    fn default() -> Self {
        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            ignore_overridden: false,
            access_modifiers: AccessModifiers::all(),
        }
    }
}

impl FromConfig for ParameterName {
    const MODULE_NAME: &'static str = "ParameterName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

        let ignore_overridden = properties
            .get("ignoreOverridden")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let access_modifiers = properties
            .get("accessModifiers")
            .map(|v| AccessModifiers::from_str(v))
            .unwrap_or_else(AccessModifiers::all);

        Self {
            format,
            format_str,
            ignore_overridden,
            access_modifiers,
        }
    }
}

/// Violation for parameter name not matching pattern.
#[derive(Debug, Clone)]
pub struct ParameterNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for ParameterNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for ParameterName {
    fn name(&self) -> &'static str {
        "ParameterName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check formal_parameter nodes
        if node.kind() != "formal_parameter" {
            return vec![];
        }

        // Skip lambda parameters - those are handled by LambdaParameterName
        if self.is_lambda_parameter(node) {
            return vec![];
        }

        // Find the enclosing method or constructor
        let Some(method_node) = self.find_enclosing_method(node) else {
            return vec![];
        };

        // Check if this is an overridden method and we should skip it
        if self.ignore_overridden && self.has_override_annotation(ctx, &method_node) {
            return vec![];
        }

        // Check access modifier
        if !self.should_check_access(&method_node) {
            return vec![];
        }

        // Get parameter name (the identifier in the formal_parameter)
        let Some(name_node) = node.children().find(|c| c.kind() == "identifier") else {
            return vec![];
        };
        let param_name = &ctx.source()[name_node.range()];

        // Check against pattern
        if !self.format.is_match(param_name) {
            return vec![Diagnostic::new(
                ParameterNameInvalid {
                    name: param_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )];
        }

        vec![]
    }
}

impl ParameterName {
    /// Check if this parameter is inside a lambda expression.
    fn is_lambda_parameter(&self, node: &CstNode) -> bool {
        // Check if the grandparent is a lambda_expression
        if let Some(parent) = node.parent() {
            if parent.kind() == "formal_parameters" || parent.kind() == "inferred_parameters" {
                if let Some(grandparent) = parent.parent() {
                    return grandparent.kind() == "lambda_expression";
                }
            }
        }
        false
    }

    /// Find the enclosing method or constructor declaration.
    fn find_enclosing_method<'a>(&self, node: &'a CstNode) -> Option<CstNode<'a>> {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "method_declaration" | "constructor_declaration" => return Some(parent),
                // Stop at class/interface boundaries
                "class_body" | "interface_body" | "enum_body" => return None,
                _ => current = parent.parent(),
            }
        }
        None
    }

    /// Check if method has @Override annotation.
    fn has_override_annotation(&self, ctx: &CheckContext, method: &CstNode) -> bool {
        let Some(modifiers) = method.children().find(|c| c.kind() == "modifiers") else {
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
                if self.scoped_identifier_ends_with(ctx, &child, target) {
                    return true;
                }
            }
        }
        false
    }

    /// Determine if we should check parameters in this method based on access modifiers.
    fn should_check_access(&self, method: &CstNode) -> bool {
        let modifiers = method.children().find(|c| c.kind() == "modifiers");

        let (has_public, has_protected, has_private) = if let Some(ref mods) = modifiers {
            let public = crate::rules::modifier::common::has_modifier(mods, "public");
            let protected = crate::rules::modifier::common::has_modifier(mods, "protected");
            let private = crate::rules::modifier::common::has_modifier(mods, "private");
            (public, protected, private)
        } else {
            (false, false, false)
        };

        // Check if this method is in an interface (implicitly public)
        let in_interface = self.is_in_interface(method);

        let is_public = has_public || (in_interface && !has_private);
        let is_protected = has_protected;
        let is_private = has_private;
        let is_package = !is_public && !is_protected && !is_private;

        (self.access_modifiers.public && is_public)
            || (self.access_modifiers.protected && is_protected)
            || (self.access_modifiers.package && is_package)
            || (self.access_modifiers.private && is_private)
    }

    /// Check if the method is in an interface.
    fn is_in_interface(&self, method: &CstNode) -> bool {
        let mut current = method.parent();
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
        let rule = ParameterName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_parameter_name() {
        let source = "class Foo { void bar(int myParam) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_parameter_name() {
        let source = "class Foo { void bar(int MyParam) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_ignore_overridden() {
        let source = "class Foo { @Override public boolean equals(Object O) { return true; } }";
        let mut properties = Properties::new();
        properties.insert("ignoreOverridden", "true");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_access_modifier_filter() {
        let source = "class Foo { private void bar(int MyParam) {} }";
        let mut properties = Properties::new();
        properties.insert("accessModifiers", "public");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // private method not checked
    }

    #[test]
    fn test_constructor_parameter() {
        let source = "class Foo { Foo(int MyParam) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }
}
