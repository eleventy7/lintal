//! EmptyCatchBlock rule implementation.
//!
//! Checks for empty catch blocks.
//! This is a port of the checkstyle EmptyCatchBlockCheck for 100% compatibility.

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for EmptyCatchBlock rule.
#[derive(Debug, Clone)]
pub struct EmptyCatchBlock {
    pub exception_variable_name: Regex,
    pub comment_format: Regex,
}

const RELEVANT_KINDS: &[&str] = &["catch_clause"];

impl Default for EmptyCatchBlock {
    fn default() -> Self {
        Self {
            // Default: match nothing (^$) for exception variable name
            exception_variable_name: Regex::new("^$").unwrap(),
            // Default: match anything (.*) for comment format
            comment_format: Regex::new(".*").unwrap(),
        }
    }
}

impl FromConfig for EmptyCatchBlock {
    const MODULE_NAME: &'static str = "EmptyCatchBlock";

    fn from_config(properties: &Properties) -> Self {
        let exception_variable_name = properties
            .get("exceptionVariableName")
            .and_then(|v| Regex::new(v).ok())
            .unwrap_or_else(|| Regex::new("^$").unwrap());

        let comment_format = properties
            .get("commentFormat")
            .and_then(|v| Regex::new(v).ok())
            .unwrap_or_else(|| Regex::new(".*").unwrap());

        Self {
            exception_variable_name,
            comment_format,
        }
    }
}

/// Violation for empty catch block.
#[derive(Debug, Clone)]
pub struct EmptyCatchBlockViolation;

impl Violation for EmptyCatchBlockViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Empty catch block.".to_string()
    }
}

impl Rule for EmptyCatchBlock {
    fn name(&self) -> &'static str {
        "EmptyCatchBlock"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Only process catch_clause nodes
        if node.kind() != "catch_clause" {
            return diagnostics;
        }

        // Check if catch block is empty (no statements, only comments)
        if !self.is_empty_catch_block(node) {
            return diagnostics;
        }

        // Get the exception variable name
        let variable_name = self.get_exception_variable_name(node);

        // Check if variable name matches the regex (suppresses violation)
        if self.exception_variable_name.is_match(&variable_name) {
            return diagnostics;
        }

        // Get the first comment in the catch block
        let comment_content = self.get_comment_first_line(node);

        // Check if comment matches the regex (suppresses violation)
        if !comment_content.is_empty() && self.comment_format.is_match(&comment_content) {
            return diagnostics;
        }

        // If we reach here, the catch block is empty and should be flagged
        if let Some(block) = node.child_by_field_name("body") {
            diagnostics.push(Diagnostic::new(EmptyCatchBlockViolation, block.range()));
        }

        diagnostics
    }
}

impl EmptyCatchBlock {
    /// Check if catch block is empty (contains no statements, only comments).
    fn is_empty_catch_block(&self, catch_node: &CstNode) -> bool {
        if let Some(block) = catch_node.child_by_field_name("body") {
            // A catch block is empty if it has no children except { }, comments
            let has_statement = block.children().any(|c| {
                !matches!(
                    c.kind(),
                    "{" | "}" | "line_comment" | "block_comment" | "ERROR"
                )
            });
            !has_statement
        } else {
            false
        }
    }

    /// Get the exception variable name from a catch clause.
    fn get_exception_variable_name(&self, catch_node: &CstNode) -> String {
        // Find the catch_formal_parameter
        if let Some(param) = catch_node
            .children()
            .find(|c| c.kind() == "catch_formal_parameter")
        {
            // Find the identifier
            if let Some(ident) = param.children().find(|c| c.kind() == "identifier") {
                return ident.text().to_string();
            }
        }
        String::new()
    }

    /// Get the first line of the first comment in the catch block.
    fn get_comment_first_line(&self, catch_node: &CstNode) -> String {
        if let Some(block) = catch_node.child_by_field_name("body") {
            // Find the first comment node
            for child in block.children() {
                match child.kind() {
                    "line_comment" => {
                        // For line comments, get the full text
                        let text = child.text().to_string();
                        // The text includes //, so we need to check if there's a child text node
                        // or extract after //
                        // Based on tree-sitter-java structure, line_comment text includes //
                        // We want just the comment content
                        if let Some(stripped) = text.strip_prefix("//") {
                            return stripped.to_string();
                        }
                        return text;
                    }
                    "block_comment" => {
                        // For block comments, get the full text
                        let text = child.text().to_string();
                        // Strip /* and */
                        let content = text
                            .strip_prefix("/*")
                            .and_then(|s| s.strip_suffix("*/"))
                            .unwrap_or(&text);

                        // Split by line endings and find first non-empty line
                        // This matches checkstyle's behavior: if no non-empty line is found,
                        // keep the original content (which may contain newlines/whitespace)
                        let mut result = content.to_string();
                        for line in content.lines() {
                            if !line.is_empty() {
                                result = line.to_string();
                                break;
                            }
                        }
                        return result;
                    }
                    _ => {}
                }
            }
        }
        String::new()
    }
}
