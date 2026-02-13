//! Shared helpers for modifier rules.

use lintal_java_cst::CstNode;

/// JLS-recommended modifier order.
pub const JLS_MODIFIER_ORDER: &[&str] = &[
    "public",
    "protected",
    "private",
    "abstract",
    "default",
    "static",
    "sealed",
    "non-sealed",
    "final",
    "transient",
    "volatile",
    "synchronized",
    "native",
    "strictfp",
];

/// Get the index of a modifier in JLS order, or None if not found.
pub fn jls_order_index(modifier: &str) -> Option<usize> {
    JLS_MODIFIER_ORDER.iter().position(|&m| m == modifier)
}

/// Resolve the actual modifier keyword kind from a child of a `modifiers` node.
///
/// In tree-sitter-java-orchard, access modifiers are wrapped in a `visibility` node
/// and other modifiers in a `modifier` node. This function unwraps those wrappers
/// to return the actual keyword kind (e.g. "public", "static", "final").
pub fn resolve_modifier_kind<'a>(node: &CstNode<'a>) -> &'a str {
    match node.kind() {
        "visibility" | "modifier" => node
            .children()
            .next()
            .map_or(node.kind(), |child| child.kind()),
        other => other,
    }
}

/// Find the actual keyword node for a modifier child of a `modifiers` node.
///
/// If the child is a `visibility` or `modifier` wrapper, returns the inner keyword node.
/// Otherwise returns the node itself.
pub fn resolve_modifier_node<'a>(node: CstNode<'a>) -> CstNode<'a> {
    match node.kind() {
        "visibility" | "modifier" => node.children().next().unwrap_or(node),
        _ => node,
    }
}

/// Check if a modifiers node contains a specific modifier.
pub fn has_modifier(modifiers: &CstNode, modifier_kind: &str) -> bool {
    modifiers
        .children()
        .any(|child| resolve_modifier_kind(&child) == modifier_kind)
}

/// Find a specific modifier keyword node within a modifiers node.
pub fn find_modifier<'a>(modifiers: &CstNode<'a>, modifier_kind: &str) -> Option<CstNode<'a>> {
    modifiers.children().find_map(|child| {
        if resolve_modifier_kind(&child) == modifier_kind {
            Some(resolve_modifier_node(child))
        } else {
            None
        }
    })
}

/// Check if we're inside an interface definition.
pub fn is_in_interface(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "interface_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an annotation definition.
pub fn is_in_annotation(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "annotation_type_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Check if the containing class is final.
pub fn is_in_final_class(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "class_declaration" {
            if let Some(modifiers) = parent.child_by_field_name("modifiers") {
                return has_modifier(&modifiers, "final");
            }
            return false;
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an anonymous class.
pub fn is_in_anonymous_class(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "object_creation_expression" {
            // Check if it has a class body (anonymous class)
            return parent.child_by_field_name("body").is_some();
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an enum definition.
pub fn is_in_enum(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "enum_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}
