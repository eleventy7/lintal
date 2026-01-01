//! FinalParameters rule implementation.
//!
//! Checks that parameters for methods, constructors, catch and for-each blocks are final.
//! This is a port of the checkstyle FinalParametersCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use std::collections::HashSet;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for FinalParameters rule.
#[derive(Debug, Clone)]
pub struct FinalParameters {
    /// Which tokens to check (METHOD_DEF, CTOR_DEF, LITERAL_CATCH, FOR_EACH_CLAUSE)
    tokens: HashSet<FinalParametersToken>,
    /// Ignore primitive types as parameters
    ignore_primitive_types: bool,
    /// Ignore unnamed parameters (single underscore)
    ignore_unnamed_parameters: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "catch_clause",
    "enhanced_for_statement",
];

/// Token types that can be checked by FinalParameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinalParametersToken {
    MethodDef,
    CtorDef,
    LiteralCatch,
    ForEachClause,
}

impl Default for FinalParameters {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(FinalParametersToken::MethodDef);
        tokens.insert(FinalParametersToken::CtorDef);

        Self {
            tokens,
            ignore_primitive_types: false,
            ignore_unnamed_parameters: true,
        }
    }
}

impl FromConfig for FinalParameters {
    const MODULE_NAME: &'static str = "FinalParameters";

    fn from_config(properties: &Properties) -> Self {
        let tokens = if let Some(tokens_str) = properties.get("tokens") {
            parse_tokens(tokens_str)
        } else {
            let mut default_tokens = HashSet::new();
            default_tokens.insert(FinalParametersToken::MethodDef);
            default_tokens.insert(FinalParametersToken::CtorDef);
            default_tokens
        };

        let ignore_primitive_types = properties
            .get("ignorePrimitiveTypes")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let ignore_unnamed_parameters = properties
            .get("ignoreUnnamedParameters")
            .map(|v| *v == "true")
            .unwrap_or(true);

        Self {
            tokens,
            ignore_primitive_types,
            ignore_unnamed_parameters,
        }
    }
}

/// Parse tokens from config string.
fn parse_tokens(tokens_str: &str) -> HashSet<FinalParametersToken> {
    let mut tokens = HashSet::new();
    for token in tokens_str.split(',') {
        let token = token.trim();
        match token {
            "METHOD_DEF" => {
                tokens.insert(FinalParametersToken::MethodDef);
            }
            "CTOR_DEF" => {
                tokens.insert(FinalParametersToken::CtorDef);
            }
            "LITERAL_CATCH" => {
                tokens.insert(FinalParametersToken::LiteralCatch);
            }
            "FOR_EACH_CLAUSE" => {
                tokens.insert(FinalParametersToken::ForEachClause);
            }
            _ => {}
        }
    }
    tokens
}

/// Violation for missing final modifier on parameter.
#[derive(Debug, Clone)]
pub struct ParameterShouldBeFinal {
    pub param_name: String,
    pub column: usize,
}

impl Violation for ParameterShouldBeFinal {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Parameter {} should be final.", self.param_name)
    }
}

impl Rule for FinalParameters {
    fn name(&self) -> &'static str {
        "FinalParameters"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "method_declaration" => {
                if self.tokens.contains(&FinalParametersToken::MethodDef) {
                    self.check_method(ctx, node)
                } else {
                    vec![]
                }
            }
            "constructor_declaration" => {
                if self.tokens.contains(&FinalParametersToken::CtorDef) {
                    self.check_constructor(ctx, node)
                } else {
                    vec![]
                }
            }
            "catch_clause" => {
                if self.tokens.contains(&FinalParametersToken::LiteralCatch) {
                    self.check_catch(ctx, node)
                } else {
                    vec![]
                }
            }
            "enhanced_for_statement" => {
                if self.tokens.contains(&FinalParametersToken::ForEachClause) {
                    self.check_for_each(ctx, node)
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

impl FinalParameters {
    /// Check method parameters.
    fn check_method(&self, ctx: &CheckContext, method: &CstNode) -> Vec<Diagnostic> {
        // Skip if there is no method body (abstract/interface/native)
        if method.child_by_field_name("body").is_none() {
            return vec![];
        }

        let Some(parameters) = method.child_by_field_name("parameters") else {
            return vec![];
        };

        let mut diagnostics = vec![];
        for child in parameters.children() {
            if matches!(child.kind(), "formal_parameter" | "spread_parameter")
                && let Some(diagnostic) = self.check_param(ctx, &child)
            {
                diagnostics.push(diagnostic);
            }
        }
        diagnostics
    }

    /// Check constructor parameters.
    fn check_constructor(&self, ctx: &CheckContext, constructor: &CstNode) -> Vec<Diagnostic> {
        let Some(parameters) = constructor.child_by_field_name("parameters") else {
            return vec![];
        };

        let mut diagnostics = vec![];
        for child in parameters.children() {
            if matches!(child.kind(), "formal_parameter" | "spread_parameter")
                && let Some(diagnostic) = self.check_param(ctx, &child)
            {
                diagnostics.push(diagnostic);
            }
        }
        diagnostics
    }

    /// Check catch clause parameter.
    fn check_catch(&self, ctx: &CheckContext, catch_clause: &CstNode) -> Vec<Diagnostic> {
        // Find catch_formal_parameter child
        for child in catch_clause.children() {
            if child.kind() == "catch_formal_parameter" {
                if let Some(diagnostic) = self.check_param(ctx, &child) {
                    return vec![diagnostic];
                }
                return vec![];
            }
        }
        vec![]
    }

    /// Check for-each statement parameter.
    fn check_for_each(&self, ctx: &CheckContext, for_each: &CstNode) -> Vec<Diagnostic> {
        // In tree-sitter Java, enhanced_for_statement has children:
        // for, (, [modifiers], type, identifier/underscore_pattern, :, expression, ), block
        // We need to check if modifiers contains "final"

        // Check if there's a modifiers child with final
        let mut has_final = false;
        let mut identifier_node = None;
        let mut type_node = None;
        let mut first_reportable_node = None;

        for child in for_each.children() {
            match child.kind() {
                "modifiers" => {
                    if first_reportable_node.is_none() {
                        first_reportable_node = Some(child);
                    }
                    if self.has_final_modifier(&child) {
                        has_final = true;
                    }
                }
                "type_identifier"
                | "generic_type"
                | "array_type"
                | "integral_type"
                | "floating_point_type"
                | "boolean_type" => {
                    if first_reportable_node.is_none() {
                        first_reportable_node = Some(child);
                    }
                    type_node = Some(child);
                }
                "identifier" | "underscore_pattern" => {
                    // The first identifier/underscore_pattern after the type is the loop variable
                    // We want to stop at the first one, not get the expression identifier
                    if identifier_node.is_none() {
                        identifier_node = Some(child);
                    }
                }
                _ => {}
            }
        }

        if has_final {
            return vec![];
        }

        // Get parameter name
        let Some(name_node) = identifier_node else {
            return vec![];
        };
        let param_name = &ctx.source()[name_node.range()];

        // Skip if unnamed parameter
        if self.ignore_unnamed_parameters && param_name == "_" {
            return vec![];
        }

        // Check if primitive type
        if let Some(type_node) = type_node
            && self.ignore_primitive_types
            && self.is_primitive_type(ctx, &type_node)
        {
            return vec![];
        }

        // Find the first node for reporting
        let first_node = first_reportable_node.unwrap_or(name_node);
        let first_leaf = self.get_first_leaf_node(&first_node);

        // Calculate where to insert "final "
        // Look for existing modifiers or insert before the type
        let insert_position = for_each
            .children()
            .find(|child| child.kind() == "modifiers")
            .map(|modifiers| modifiers.range().end())
            .or_else(|| type_node.map(|t| t.range().start()))
            .unwrap_or_else(|| first_leaf.range().start());

        vec![
            Diagnostic::new(
                ParameterShouldBeFinal {
                    param_name: param_name.to_string(),
                    column: Self::get_column(ctx, &first_leaf),
                },
                first_leaf.range(),
            )
            .with_fix(Fix::safe_edit(Edit::insertion(
                "final ".to_string(),
                insert_position,
            ))),
        ]
    }

    /// Check if a parameter should have final modifier.
    fn check_param(&self, ctx: &CheckContext, param: &CstNode) -> Option<Diagnostic> {
        // Check if already has final modifier
        // In tree-sitter Java, formal_parameter has a child with kind "modifiers"
        // We need to find it and check if it contains "final"
        for child in param.children() {
            if child.kind() == "modifiers" {
                // Found modifiers child, check if it contains final
                if self.has_final_modifier(&child) {
                    return None;
                }
                break; // Only one modifiers child
            }
        }

        // Check if this is a receiver parameter
        if self.is_receiver_parameter(ctx, param) {
            return None;
        }

        // Get parameter name
        let param_name_node = param.child_by_field_name("name")?;
        let param_name = &ctx.source()[param_name_node.range()];

        // Check if unnamed parameter
        if self.ignore_unnamed_parameters && param_name == "_" {
            return None;
        }

        // Check if primitive type
        if let Some(type_node) = param.child_by_field_name("type")
            && self.ignore_primitive_types
            && self.is_primitive_type(ctx, &type_node)
        {
            return None;
        }

        // Find the first node to report on - this is the leftmost leaf node
        // which typically is the type or first annotation
        let first_node = self.get_first_leaf_node(param);

        // Calculate where to insert "final "
        // If there's a modifiers node, insert at the end of it
        // Otherwise, insert before the type
        let insert_position = param
            .children()
            .find(|child| child.kind() == "modifiers")
            .map(|modifiers| modifiers.range().end())
            .or_else(|| {
                param
                    .child_by_field_name("type")
                    .map(|type_node| type_node.range().start())
            })
            .unwrap_or_else(|| first_node.range().start());

        Some(
            Diagnostic::new(
                ParameterShouldBeFinal {
                    param_name: param_name.to_string(),
                    column: Self::get_column(ctx, &first_node),
                },
                first_node.range(),
            )
            .with_fix(Fix::safe_edit(Edit::insertion(
                "final ".to_string(),
                insert_position,
            ))),
        )
    }

    /// Check if modifiers contain final.
    fn has_final_modifier(&self, modifiers: &CstNode) -> bool {
        super::common::has_modifier(modifiers, "final")
    }

    /// Check if parameter is a receiver parameter (e.g., "Foo.this").
    fn is_receiver_parameter(&self, ctx: &CheckContext, param: &CstNode) -> bool {
        // Receiver parameters have a different structure
        // They look like: Type Identifier.this
        // Check if name contains "this"
        if let Some(name_node) = param.child_by_field_name("name") {
            let name = &ctx.source()[name_node.range()];
            if name.ends_with(".this") || name == "this" {
                return true;
            }
        }

        // Also check for receiver_parameter node type
        if param.kind() == "receiver_parameter" {
            return true;
        }

        false
    }

    /// Check if type is a primitive type.
    fn is_primitive_type(&self, ctx: &CheckContext, type_node: &CstNode) -> bool {
        // Check if this is an array type - if so, it's not primitive
        if type_node.kind() == "array_type" {
            return false;
        }

        // Get the first child which should be the actual type
        for child in type_node.children() {
            match child.kind() {
                "integral_type" | "floating_point_type" | "boolean_type" => return true,
                "array_type" => return false,
                _ => {}
            }
        }

        // Check if the type text matches primitive types
        let type_text = &ctx.source()[type_node.range()];
        matches!(
            type_text,
            "byte" | "short" | "int" | "long" | "float" | "double" | "boolean" | "char"
        )
    }

    /// Get the first leaf node (leftmost token) in the subtree.
    /// This matches checkstyle's CheckUtil.getFirstNode behavior.
    fn get_first_leaf_node<'a>(&self, node: &'a CstNode) -> CstNode<'a> {
        let mut current = *node;

        loop {
            let mut leftmost_child = None;
            let mut leftmost_start = current.range().start();

            for child in current.children() {
                if child.range().start() < leftmost_start {
                    leftmost_start = child.range().start();
                    leftmost_child = Some(child);
                }
            }

            match leftmost_child {
                Some(child) => current = child,
                None => return current,
            }
        }
    }

    /// Get column number (1-indexed) for a node.
    fn get_column(ctx: &CheckContext, node: &CstNode) -> usize {
        ctx.source_code()
            .line_column(node.range().start())
            .column
            .get()
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
        let rule = FinalParameters::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_method_with_final_param() {
        let source = "class Foo { void test(final String s) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_method_without_final_param() {
        let source = "class Foo { void test(String s) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_abstract_method_ignored() {
        let source = "abstract class Foo { abstract void test(String s); }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_interface_method_ignored() {
        let source = "interface Foo { void test(String s); }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_constructor_with_final_param() {
        let source = "class Foo { Foo(final String s) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_constructor_without_final_param() {
        let source = "class Foo { Foo(String s) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_ignore_primitive_types() {
        let source = "class Foo { void test(int i, String s) {} }";
        let mut properties = Properties::new();
        properties.insert("ignorePrimitiveTypes", "true");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 1); // Only String s should be flagged
    }

    #[test]
    fn test_ignore_unnamed_parameters() {
        let source = "class Foo { void test(String _) {} }";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0); // Default ignoreUnnamedParameters=true
    }

    #[test]
    fn test_check_unnamed_parameters() {
        let source = "class Foo { void test(String _) {} }";
        let mut properties = Properties::new();
        properties.insert("ignoreUnnamedParameters", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 1);
    }
}
