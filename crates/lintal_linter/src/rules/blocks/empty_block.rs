//! EmptyBlock rule implementation.
//!
//! Checks for empty blocks.
//! This is a port of the checkstyle EmptyBlockCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Block option for empty block checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlockOption {
    /// Must have at least one statement.
    #[default]
    Statement,
    /// Must have any text (including comments).
    Text,
}

/// Configuration for EmptyBlock rule.
#[derive(Debug, Clone, Default)]
pub struct EmptyBlock {
    pub option: BlockOption,
}

impl FromConfig for EmptyBlock {
    const MODULE_NAME: &'static str = "EmptyBlock";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| {
                let v = v.trim().to_uppercase();
                match v.as_str() {
                    "TEXT" => BlockOption::Text,
                    "STATEMENT" => BlockOption::Statement,
                    _ => BlockOption::Statement,
                }
            })
            .unwrap_or(BlockOption::Statement);

        Self { option }
    }
}

/// Violation for empty block with no statement.
#[derive(Debug, Clone)]
pub struct EmptyBlockNoStatement;

impl Violation for EmptyBlockNoStatement {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Must have at least one statement.".to_string()
    }
}

/// Violation for empty block with no text.
#[derive(Debug, Clone)]
pub struct EmptyBlockNoText {
    pub block_type: String,
}

impl Violation for EmptyBlockNoText {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("Empty {} block.", self.block_type)
    }
}

impl Rule for EmptyBlock {
    fn name(&self) -> &'static str {
        "EmptyBlock"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Map node kind to block type name for violation message
        // Note: catch blocks are handled by EmptyCatchBlock, not EmptyBlock
        let block_type = match node.kind() {
            "while_statement" => Some("while"),
            "try_statement" => Some("try"),
            // "catch_clause" is NOT checked - handled by EmptyCatchBlock
            "finally" => Some("finally"), // The "finally" keyword itself
            "do_statement" => Some("do"),
            "if_statement" => Some("if"),
            "for_statement" | "enhanced_for_statement" => Some("for"),
            "switch_expression" => Some("switch"),
            "synchronized_statement" => Some("synchronized"),
            "static_initializer" => Some("STATIC_INIT"),
            "block" => {
                // Check if this is an instance initializer (block inside class_body, not part of method)
                if let Some(parent) = node.parent() {
                    if parent.kind() == "class_body" {
                        Some("INSTANCE_INIT")
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "switch_block_statement_group" => {
                // This is for case labels
                if let Some(_parent) = node.parent() {
                    // Find the case or default label
                    if let Some(label) = node.children().find(|c| {
                        c.kind() == "switch_label"
                            && (c.child_by_field_name("value").is_some()
                                || c.children().any(|c| c.kind() == "default"))
                    }) {
                        if label.children().any(|c| c.kind() == "default") {
                            Some("default")
                        } else {
                            Some("case")
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "switch_rule" => {
                // For switch expressions with arrow (->)
                if let Some(label) = node.child_by_field_name("label") {
                    if label.children().any(|c| c.kind() == "default") {
                        Some("default")
                    } else {
                        Some("case")
                    }
                } else {
                    None
                }
            }
            // Note: ARRAY_INIT is NOT in checkstyle's default tokens for EmptyBlock.
            // Empty array initializers {} are allowed by default.
            _ => None,
        };

        if let Some(block_type_name) = block_type
            && let Some(block) = self.find_block(node)
        {
            if self.option == BlockOption::Statement {
                if self.is_empty_statement(&block, node) {
                    diagnostics.push(Diagnostic::new(EmptyBlockNoStatement, block.range()));
                }
            } else if !self.has_text(ctx, &block) {
                diagnostics.push(Diagnostic::new(
                    EmptyBlockNoText {
                        block_type: block_type_name.to_string(),
                    },
                    block.range(),
                ));
            }
        }

        diagnostics
    }
}

impl EmptyBlock {
    /// Find the block associated with a node.
    fn find_block<'a>(&self, node: &'a CstNode) -> Option<CstNode<'a>> {
        match node.kind() {
            "while_statement" | "do_statement" | "for_statement" | "enhanced_for_statement" => node
                .child_by_field_name("body")
                .filter(|body| body.kind() == "block"),
            "if_statement" => {
                // For if without else, check the consequence
                node.child_by_field_name("consequence")
                    .filter(|body| body.kind() == "block")
            }
            "try_statement" => node.child_by_field_name("body"),
            // "catch_clause" is NOT checked - handled by EmptyCatchBlock
            "finally" => {
                // The "finally" keyword - get the next sibling which should be the block
                node.next_named_sibling().filter(|s| s.kind() == "block")
            }
            "synchronized_statement" => node.child_by_field_name("body"),
            "static_initializer" => node.children().find(|c| c.kind() == "block"),
            "block" => {
                // For instance initializers, the block itself is what we want to check
                Some(*node)
            }
            "switch_expression" => node.child_by_field_name("body"),
            "switch_block_statement_group" | "switch_rule" => {
                // For case/default in switch statements/expressions
                node.children().find(|c| c.kind() == "block")
            }
            _ => None,
        }
    }

    /// Check if block is empty (has no statements).
    fn is_empty_statement(&self, block: &CstNode, _parent: &CstNode) -> bool {
        match block.kind() {
            "block" => {
                // A block is empty if it has no children except { }
                // or only has comments/whitespace
                let has_statement = block.children().any(|c| {
                    !matches!(
                        c.kind(),
                        "{" | "}" | "line_comment" | "block_comment" | "ERROR"
                    )
                });
                !has_statement
            }
            "switch_block" => {
                // For switches, check if there are any case_group or switch_rule children
                let has_content = block
                    .children()
                    .any(|c| matches!(c.kind(), "switch_block_statement_group" | "switch_rule"));
                !has_content
            }
            _ => false,
        }
    }

    /// Check if block has any text (including comments).
    fn has_text(&self, ctx: &CheckContext, block: &CstNode) -> bool {
        // Get the range of the block content (between { and })
        let content_range = self.get_block_content_range(block);
        if content_range.is_empty() {
            return false;
        }

        let content = ctx.text_at(content_range);

        // Check if there's any non-whitespace text
        content.chars().any(|c: char| !c.is_whitespace())
    }

    /// Get the range of block content (between braces).
    fn get_block_content_range(&self, block: &CstNode) -> TextRange {
        // Find the opening and closing braces
        let open_brace = block.children().find(|c| c.kind() == "{");
        let close_brace = block.children().find(|c| c.kind() == "}");

        if let (Some(open), Some(close)) = (open_brace, close_brace) {
            TextRange::new(open.range().end(), close.range().start())
        } else {
            // For nodes without explicit braces, return empty range
            TextRange::default()
        }
    }
}
