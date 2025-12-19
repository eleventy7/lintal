//! Shared helpers for modifier rules.

use lintal_java_cst::CstNode;

/// JLS-recommended modifier order.
pub const JLS_MODIFIER_ORDER: &[&str] = &[
    "public", "protected", "private", "abstract", "default", "static",
    "sealed", "non-sealed", "final", "transient", "volatile",
    "synchronized", "native", "strictfp",
];

/// Get the index of a modifier in JLS order, or None if not found.
pub fn jls_order_index(modifier: &str) -> Option<usize> {
    JLS_MODIFIER_ORDER.iter().position(|&m| m == modifier)
}

/// Check if a modifiers node contains a specific modifier.
pub fn has_modifier(modifiers: &CstNode, modifier_kind: &str) -> bool {
    modifiers.children().any(|child| child.kind() == modifier_kind)
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
