//! AvoidNestedBlocks rule implementation.
//!
//! Finds nested blocks (blocks that are used freely in the code).
//! This is a port of the checkstyle AvoidNestedBlocksCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for AvoidNestedBlocks rule.
#[derive(Debug, Clone, Default)]
pub struct AvoidNestedBlocks {
    /// Allow nested blocks if they are the only child of a switch case.
    pub allow_in_switch_case: bool,
}

const RELEVANT_KINDS: &[&str] = &["block"];

impl FromConfig for AvoidNestedBlocks {
    const MODULE_NAME: &'static str = "AvoidNestedBlocks";

    fn from_config(properties: &Properties) -> Self {
        let allow_in_switch_case = properties
            .get("allowInSwitchCase")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        Self {
            allow_in_switch_case,
        }
    }
}

/// Violation for nested blocks.
#[derive(Debug, Clone)]
pub struct NestedBlock;

impl Violation for NestedBlock {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Avoid nested blocks.".to_string()
    }
}

impl Rule for AvoidNestedBlocks {
    fn name(&self) -> &'static str {
        "AvoidNestedBlocks"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only check block nodes
        if node.kind() != "block" {
            return diagnostics;
        }

        // Check if parent is a statement-containing node (equivalent to checkstyle's SLIST)
        // In tree-sitter-java, nested blocks can have these parents:
        // - "block" (nested inside another block)
        // - "switch_block_statement_group" (inside a switch case)
        if let Some(parent) = node.parent() {
            let is_nested =
                parent.kind() == "block" || parent.kind() == "switch_block_statement_group";

            if is_nested {
                // If allowInSwitchCase is true, check if this block has no siblings
                // (meaning it's the only child of a switch case)
                if self.allow_in_switch_case && !has_siblings(&parent, node) {
                    return diagnostics;
                }

                // Report violation
                diagnostics.push(Diagnostic::new(NestedBlock, find_opening_brace(node)));
            }
        }

        diagnostics
    }
}

/// Check if a node has any siblings by checking if parent has more than one statement child.
/// For switch_block_statement_group parents, we only count non-label, non-comment children as siblings.
fn has_siblings(parent: &CstNode, _node: &CstNode) -> bool {
    // Special handling for switch_block_statement_group
    // We only count statement children, not switch_label or comment children
    if parent.kind() == "switch_block_statement_group" {
        let statement_count = parent
            .named_children()
            .filter(|child| {
                child.kind() != "switch_label"
                    && child.kind() != "line_comment"
                    && child.kind() != "block_comment"
            })
            .count();
        return statement_count > 1;
    }

    // For other parents (like "block"), count all named children
    let named_child_count = parent.named_children().count();
    named_child_count > 1
}

/// Find the opening brace of a block for the diagnostic range.
fn find_opening_brace(node: &CstNode) -> TextRange {
    // Find the '{' token
    for child in node.children() {
        if child.kind() == "{" {
            return child.range();
        }
    }
    // Fallback to the node's range
    node.range()
}
