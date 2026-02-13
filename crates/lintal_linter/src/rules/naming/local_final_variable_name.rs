//! LocalFinalVariableName rule implementation.
//!
//! Checks that final local variable names conform to a specified pattern.
//! Also checks final parameters and try-with-resources variables.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for local final variable names: camelCase starting with lowercase, or just underscore
const DEFAULT_FORMAT: &str = r"^([a-z][a-zA-Z0-9]*|_)$";

/// Node kinds that we need to check
const RELEVANT_KINDS: &[&str] = &[
    "local_variable_declaration",
    "enhanced_for_statement",
    "formal_parameter",
    "catch_formal_parameter",
    "resource",
];

/// Token types that this rule can check
#[derive(Debug, Clone, Default)]
struct Tokens {
    variable_def: bool,
    parameter_def: bool,
    resource: bool,
}

impl Tokens {
    fn all() -> Self {
        Self {
            variable_def: true,
            parameter_def: true,
            resource: true,
        }
    }

    fn from_str(s: &str) -> Self {
        let mut tokens = Tokens::default();
        for part in s.split(',') {
            match part.trim() {
                "VARIABLE_DEF" => tokens.variable_def = true,
                "PARAMETER_DEF" => tokens.parameter_def = true,
                "RESOURCE" => tokens.resource = true,
                _ => {}
            }
        }
        tokens
    }
}

/// Configuration for LocalFinalVariableName rule.
#[derive(Debug, Clone)]
pub struct LocalFinalVariableName {
    /// Regex pattern for valid local final variable names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Which token types to check
    tokens: Tokens,
}

impl Default for LocalFinalVariableName {
    fn default() -> Self {
        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            tokens: Tokens::all(),
        }
    }
}

impl FromConfig for LocalFinalVariableName {
    const MODULE_NAME: &'static str = "LocalFinalVariableName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

        let tokens = properties
            .get("tokens")
            .map(|v| Tokens::from_str(v))
            .unwrap_or_else(Tokens::all);

        Self {
            format,
            format_str,
            tokens,
        }
    }
}

/// Violation for local final variable name not matching pattern.
#[derive(Debug, Clone)]
pub struct LocalFinalVariableNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for LocalFinalVariableNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for LocalFinalVariableName {
    fn name(&self) -> &'static str {
        "LocalFinalVariableName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "local_variable_declaration" if self.tokens.variable_def => {
                self.check_local_variable(ctx, node)
            }
            "enhanced_for_statement" if self.tokens.variable_def => {
                self.check_enhanced_for(ctx, node)
            }
            "formal_parameter" if self.tokens.parameter_def => {
                self.check_final_parameter(ctx, node)
            }
            "catch_formal_parameter" if self.tokens.parameter_def => {
                self.check_catch_parameter(ctx, node)
            }
            "resource" if self.tokens.resource => self.check_resource(ctx, node),
            _ => vec![],
        }
    }
}

impl LocalFinalVariableName {
    /// Check a final local variable declaration.
    fn check_local_variable(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check final variables
        if !self.has_final_modifier(node) {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Find variable declarators
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let var_name = &ctx.source()[name_node.range()];

                    if !self.format.is_match(var_name) {
                        diagnostics.push(Diagnostic::new(
                            LocalFinalVariableNameInvalid {
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

    /// Check an enhanced for statement (for-each loop) with final variable.
    fn check_enhanced_for(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check if the loop variable is final
        if !self.enhanced_for_has_final(node) {
            return vec![];
        }

        // Get the loop variable identifier (it's the identifier that's not after the colon)
        let mut found_colon = false;
        let mut var_name_node = None;

        for child in node.children() {
            if child.kind() == ":" {
                found_colon = true;
            } else if child.kind() == "identifier" && !found_colon {
                var_name_node = Some(child);
            }
        }

        let Some(name_node) = var_name_node else {
            return vec![];
        };

        let var_name = &ctx.source()[name_node.range()];

        if !self.format.is_match(var_name) {
            return vec![Diagnostic::new(
                LocalFinalVariableNameInvalid {
                    name: var_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )];
        }

        vec![]
    }

    /// Check a final method/constructor parameter.
    fn check_final_parameter(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Skip if this is a lambda parameter
        if self.is_lambda_parameter(node) {
            return vec![];
        }

        // Only check final parameters
        if !self.has_final_modifier(node) {
            return vec![];
        }

        // Get parameter name
        let Some(name_node) = node.children().find(|c| c.kind() == "identifier") else {
            return vec![];
        };

        let param_name = &ctx.source()[name_node.range()];

        if !self.format.is_match(param_name) {
            return vec![Diagnostic::new(
                LocalFinalVariableNameInvalid {
                    name: param_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )];
        }

        vec![]
    }

    /// Check a catch clause parameter.
    fn check_catch_parameter(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check final catch parameters
        if !self.has_final_modifier(node) {
            return vec![];
        }

        // Get parameter name (the identifier after the type)
        let Some(name_node) = node.children().find(|c| c.kind() == "identifier") else {
            return vec![];
        };

        let param_name = &ctx.source()[name_node.range()];

        if !self.format.is_match(param_name) {
            return vec![Diagnostic::new(
                LocalFinalVariableNameInvalid {
                    name: param_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )];
        }

        vec![]
    }

    /// Check a try-with-resources variable.
    fn check_resource(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Resources are implicitly final in try-with-resources
        // Structure can be either:
        // 1. Type identifier = expr (declaration)
        // 2. identifier (reference to existing variable - Java 9+)
        // 3. field_access (reference to field - Java 9+)

        // Check if this is a declaration (has a type and identifier with =)
        let has_assignment = node.children().any(|c| c.kind() == "=");
        if !has_assignment {
            // This is a reference to an existing variable, not a declaration
            return vec![];
        }

        // Find the identifier (variable name)
        // It's the identifier that comes after the type but before =
        let mut found_type = false;
        for child in node.children() {
            // Skip modifiers if present
            if child.kind() == "modifiers"
                || child.kind() == "final"
                || child.kind() == "modifier"
            {
                continue;
            }
            // Check for type
            if !found_type && self.is_type_node(&child) {
                found_type = true;
                continue;
            }
            // After type, the next identifier is the variable name
            if found_type && child.kind() == "identifier" {
                let var_name = &ctx.source()[child.range()];

                if !self.format.is_match(var_name) {
                    return vec![Diagnostic::new(
                        LocalFinalVariableNameInvalid {
                            name: var_name.to_string(),
                            pattern: self.format_str.clone(),
                        },
                        child.range(),
                    )];
                }
                return vec![];
            }
        }

        vec![]
    }

    /// Check if a node is a type node.
    fn is_type_node(&self, node: &CstNode) -> bool {
        matches!(
            node.kind(),
            "type_identifier"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type"
                | "generic_type"
                | "array_type"
                | "scoped_type_identifier"
        )
    }

    /// Check if a local variable declaration has the final modifier.
    fn has_final_modifier(&self, node: &CstNode) -> bool {
        for child in node.children() {
            if child.kind() == "modifiers" {
                return crate::rules::modifier::common::has_modifier(&child, "final");
            }
            if crate::rules::modifier::common::resolve_modifier_kind(&child) == "final" {
                return true;
            }
        }
        false
    }

    /// Check if an enhanced for loop variable has the final modifier.
    fn enhanced_for_has_final(&self, node: &CstNode) -> bool {
        for child in node.children() {
            match child.kind() {
                "modifiers" => {
                    return crate::rules::modifier::common::has_modifier(&child, "final");
                }
                "final" | "modifier" => {
                    return crate::rules::modifier::common::resolve_modifier_kind(&child)
                        == "final";
                }
                "type_identifier"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type"
                | "generic_type"
                | "array_type" => break,
                _ => {}
            }
        }
        false
    }

    /// Check if this parameter is inside a lambda expression.
    fn is_lambda_parameter(&self, node: &CstNode) -> bool {
        if let Some(parent) = node.parent() {
            if parent.kind() == "formal_parameters" || parent.kind() == "inferred_parameters" {
                if let Some(grandparent) = parent.parent() {
                    return grandparent.kind() == "lambda_expression";
                }
            }
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
        let rule = LocalFinalVariableName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_final_local_variable() {
        let source = "class Foo { void bar() { final int myVar = 1; } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_final_local_variable() {
        let source = "class Foo { void bar() { final int MyVar = 1; } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_non_final_not_checked() {
        let source = "class Foo { void bar() { int MyVar = 1; } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Non-final not checked
    }

    #[test]
    fn test_final_parameter() {
        let source = "class Foo { void bar(final int MyParam) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }
}
