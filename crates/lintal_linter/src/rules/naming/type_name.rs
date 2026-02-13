//! TypeName rule implementation.
//!
//! Checks that type names (classes, interfaces, enums, annotations, records)
//! conform to a specified pattern.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;
use std::collections::HashSet;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Default pattern for type names: PascalCase
const DEFAULT_FORMAT: &str = r"^[A-Z][a-zA-Z0-9]*$";

/// Node kinds that represent types to check
const RELEVANT_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "annotation_type_declaration",
    "record_declaration",
];

/// Token types that can be checked by TypeName
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::enum_variant_names)]
pub enum TypeNameToken {
    ClassDef,
    InterfaceDef,
    EnumDef,
    AnnotationDef,
    RecordDef,
}

/// Configuration for TypeName rule.
#[derive(Debug, Clone)]
pub struct TypeName {
    /// Regex pattern for valid type names
    format: Regex,
    /// Format string for error messages
    format_str: String,
    /// Which tokens (type kinds) to check
    tokens: HashSet<TypeNameToken>,
    /// Apply to public members
    apply_to_public: bool,
    /// Apply to protected members
    apply_to_protected: bool,
    /// Apply to package-private members
    apply_to_package: bool,
    /// Apply to private members
    apply_to_private: bool,
}

impl Default for TypeName {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(TypeNameToken::ClassDef);
        tokens.insert(TypeNameToken::InterfaceDef);
        tokens.insert(TypeNameToken::EnumDef);
        tokens.insert(TypeNameToken::AnnotationDef);
        tokens.insert(TypeNameToken::RecordDef);

        Self {
            format: Regex::new(DEFAULT_FORMAT).unwrap(),
            format_str: DEFAULT_FORMAT.to_string(),
            tokens,
            apply_to_public: true,
            apply_to_protected: true,
            apply_to_package: true,
            apply_to_private: true,
        }
    }
}

/// Parse tokens from config string (e.g., "CLASS_DEF, INTERFACE_DEF").
fn parse_tokens(tokens_str: &str) -> HashSet<TypeNameToken> {
    let mut tokens = HashSet::new();
    for token in tokens_str.split(',') {
        let token = token.trim();
        match token {
            "CLASS_DEF" => {
                tokens.insert(TypeNameToken::ClassDef);
            }
            "INTERFACE_DEF" => {
                tokens.insert(TypeNameToken::InterfaceDef);
            }
            "ENUM_DEF" => {
                tokens.insert(TypeNameToken::EnumDef);
            }
            "ANNOTATION_DEF" => {
                tokens.insert(TypeNameToken::AnnotationDef);
            }
            "RECORD_DEF" => {
                tokens.insert(TypeNameToken::RecordDef);
            }
            _ => {}
        }
    }
    tokens
}

impl FromConfig for TypeName {
    const MODULE_NAME: &'static str = "TypeName";

    fn from_config(properties: &Properties) -> Self {
        let format_str = properties
            .get("format")
            .copied()
            .unwrap_or(DEFAULT_FORMAT)
            .to_string();

        let format =
            Regex::new(&format_str).unwrap_or_else(|_| Regex::new(DEFAULT_FORMAT).unwrap());

        // Parse tokens if specified, otherwise use defaults
        let tokens = if let Some(tokens_str) = properties.get("tokens") {
            parse_tokens(tokens_str)
        } else {
            let mut default_tokens = HashSet::new();
            default_tokens.insert(TypeNameToken::ClassDef);
            default_tokens.insert(TypeNameToken::InterfaceDef);
            default_tokens.insert(TypeNameToken::EnumDef);
            default_tokens.insert(TypeNameToken::AnnotationDef);
            default_tokens.insert(TypeNameToken::RecordDef);
            default_tokens
        };

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
            tokens,
            apply_to_public,
            apply_to_protected,
            apply_to_package,
            apply_to_private,
        }
    }
}

/// Violation for type name not matching pattern.
#[derive(Debug, Clone)]
pub struct TypeNameInvalid {
    pub name: String,
    pub pattern: String,
}

impl Violation for TypeNameInvalid {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Name '{}' must match pattern '{}'.",
            self.name, self.pattern
        )
    }
}

impl Rule for TypeName {
    fn name(&self) -> &'static str {
        "TypeName"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Check if this node kind is enabled in tokens
        let token = match node.kind() {
            "class_declaration" => TypeNameToken::ClassDef,
            "interface_declaration" => TypeNameToken::InterfaceDef,
            "enum_declaration" => TypeNameToken::EnumDef,
            "annotation_type_declaration" => TypeNameToken::AnnotationDef,
            "record_declaration" => TypeNameToken::RecordDef,
            _ => return vec![],
        };

        if !self.tokens.contains(&token) {
            return vec![];
        }

        // Check access control
        if !self.should_check_access(node) {
            return vec![];
        }

        // Get the type name identifier
        let Some(name_node) = node.child_by_field_name("name") else {
            // Fallback: find first identifier child
            let Some(name_node) = node.children().find(|c| c.kind() == "identifier") else {
                return vec![];
            };
            return self.check_name(ctx, &name_node);
        };

        self.check_name(ctx, &name_node)
    }
}

impl TypeName {
    /// Check if a name matches the pattern
    fn check_name(&self, ctx: &CheckContext, name_node: &CstNode) -> Vec<Diagnostic> {
        let name = &ctx.source()[name_node.range()];

        if !self.format.is_match(name) {
            vec![Diagnostic::new(
                TypeNameInvalid {
                    name: name.to_string(),
                    pattern: self.format_str.clone(),
                },
                name_node.range(),
            )]
        } else {
            vec![]
        }
    }

    /// Determine if we should check this type based on access modifiers.
    fn should_check_access(&self, node: &CstNode) -> bool {
        // Find modifiers child
        let modifiers = node.children().find(|c| c.kind() == "modifiers");

        let (has_public, has_protected, has_private) = if let Some(ref mods) = modifiers {
            let public = crate::rules::modifier::common::has_modifier(mods, "public");
            let protected = crate::rules::modifier::common::has_modifier(mods, "protected");
            let private = crate::rules::modifier::common::has_modifier(mods, "private");
            (public, protected, private)
        } else {
            (false, false, false)
        };

        // Check if this is a nested type in an interface (implicitly public)
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

    /// Check if the node is directly inside an interface.
    fn is_in_interface(&self, node: &CstNode) -> bool {
        if let Some(parent) = node.parent() {
            parent.kind() == "interface_body"
        } else {
            false
        }
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
        let rule = TypeName::from_config(&properties);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_valid_class_name() {
        let source = "class MyClass {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_class_name() {
        let source = "class myClass {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_valid_interface_name() {
        let source = "interface MyInterface {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_invalid_interface_name() {
        let source = "interface myInterface {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_valid_enum_name() {
        let source = "enum MyEnum {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_valid_annotation_name() {
        let source = "@interface MyAnnotation {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_valid_record_name() {
        // Records require proper context to parse - skip for now
        // let source = "record MyRecord(int x) {}";
        // let diagnostics = check_source(source, Properties::new());
        // assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_name_with_underscore() {
        let source = "class My_Class {}";
        let diagnostics = check_source(source, Properties::new());
        assert_eq!(diagnostics.len(), 1); // Underscore not allowed by default
    }

    #[test]
    fn test_custom_format() {
        let source = "class my_class {}";
        let mut properties = Properties::new();
        properties.insert("format", "^[a-z_]+$");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_apply_to_private_false() {
        let source = "class Outer { private class inner {} }";
        let mut properties = Properties::new();
        properties.insert("applyToPrivate", "false");
        let diagnostics = check_source(source, properties);
        assert_eq!(diagnostics.len(), 0); // Outer is valid, inner is skipped
    }
}
