//! RedundantModifier rule implementation.
//!
//! Checks for redundant modifiers in various contexts.
//! This is a port of the checkstyle RedundantModifierCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for RedundantModifier rule.
#[derive(Debug, Clone)]
pub struct RedundantModifier {
    /// JDK version for version-specific checks (e.g., strictfp in JDK 17+)
    jdk_version: u32,
}

const RELEVANT_KINDS: &[&str] = &[
    "interface_declaration",
    "annotation_type_declaration",
    "field_declaration",
    "constant_declaration",
    "method_declaration",
    "annotation_type_element_declaration",
    "class_declaration",
    "enum_declaration",
    "record_declaration",
    "constructor_declaration",
    "try_with_resources_statement",
    "formal_parameter",
    "local_variable_declaration",
    "instanceof_expression",
    "catch_formal_parameter",
    "switch_rule",
];

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
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Redundant '{}' modifier.", self.modifier)
    }
}

impl Rule for RedundantModifier {
    fn name(&self) -> &'static str {
        "RedundantModifier"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Check for redundant strictfp modifier (JDK 17+)
        let mut diagnostics = if self.jdk_version >= 17 {
            self.check_strictfp_modifier(ctx, node)
        } else {
            vec![]
        };

        // Add node-specific checks
        diagnostics.extend(match node.kind() {
            "interface_declaration" | "annotation_type_declaration" => {
                self.check_interface_modifiers(ctx, node)
            }
            "field_declaration" | "constant_declaration" => self.check_field_modifiers(ctx, node),
            "method_declaration" | "annotation_type_element_declaration" => {
                self.check_method_modifiers(ctx, node)
            }
            "class_declaration" => self.check_class_modifiers(ctx, node),
            "enum_declaration" => self.check_enum_modifiers(ctx, node),
            "record_declaration" => self.check_record_modifiers(ctx, node),
            "constructor_declaration" => self.check_constructor_modifiers(ctx, node),
            "try_with_resources_statement" => self.check_try_with_resources(ctx, node),
            "formal_parameter" => self.check_parameter_modifiers(ctx, node),
            "local_variable_declaration" => self.check_local_variable_modifiers(ctx, node),
            "instanceof_expression" => self.check_instanceof_modifiers(ctx, node),
            "catch_formal_parameter" => self.check_catch_parameter_modifiers(ctx, node),
            "switch_rule" => self.check_switch_rule_modifiers(ctx, node),
            _ => vec![],
        });

        diagnostics
    }
}

impl RedundantModifier {
    /// Check for redundant modifiers on interface/annotation declarations.
    fn check_interface_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

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
        let modifiers = node.children().find(|child| child.kind() == "modifiers");

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

        // Find modifiers - it's the first child with kind "modifiers"
        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        // Check if the method is in an interface or annotation
        if self.is_in_interface_or_annotation(node) {
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
            return diagnostics;
        }

        // Task 7: Check for redundant 'final' modifiers on methods
        if let Some(modifiers) = modifiers
            && let Some(final_mod) = self.find_modifier(&modifiers, "final")
        {
            // Exception: @SafeVarargs methods can have final modifier
            if self.has_safe_varargs_annotation(node) {
                return diagnostics;
            }

            let has_private = self.find_modifier(&modifiers, "private").is_some();
            let has_static = self.find_modifier(&modifiers, "static").is_some();
            let in_final_class = self.is_in_final_class(node);
            let in_anonymous_class = self.is_in_anonymous_class(node);
            let in_enum = self.is_in_enum(node);

            // Check if final is redundant in any of these cases:
            // 1. Final on private method (cannot be overridden)
            // 2. Final on method in final class
            // 3. Final on method in anonymous class
            // 4. Final on static method in enum (static methods cannot be overridden)
            let is_final_redundant =
                has_private || in_final_class || in_anonymous_class || (in_enum && has_static);

            if is_final_redundant {
                diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
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

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

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

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Nested enums are implicitly static
            if is_nested && let Some(static_mod) = self.find_modifier(&modifiers, "static") {
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

    /// Check for redundant modifiers on constructor declarations.
    fn check_constructor_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Check if constructor is in an enum - all visibility modifiers are redundant
            if self.is_in_enum(node) {
                // Any visibility modifier on enum constructor is redundant
                for visibility in ["public", "protected", "private"] {
                    if let Some(vis_mod) = self.find_modifier(&modifiers, visibility) {
                        diagnostics.push(self.create_diagnostic(ctx, &vis_mod, visibility));
                    }
                }
            } else {
                // Check if the constructor is public in a non-public class
                if let Some(public_mod) = self.find_modifier(&modifiers, "public")
                    && !self.is_constructor_in_public_accessible_class(node)
                {
                    diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
                }
            }
        }

        diagnostics
    }

    /// Find a modifier by name in a modifiers node.
    fn find_modifier<'a>(
        &self,
        modifiers: &CstNode<'a>,
        modifier_name: &str,
    ) -> Option<CstNode<'a>> {
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
                | "annotation_type_declaration"
                | "record_declaration" => return true,
                _ => current = parent.parent(),
            }
        }
        false
    }

    /// Check if a node is inside an enum definition.
    fn is_in_enum(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "enum_declaration" {
                return true;
            }
            current = parent.parent();
        }
        false
    }

    /// Check if a constructor is in a class that is publicly accessible or protected.
    /// Per checkstyle: public modifier on constructor is redundant if the class is
    /// neither public (accessible from public scope) nor protected.
    fn is_constructor_in_public_accessible_class(&self, constructor: &CstNode) -> bool {
        // Find the enclosing class declaration
        if let Some(class_def) = self.find_enclosing_class(constructor) {
            // Check if class is protected or public-accessible
            self.is_class_protected(&class_def) || self.is_class_public(&class_def)
        } else {
            false
        }
    }

    /// Find the enclosing class declaration for a node.
    fn find_enclosing_class<'a>(&self, node: &CstNode<'a>) -> Option<CstNode<'a>> {
        let mut current = node.parent();
        while let Some(parent) = current {
            if matches!(parent.kind(), "class_declaration" | "record_declaration") {
                return Some(parent);
            }
            current = parent.parent();
        }
        None
    }

    /// Check if a class is protected.
    fn is_class_protected(&self, class_def: &CstNode) -> bool {
        class_def
            .children()
            .find(|child| child.kind() == "modifiers")
            .map(|modifiers| self.find_modifier(&modifiers, "protected").is_some())
            .unwrap_or(false)
    }

    /// Check if a class is accessible from public scope.
    /// A class is public-accessible if:
    /// - It's a top-level class with public modifier, OR
    /// - It's a nested class with public modifier AND its parent class is also public-accessible, OR
    /// - It's nested directly inside an interface or annotation (implicitly public)
    fn is_class_public(&self, class_def: &CstNode) -> bool {
        // Check if this class is nested inside an interface/annotation (implicitly public)
        if self.is_direct_child_of_interface_or_annotation(class_def) {
            return true;
        }

        // Check if this class has public modifier
        let has_public = class_def
            .children()
            .find(|child| child.kind() == "modifiers")
            .map(|modifiers| self.find_modifier(&modifiers, "public").is_some())
            .unwrap_or(false);

        if !has_public {
            return false;
        }

        // Check if this is a top-level class (parent is program)
        if let Some(parent) = class_def.parent() {
            if parent.kind() == "program" {
                return true; // Top-level public class
            }

            // Check if it's a nested class - find parent class
            if parent.kind() == "class_body"
                && let Some(parent_class) = parent.parent()
                && parent_class.kind() == "class_declaration"
            {
                // Recursively check if parent class is public
                return self.is_class_public(&parent_class);
            }
        }

        // If we can't determine parent, conservatively assume not accessible
        // to avoid false negatives (not flagging a truly redundant modifier)
        false
    }

    /// Check if a node is in a final class.
    fn is_in_final_class(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            // Stop at enum boundary - enum methods can be overridden by enum constant bodies
            if parent.kind() == "enum_declaration" {
                return false;
            }
            if parent.kind() == "class_declaration" {
                if let Some(modifiers) = parent.children().find(|c| c.kind() == "modifiers") {
                    return self.find_modifier(&modifiers, "final").is_some();
                }
                return false;
            }
            current = parent.parent();
        }
        false
    }

    /// Check if a node is in an anonymous class.
    fn is_in_anonymous_class(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            // Anonymous class is an object_creation_expression with a class_body
            if parent.kind() == "object_creation_expression" {
                return parent.children().any(|c| c.kind() == "class_body");
            }
            // Enum constants with bodies are also anonymous classes
            if parent.kind() == "enum_constant" {
                return parent.children().any(|c| c.kind() == "class_body");
            }
            // Stop at class/enum boundaries (but not at enum_constant)
            if matches!(parent.kind(), "class_declaration" | "enum_declaration") {
                return false;
            }
            current = parent.parent();
        }
        false
    }

    /// Check if a method has @SafeVarargs annotation.
    fn has_safe_varargs_annotation(&self, method_node: &CstNode) -> bool {
        // Annotations are inside the modifiers node
        if let Some(modifiers) = method_node.children().find(|c| c.kind() == "modifiers") {
            for child in modifiers.children() {
                if matches!(child.kind(), "marker_annotation" | "annotation") {
                    // Get the annotation text (e.g., "@SafeVarargs")
                    let text = child.text();
                    // Check if it contains "SafeVarargs"
                    if text.contains("SafeVarargs") {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check for redundant final modifier on try-with-resources variables.
    fn check_try_with_resources(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Get the resources field which contains the resource specification
        if let Some(resources) = node.child_by_field_name("resources") {
            // The resources node is a resource_specification which contains resource nodes
            for resource in resources.children() {
                // Each resource can be either:
                // - A resource (local variable declaration with optional final)
                // - An identifier (reference to an existing variable)
                if resource.kind() == "resource" {
                    // Check if the resource has modifiers
                    if let Some(modifiers) = resource.children().find(|c| c.kind() == "modifiers") {
                        // Check for final modifier
                        if let Some(final_mod) = self.find_modifier(&modifiers, "final") {
                            diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Check for redundant final modifier on parameters of abstract/interface/native methods.
    /// Also checks for unnamed lambda parameters in JDK 22+.
    fn check_parameter_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Task 10: Check for unnamed lambda parameters in JDK 22+
        if self.jdk_version >= 22
            && self.is_in_lambda(node)
            && self.is_unnamed_variable(node)
            && let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
            && let Some(final_mod) = self.find_modifier(&modifiers, "final")
        {
            diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
        }

        // Check if this parameter belongs to a method that cannot have a body
        // (abstract, interface, or native)
        if let Some(method) = self.find_parent_method(node) {
            // Check if it's an interface method (always abstract unless default/static)
            let is_interface_method = self.is_in_interface_or_annotation(&method);

            // Check if it's an abstract method
            let is_abstract =
                if let Some(modifiers) = method.children().find(|c| c.kind() == "modifiers") {
                    self.find_modifier(&modifiers, "abstract").is_some()
                        || self.find_modifier(&modifiers, "native").is_some()
                } else {
                    false
                };

            // For interface methods, check if they have default or static (which means they have body)
            let has_body = if is_interface_method {
                if let Some(modifiers) = method.children().find(|c| c.kind() == "modifiers") {
                    self.find_modifier(&modifiers, "default").is_some()
                        || self.find_modifier(&modifiers, "static").is_some()
                } else {
                    false
                }
            } else {
                false
            };

            // Check if final is redundant (abstract/interface/native methods without body)
            if (is_abstract || is_interface_method) && !has_body {
                // Check if the parameter has a final modifier
                if let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
                    && let Some(final_mod) = self.find_modifier(&modifiers, "final")
                {
                    diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
                }
            }
        }

        diagnostics
    }

    /// Find the parent method declaration for a parameter.
    fn find_parent_method<'a>(&self, node: &CstNode<'a>) -> Option<CstNode<'a>> {
        let mut current = node.parent();
        while let Some(parent) = current {
            if matches!(
                parent.kind(),
                "method_declaration" | "annotation_type_element_declaration"
            ) {
                return Some(parent);
            }
            current = parent.parent();
        }
        None
    }

    /// Create a diagnostic for a redundant modifier.
    fn create_diagnostic(&self, ctx: &CheckContext, node: &CstNode, modifier: &str) -> Diagnostic {
        // Calculate range to delete: the modifier keyword plus any trailing whitespace
        let modifier_range = node.range();
        let source = ctx.source();

        // Find trailing whitespace after the modifier
        let mut delete_end = modifier_range.end();
        let source_bytes = source.as_bytes();
        let start_idx = usize::from(modifier_range.end());

        // Skip whitespace characters after the modifier (but not newlines)
        for (offset, &byte) in source_bytes[start_idx..].iter().enumerate() {
            let ch = byte as char;
            if ch == ' ' || ch == '\t' {
                delete_end = lintal_text_size::TextSize::new((start_idx + offset + 1) as u32);
            } else {
                break;
            }
        }

        let delete_range = TextRange::new(modifier_range.start(), delete_end);

        Diagnostic::new(
            RedundantModifierViolation {
                modifier: modifier.to_string(),
            },
            node.range(),
        )
        .with_fix(Fix::safe_edit(Edit::range_deletion(delete_range)))
    }

    /// Check for redundant modifiers on record declarations.
    fn check_record_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers {
            // Records are implicitly final
            if let Some(final_mod) = self.find_modifier(&modifiers, "final") {
                diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
            }

            // Nested records are implicitly static
            if self.is_nested(node)
                && let Some(static_mod) = self.find_modifier(&modifiers, "static")
            {
                diagnostics.push(self.create_diagnostic(ctx, &static_mod, "static"));
            }

            // Records inside interfaces are also implicitly public
            if self.is_direct_child_of_interface_or_annotation(node)
                && let Some(public_mod) = self.find_modifier(&modifiers, "public")
            {
                diagnostics.push(self.create_diagnostic(ctx, &public_mod, "public"));
            }
        }

        diagnostics
    }

    /// Check for redundant strictfp modifier (JDK 17+).
    /// In JDK 17+, strictfp is redundant on all declarations.
    fn check_strictfp_modifier(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check nodes that can have modifiers
        let can_have_modifiers = matches!(
            node.kind(),
            "class_declaration"
                | "interface_declaration"
                | "annotation_type_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "method_declaration"
                | "annotation_type_element_declaration"
        );

        if !can_have_modifiers {
            return diagnostics;
        }

        let modifiers = node.children().find(|child| child.kind() == "modifiers");

        if let Some(modifiers) = modifiers
            && let Some(strictfp_mod) = self.find_modifier(&modifiers, "strictfp")
        {
            diagnostics.push(self.create_diagnostic(ctx, &strictfp_mod, "strictfp"));
        }

        diagnostics
    }

    /// Check for redundant final modifier on local variable declarations.
    /// In JDK 22+, final on unnamed variables (_) is redundant.
    fn check_local_variable_modifiers(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check in JDK 22+
        if self.jdk_version < 22 {
            return diagnostics;
        }

        // Check if variable is named "_"
        if !self.is_unnamed_variable(node) {
            return diagnostics;
        }

        // Check for final modifier
        if let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
            && let Some(final_mod) = self.find_modifier(&modifiers, "final")
        {
            diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
        }

        diagnostics
    }

    /// Check for redundant final modifier on instanceof expressions with pattern variables.
    /// In JDK 22+, final on unnamed pattern variables (_) is redundant.
    fn check_instanceof_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check in JDK 22+
        if self.jdk_version < 22 {
            return diagnostics;
        }

        // Check if the instanceof has a final modifier
        let has_final = node.children().any(|c| c.kind() == "final");
        if !has_final {
            return diagnostics;
        }

        // Check if the pattern variable is named "_"
        let has_underscore = node
            .children()
            .any(|c| c.kind() == "identifier" && c.text().trim() == "_");

        if has_underscore {
            // Find the final modifier node
            if let Some(final_mod) = node.children().find(|c| c.kind() == "final") {
                diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
            }
        }

        diagnostics
    }

    /// Check for redundant final modifier on catch parameters.
    /// In JDK 22+, final on unnamed catch parameters (_) is redundant.
    fn check_catch_parameter_modifiers(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check in JDK 22+
        if self.jdk_version < 22 {
            return diagnostics;
        }

        // Check if parameter is named "_"
        if !self.is_unnamed_variable(node) {
            return diagnostics;
        }

        // Check for final modifier
        if let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
            && let Some(final_mod) = self.find_modifier(&modifiers, "final")
        {
            diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
        }

        diagnostics
    }

    /// Check for redundant final modifier on switch rule patterns.
    /// In JDK 22+, final on unnamed pattern variables (_) in switch cases is redundant.
    fn check_switch_rule_modifiers(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check in JDK 22+
        if self.jdk_version < 22 {
            return diagnostics;
        }

        // Check if there's an ERROR or identifier node with text "_" (tree-sitter limitation)
        let has_underscore = node
            .children()
            .any(|c| (c.kind() == "ERROR" || c.kind() == "identifier") && c.text().trim() == "_");

        if !has_underscore {
            return diagnostics;
        }

        // Look for final modifier in the switch_label child
        if let Some(switch_label) = node.children().find(|c| c.kind() == "switch_label") {
            // Check if the label contains a pattern with final
            if let Some(pattern) = switch_label.children().find(|c| c.kind() == "pattern") {
                // Check for final modifier in the pattern
                if let Some(final_mod) = pattern.children().find(|c| c.kind() == "final") {
                    diagnostics.push(self.create_diagnostic(ctx, &final_mod, "final"));
                }
            }
        }

        diagnostics
    }

    /// Check if a variable/parameter node is an unnamed variable (_).
    fn is_unnamed_variable(&self, node: &CstNode) -> bool {
        // Look for underscore_pattern or identifier with text "_"
        for child in node.children() {
            if child.kind() == "underscore_pattern" {
                return true;
            }
            if child.kind() == "identifier" {
                let text = child.text().trim();
                if text == "_" {
                    return true;
                }
            }
            // For variable_declarator, check its first child
            if child.kind() == "variable_declarator" {
                for grandchild in child.children() {
                    if grandchild.kind() == "underscore_pattern" {
                        return true;
                    }
                    if grandchild.kind() == "identifier" && grandchild.text().trim() == "_" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if a node is inside a lambda expression.
    fn is_in_lambda(&self, node: &CstNode) -> bool {
        let mut current = node.parent();
        while let Some(parent) = current {
            if parent.kind() == "lambda_expression" {
                return true;
            }
            // Stop at method/class boundaries
            if matches!(
                parent.kind(),
                "method_declaration" | "constructor_declaration" | "class_declaration"
            ) {
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
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'public' modifier")
        );
    }

    #[test]
    fn test_redundant_abstract_in_interface() {
        let source = "interface Foo { abstract void test(); }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'abstract' modifier")
        );
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
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'static' modifier")
        );
    }

    #[test]
    fn test_redundant_public_class_in_interface() {
        let source = "interface Foo { public class Bar {} }";
        let diagnostics = check_source(source, None);
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'public' modifier")
        );
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
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'static' modifier")
        );
    }

    #[test]
    fn test_annotation_fields() {
        let source = "@interface Annotation { public String s1 = \"\"; public String blah(); }";
        let diagnostics = check_source(source, None);
        // Should find 2 violations: public on field and public on method
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'public' modifier")
        );
        assert!(
            diagnostics[1]
                .kind
                .body
                .contains("Redundant 'public' modifier")
        );
    }

    #[test]
    fn test_safe_varargs_allows_final() {
        let source = "class Test { @SafeVarargs private final void foo(int... k) {} }";
        let diagnostics = check_source(source, None);
        // @SafeVarargs methods should allow final modifier
        assert_eq!(
            diagnostics.len(),
            0,
            "Expected 0 violations for @SafeVarargs method"
        );
    }

    #[test]
    fn test_private_final_without_safe_varargs() {
        let source = "class Test { private final void foo() {} }";
        let diagnostics = check_source(source, None);
        // Without @SafeVarargs, final on private method is redundant
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'final' modifier")
        );
    }

    #[test]
    fn test_record_in_record() {
        let source = "record Outer() { static record Nested() {} }";
        let diagnostics = check_source(source, None);
        // Should find static modifier on nested record as redundant
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'static' modifier")
        );
    }

    #[test]
    fn test_strictfp_jdk17() {
        let source = "strictfp class Test {}";
        let diagnostics = check_source(source, Some(17));
        // strictfp is redundant in JDK 17+
        assert_eq!(diagnostics.len(), 1);
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'strictfp' modifier")
        );
    }

    #[test]
    fn test_strictfp_jdk16() {
        let source = "strictfp class Test {}";
        let diagnostics = check_source(source, Some(16));
        // strictfp is NOT redundant before JDK 17
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_unnamed_local_variable_jdk22() {
        let source = r#"
class Test {
    void m() {
        final int _ = 5;
    }
}
"#;
        let diagnostics = check_source(source, Some(22));
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for unnamed local variable"
        );
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'final' modifier")
        );
    }

    #[test]
    fn test_unnamed_pattern_variable_jdk22() {
        let source = r#"
class Test {
    void m(Object o) {
        if (o instanceof final String _) { }
    }
}
"#;
        let diagnostics = check_source(source, Some(22));
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for unnamed pattern variable"
        );
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'final' modifier")
        );
    }

    #[test]
    fn test_unnamed_catch_parameter_jdk22() {
        let source = r#"
class Test {
    void m() {
        try {
        } catch (final Exception _) {
        }
    }
}
"#;
        let diagnostics = check_source(source, Some(22));
        assert_eq!(
            diagnostics.len(),
            1,
            "Expected 1 violation for unnamed catch parameter"
        );
        assert!(
            diagnostics[0]
                .kind
                .body
                .contains("Redundant 'final' modifier")
        );
    }

    #[test]
    fn test_unnamed_lambda_parameter_jdk22() {
        let source = r#"
import java.util.function.BiFunction;
class Test {
    void m() {
        BiFunction<Integer, Integer, Integer> f = (final Integer _, final Integer _) -> {
            return 5;
        };
    }
}
"#;
        let diagnostics = check_source(source, Some(22));
        assert_eq!(
            diagnostics.len(),
            2,
            "Expected 2 violations for unnamed lambda parameters"
        );
        for diagnostic in diagnostics {
            assert!(diagnostic.kind.body.contains("Redundant 'final' modifier"));
        }
    }
}
