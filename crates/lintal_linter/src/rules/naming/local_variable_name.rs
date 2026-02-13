//! LocalVariableName rule implementation.
//!
//! Checks that local variable names conform to a specified pattern.
//! Does not check final local variables if they should be checked by LocalFinalVariableName.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for local variable names: camelCase starting with lowercase, or just underscore
const DEFAULT_FORMAT: &str = r"^([a-z][a-zA-Z0-9]*|_)$";

/// Node kinds that represent local variables
const RELEVANT_KINDS: &[&str] = &["local_variable_declaration", "enhanced_for_statement"];

/// Configuration for LocalVariableName rule.
#[derive(Debug, Clone)]
pub struct LocalVariableName {
    /// Regex pattern for valid local variable names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Allow single character variable names in for loops
    allow_one_char_var_in_for_loop: bool,
}

impl Default for LocalVariableName {
    fn default() -> Self {
        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            allow_one_char_var_in_for_loop: false,
        }
    }
}

impl FromConfig for LocalVariableName {
    const MODULE_NAME: &'static str = "LocalVariableName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

        let allow_one_char_var_in_for_loop = properties
            .get("allowOneCharVarInForLoop")
            .map(|v| *v == "true")
            .unwrap_or(false);

        Self {
            format,
            format_str,
            allow_one_char_var_in_for_loop,
        }
    }
}

/// Violation for local variable name not matching pattern.
#[derive(Debug, Clone)]
pub struct LocalVariableNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for LocalVariableNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for LocalVariableName {
    fn name(&self) -> &'static str {
        "LocalVariableName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "local_variable_declaration" => self.check_local_variable(ctx, node),
            "enhanced_for_statement" => self.check_enhanced_for(ctx, node),
            _ => vec![],
        }
    }
}

impl LocalVariableName {
    /// Check a local variable declaration.
    fn check_local_variable(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Skip final variables - those are handled by LocalFinalVariableName
        if self.has_final_modifier(node) {
            return vec![];
        }

        // Skip if this is in a for statement init (those are loop variables)
        let in_for_init = self.is_in_for_init(node);

        let mut diagnostics = vec![];

        // Find variable declarators
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let var_name = &ctx.source()[name_node.range()];

                    // Skip single-char vars in for loops if allowed
                    if in_for_init && self.allow_one_char_var_in_for_loop && var_name.len() == 1 {
                        continue;
                    }

                    if !self.format.is_match(var_name) {
                        diagnostics.push(Diagnostic::new(
                            LocalVariableNameInvalid {
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

    /// Check an enhanced for statement (for-each loop).
    fn check_enhanced_for(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Skip if the loop variable is final
        if self.enhanced_for_has_final(node) {
            return vec![];
        }

        // Get the loop variable identifier (it's the identifier that's not after the colon)
        // Structure: for ( Type identifier : iterable ) ...
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

        // Skip single-char vars if allowed
        if self.allow_one_char_var_in_for_loop && var_name.len() == 1 {
            return vec![];
        }

        if !self.format.is_match(var_name) {
            return vec![Diagnostic::new(
                LocalVariableNameInvalid {
                    name: var_name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )];
        }

        vec![]
    }

    /// Check if the node is in a for statement initialization.
    fn is_in_for_init(&self, node: &CstNode) -> bool {
        if let Some(parent) = node.parent() {
            return parent.kind() == "for_statement";
        }
        false
    }

    /// Check if a local variable declaration has the final modifier.
    fn has_final_modifier(&self, node: &CstNode) -> bool {
        for child in node.children() {
            if child.kind() == "modifiers" {
                return crate::rules::modifier::common::has_modifier(&child, "final");
            }
            // Final can also appear directly (or wrapped in modifier node)
            if crate::rules::modifier::common::resolve_modifier_kind(&child) == "final" {
                return true;
            }
        }
        false
    }

    /// Check if an enhanced for loop variable has the final modifier.
    fn enhanced_for_has_final(&self, node: &CstNode) -> bool {
        // In enhanced_for_statement, final modifier can appear before the type
        for child in node.children() {
            match child.kind() {
                "modifiers" => {
                    return crate::rules::modifier::common::has_modifier(&child, "final");
                }
                "final" | "modifier" => {
                    return crate::rules::modifier::common::resolve_modifier_kind(&child)
                        == "final";
                }
                // Stop at the type - modifiers come before
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
        let rule = LocalVariableName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_local_variable_name() {
        let source = "class Foo { void bar() { int myVar = 1; } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_local_variable_name() {
        let source = "class Foo { void bar() { int MyVar = 1; } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_for_loop_variable() {
        let source = "class Foo { void bar() { for(int I=0;I<10;I++){} } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_enhanced_for_loop_variable() {
        let source = "class Foo { void bar() { for(Object O : list){} } }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_allow_one_char_in_for_loop() {
        let source = "class Foo { void bar() { for(int I=0;I<10;I++){} } }";
        let mut properties = Properties::new();
        properties.insert("allowOneCharVarInForLoop", "true");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // Single-char allowed in for loop
    }
}
