//! NeedBraces rule implementation.
//!
//! Checks for braces around code blocks.
//! This is a port of the checkstyle NeedBracesCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for NeedBraces rule.
#[derive(Debug, Clone, Default)]
pub struct NeedBraces {
    pub allow_single_line_statement: bool,
    pub allow_empty_loop_body: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "enhanced_for_statement",
];

impl FromConfig for NeedBraces {
    const MODULE_NAME: &'static str = "NeedBraces";

    fn from_config(properties: &Properties) -> Self {
        let allow_single_line_statement = properties
            .get("allowSingleLineStatement")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        let allow_empty_loop_body = properties
            .get("allowEmptyLoopBody")
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(false);

        Self {
            allow_single_line_statement,
            allow_empty_loop_body,
        }
    }
}

/// Violation for missing braces.
#[derive(Debug, Clone)]
pub struct NeedBracesViolation {
    pub construct: String,
}

impl Violation for NeedBracesViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("'{}' construct must use '{{}}'s", self.construct)
    }
}

impl Rule for NeedBraces {
    fn name(&self) -> &'static str {
        "NeedBraces"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            "if_statement" => {
                diagnostics.extend(self.check_if_statement(ctx, node));
            }
            "while_statement" => {
                diagnostics.extend(self.check_while_statement(ctx, node));
            }
            "do_statement" => {
                diagnostics.extend(self.check_do_statement(ctx, node));
            }
            "for_statement" | "enhanced_for_statement" => {
                diagnostics.extend(self.check_for_statement(ctx, node));
            }
            _ => {}
        }

        diagnostics
    }
}

impl NeedBraces {
    /// Check if statement for missing braces.
    fn check_if_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check the consequence (then branch)
        if let Some(consequence) = node.child_by_field_name("consequence")
            && consequence.kind() != "block"
            && !self.is_skip_statement(ctx, node, "if")
        {
            diagnostics.push(Diagnostic::new(
                NeedBracesViolation {
                    construct: "if".to_string(),
                },
                node.range(),
            ));
        }

        // Check the alternative (else branch)
        if let Some(alternative) = node.child_by_field_name("alternative")
            && alternative.kind() != "block"
            && alternative.kind() != "if_statement"
        {
            // Find the "else" keyword for the diagnostic location
            if let Some(else_kw) = node.children().find(|c| c.kind() == "else")
                && !self.is_skip_statement(ctx, node, "else")
            {
                diagnostics.push(Diagnostic::new(
                    NeedBracesViolation {
                        construct: "else".to_string(),
                    },
                    else_kw.range(),
                ));
            }
        }

        diagnostics
    }

    /// Check while statement for missing braces.
    fn check_while_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body") {
            // Check if it's an empty statement
            if self.allow_empty_loop_body && body.kind() == ";" {
                return diagnostics;
            }

            if body.kind() != "block" && !self.is_skip_statement(ctx, node, "while") {
                diagnostics.push(Diagnostic::new(
                    NeedBracesViolation {
                        construct: "while".to_string(),
                    },
                    node.range(),
                ));
            }
        }

        diagnostics
    }

    /// Check do-while statement for missing braces.
    fn check_do_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body")
            && body.kind() != "block"
            && !self.is_skip_statement(ctx, node, "do")
        {
            diagnostics.push(Diagnostic::new(
                NeedBracesViolation {
                    construct: "do".to_string(),
                },
                node.range(),
            ));
        }

        diagnostics
    }

    /// Check for statement for missing braces.
    fn check_for_statement(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        if let Some(body) = node.child_by_field_name("body") {
            // Check if it's an empty statement
            if self.allow_empty_loop_body && body.kind() == ";" {
                return diagnostics;
            }

            if body.kind() != "block" && !self.is_skip_statement(ctx, node, "for") {
                diagnostics.push(Diagnostic::new(
                    NeedBracesViolation {
                        construct: "for".to_string(),
                    },
                    node.range(),
                ));
            }
        }

        diagnostics
    }

    /// Check if current statement can be skipped by "need braces" warning.
    /// This implements the allowSingleLineStatement option.
    fn is_skip_statement(&self, ctx: &CheckContext, node: &CstNode, kind: &str) -> bool {
        if !self.allow_single_line_statement {
            return false;
        }

        // Check if the statement is single-line
        self.is_single_line_statement(ctx, node, kind)
    }

    /// Check if current statement is single-line statement.
    fn is_single_line_statement(&self, ctx: &CheckContext, node: &CstNode, kind: &str) -> bool {
        // For single-line detection, we need to check if the statement and its body are on the same line
        // This matches checkstyle's logic

        // First check if the parent is a block (SLIST in checkstyle terms)
        if let Some(parent) = node.parent() {
            if parent.kind() != "block" {
                return false;
            }
        } else {
            return false;
        }

        let source_code = ctx.source_code();

        let start_line = source_code.line_column(node.range().start()).line;

        match kind {
            "if" => {
                // For if, check if condition and consequence are on same line
                if let Some(consequence) = node.child_by_field_name("consequence") {
                    let consequence_line =
                        source_code.line_column(consequence.range().start()).line;
                    start_line == consequence_line
                } else {
                    false
                }
            }
            "else" => {
                // For else, check if else keyword and body are on same line
                if let Some(alternative) = node.child_by_field_name("alternative") {
                    let else_keyword = node.children().find(|c| c.kind() == "else");
                    if let Some(else_kw) = else_keyword {
                        let else_line = source_code.line_column(else_kw.range().start()).line;
                        let alt_line = source_code.line_column(alternative.range().start()).line;
                        else_line == alt_line
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            "while" => {
                // For while, check if while and body are on same line
                if let Some(body) = node.child_by_field_name("body") {
                    let body_line = source_code.line_column(body.range().start()).line;
                    start_line == body_line
                } else {
                    false
                }
            }
            "do" => {
                // For do-while, check if do and body are on same line AND while is on same line
                if let Some(_body) = node.child_by_field_name("body") {
                    let end_line = source_code.line_column(node.range().end()).line;
                    start_line == end_line
                } else {
                    false
                }
            }
            "for" => {
                // For for, check if for and body are on same line
                // Also, empty statement (;) counts as single line
                if let Some(body) = node.child_by_field_name("body") {
                    if body.kind() == ";" {
                        return true;
                    }
                    let body_line = source_code.line_column(body.range().start()).line;
                    start_line == body_line
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
