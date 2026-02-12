//! DescendantToken rule implementation.
//!
//! A meta-rule that counts descendant tokens of a particular type within
//! parent tokens and flags violations based on minimum/maximum counts.
//!
//! Checkstyle equivalent: DescendantTokenCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use tree_sitter::Node;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: descendant token count exceeds maximum.
#[derive(Debug, Clone)]
pub struct DescendantTokenMaxViolation {
    pub message: String,
}

impl Violation for DescendantTokenMaxViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        self.message.clone()
    }
}

/// Violation: descendant token count below minimum.
#[derive(Debug, Clone)]
pub struct DescendantTokenMinViolation {
    pub message: String,
}

impl Violation for DescendantTokenMinViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        self.message.clone()
    }
}

/// A checkstyle token type that maps to tree-sitter node kinds.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CheckstyleToken {
    /// The original checkstyle token name (e.g., "LITERAL_RETURN").
    pub name: String,
}

/// Configuration for DescendantToken rule.
#[derive(Debug, Clone)]
pub struct DescendantToken {
    pub parent_tokens: Vec<CheckstyleToken>,
    pub limited_tokens: Vec<CheckstyleToken>,
    pub minimum_number: usize,
    pub maximum_number: usize,
    pub minimum_depth: usize,
    pub maximum_depth: usize,
    pub sum_token_counts: bool,
    pub minimum_message: Option<String>,
    pub maximum_message: Option<String>,
}

const RELEVANT_KINDS: &[&str] = &["program"];

impl Default for DescendantToken {
    fn default() -> Self {
        Self {
            parent_tokens: vec![],
            limited_tokens: vec![],
            minimum_number: 0,
            maximum_number: i32::MAX as usize,
            minimum_depth: 0,
            maximum_depth: i32::MAX as usize,
            sum_token_counts: false,
            minimum_message: None,
            maximum_message: None,
        }
    }
}

impl FromConfig for DescendantToken {
    const MODULE_NAME: &'static str = "DescendantToken";

    fn from_config(properties: &Properties) -> Self {
        let parent_tokens = properties
            .get("tokens")
            .map(|v| parse_token_list(v))
            .unwrap_or_default();

        let limited_tokens = properties
            .get("limitedTokens")
            .map(|v| parse_token_list(v))
            .unwrap_or_default();

        let minimum_number = properties
            .get("minimumNumber")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let maximum_number = properties
            .get("maximumNumber")
            .and_then(|s| s.parse().ok())
            .unwrap_or(i32::MAX as usize);

        let minimum_depth = properties
            .get("minimumDepth")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let maximum_depth = properties
            .get("maximumDepth")
            .and_then(|s| s.parse().ok())
            .unwrap_or(i32::MAX as usize);

        let sum_token_counts = properties
            .get("sumTokenCounts")
            .is_some_and(|v| *v == "true");

        let minimum_message = properties
            .get("minimumMessage")
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());

        let maximum_message = properties
            .get("maximumMessage")
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string());

        Self {
            parent_tokens,
            limited_tokens,
            minimum_number,
            maximum_number,
            minimum_depth,
            maximum_depth,
            sum_token_counts,
            minimum_message,
            maximum_message,
        }
    }
}

fn parse_token_list(value: &str) -> Vec<CheckstyleToken> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| CheckstyleToken {
            name: s.to_string(),
        })
        .collect()
}

impl Rule for DescendantToken {
    fn name(&self) -> &'static str {
        "DescendantToken"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only run at root node
        if node.parent().is_some() {
            return vec![];
        }

        // If no parent tokens or limited tokens, nothing to do
        if self.parent_tokens.is_empty() {
            return vec![];
        }

        let source = ctx.source();
        let root = node.inner();
        let mut diagnostics = vec![];

        self.walk_tree(&root, source, &mut diagnostics);

        diagnostics
    }
}

impl DescendantToken {
    fn walk_tree(&self, node: &Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
        // Check if this node matches any parent token
        for parent_token in &self.parent_tokens {
            if matches_token(node, source, parent_token) {
                self.check_parent_node(node, source, parent_token, diagnostics);
            }
        }

        // Recurse into children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.walk_tree(&cursor.node(), source, diagnostics);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn check_parent_node(
        &self,
        parent: &Node,
        source: &str,
        parent_token: &CheckstyleToken,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        if self.limited_tokens.is_empty() {
            return;
        }

        let range = token_diagnostic_range(parent);

        if self.sum_token_counts {
            // Count all limited tokens together
            let mut total_count = 0usize;
            for limited_token in &self.limited_tokens {
                total_count += self.count_descendants(parent, source, limited_token, 0);
            }

            // Check maximum
            if total_count > self.maximum_number {
                let message = self.format_maximum_message(
                    parent_token,
                    self.limited_tokens.first().map_or("", |t| &t.name),
                    total_count,
                );
                diagnostics.push(Diagnostic::new(
                    DescendantTokenMaxViolation { message },
                    range,
                ));
            }

            // Check minimum
            if total_count < self.minimum_number {
                let message = self.format_minimum_message_sum(parent_token, total_count);
                diagnostics.push(Diagnostic::new(
                    DescendantTokenMinViolation { message },
                    range,
                ));
            }
        } else {
            // Check each limited token separately
            for limited_token in &self.limited_tokens {
                let count = self.count_descendants(parent, source, limited_token, 0);

                // Check maximum
                if count > self.maximum_number {
                    let message =
                        self.format_maximum_message(parent_token, &limited_token.name, count);
                    diagnostics.push(Diagnostic::new(
                        DescendantTokenMaxViolation { message },
                        range,
                    ));
                }

                // Check minimum
                if count < self.minimum_number {
                    let message = self.format_minimum_message(parent_token, limited_token, count);
                    diagnostics.push(Diagnostic::new(
                        DescendantTokenMinViolation { message },
                        range,
                    ));
                }
            }
        }
    }

    fn count_descendants(
        &self,
        node: &Node,
        source: &str,
        target: &CheckstyleToken,
        current_depth: usize,
    ) -> usize {
        let mut count = 0;

        // Check the node itself at depth 0 or children at depth > 0
        if current_depth >= self.minimum_depth
            && current_depth <= self.maximum_depth
            && matches_token(node, source, target)
        {
            count += 1;
        }

        // Recurse into children if we haven't exceeded max depth
        if current_depth < self.maximum_depth {
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    // Some tree-sitter nodes are transparent wrappers that
                    // don't have checkstyle equivalents — don't count their depth.
                    let child_depth = if is_depth_transparent(cursor.node().kind()) {
                        current_depth
                    } else {
                        current_depth + 1
                    };
                    count += self.count_descendants(&cursor.node(), source, target, child_depth);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }

        count
    }

    fn format_maximum_message(
        &self,
        parent_token: &CheckstyleToken,
        limited_name: &str,
        count: usize,
    ) -> String {
        if let Some(ref msg) = self.maximum_message {
            // Handle message template with {2} placeholder for token name
            msg.replace("{2}", limited_name)
        } else {
            format!(
                "Count of {} for '{}' descendant '{}' exceeds maximum count {}.",
                count, parent_token.name, limited_name, self.maximum_number
            )
        }
    }

    fn format_minimum_message(
        &self,
        parent_token: &CheckstyleToken,
        limited_token: &CheckstyleToken,
        count: usize,
    ) -> String {
        if let Some(ref msg) = self.minimum_message {
            msg.clone()
        } else {
            format!(
                "Count of {} for '{}' descendant '{}' is less than minimum count {}.",
                count, parent_token.name, limited_token.name, self.minimum_number
            )
        }
    }

    fn format_minimum_message_sum(&self, parent_token: &CheckstyleToken, count: usize) -> String {
        if let Some(ref msg) = self.minimum_message {
            msg.clone()
        } else {
            format!(
                "Total count of {} is less than minimum count {} under '{}'.",
                count, self.minimum_number, parent_token.name
            )
        }
    }
}

/// Get a diagnostic range for a parent token node.
/// For binary/assignment expressions, reports at the operator position (matching checkstyle).
fn token_diagnostic_range(node: &Node) -> TextRange {
    if matches!(node.kind(), "binary_expression" | "assignment_expression")
        && let Some(op) = node.child_by_field_name("operator")
    {
        let start = TextSize::new(op.start_byte() as u32);
        let end = TextSize::new(op.end_byte() as u32);
        return TextRange::new(start, end);
    }
    let start = TextSize::new(node.start_byte() as u32);
    let end = TextSize::new(node.end_byte() as u32);
    TextRange::new(start, end)
}

/// Nodes that are transparent wrappers without checkstyle equivalents.
/// These don't count toward depth in descendant counting.
fn is_depth_transparent(kind: &str) -> bool {
    matches!(kind, "switch_block")
}

/// Check if a tree-sitter node matches a checkstyle token.
fn matches_token(node: &Node, source: &str, token: &CheckstyleToken) -> bool {
    let kind = node.kind();
    let text = || &source[node.start_byte()..node.end_byte()];

    match token.name.as_str() {
        // Statement tokens
        "LITERAL_RETURN" => kind == "return_statement",
        "LITERAL_FINALLY" => kind == "finally_clause",
        "LITERAL_CATCH" => kind == "catch_clause",
        "LITERAL_TRY" => kind == "try_statement" || kind == "try_with_resources_statement",
        "LITERAL_ASSERT" => kind == "assert_statement",

        // Switch tokens
        "LITERAL_SWITCH" => kind == "switch_expression" || kind == "switch_statement",
        "LITERAL_DEFAULT" => {
            // Default case in switch
            if kind == "switch_label" {
                text().starts_with("default")
            } else {
                kind == "default" || (kind == "identifier" && text() == "default")
            }
        }

        // Literal keywords
        "LITERAL_THIS" => kind == "this",
        "LITERAL_NULL" => kind == "null_literal",
        "LITERAL_NATIVE" => kind == "native",
        "LITERAL_FOR" => kind == "for_statement" || kind == "enhanced_for_statement",
        "LITERAL_WHILE" => kind == "while_statement",
        "LITERAL_DO" => kind == "do_statement",
        "LITERAL_IF" => kind == "if_statement",
        "LITERAL_ELSE" => kind == "else",
        "LITERAL_BREAK" => kind == "break_statement",
        "LITERAL_CONTINUE" => kind == "continue_statement",
        "LITERAL_THROW" => kind == "throw_statement",
        "LITERAL_NEW" => {
            kind == "object_creation_expression" || kind == "array_creation_expression"
        }

        // Assignment operators
        "ASSIGN" => {
            kind == "assignment_expression" && {
                node.child_by_field_name("operator")
                    .is_some_and(|op| &source[op.start_byte()..op.end_byte()] == "=")
            }
        }
        "PLUS_ASSIGN" => is_assignment_op(node, source, "+="),
        "MINUS_ASSIGN" => is_assignment_op(node, source, "-="),
        "STAR_ASSIGN" => is_assignment_op(node, source, "*="),
        "DIV_ASSIGN" => is_assignment_op(node, source, "/="),
        "MOD_ASSIGN" => is_assignment_op(node, source, "%="),
        "BAND_ASSIGN" => is_assignment_op(node, source, "&="),
        "BOR_ASSIGN" => is_assignment_op(node, source, "|="),
        "BXOR_ASSIGN" => is_assignment_op(node, source, "^="),
        "SL_ASSIGN" => is_assignment_op(node, source, "<<="),
        "SR_ASSIGN" => is_assignment_op(node, source, ">>="),
        "BSR_ASSIGN" => is_assignment_op(node, source, ">>>="),

        // Unary prefix operators
        "DEC" | "INC" => {
            kind == "unary_expression" && {
                let op_text = node.child(0).map(|c| &source[c.start_byte()..c.end_byte()]);
                match token.name.as_str() {
                    "DEC" => op_text == Some("--"),
                    "INC" => op_text == Some("++"),
                    _ => false,
                }
            }
        }

        // Postfix operators
        "POST_DEC" | "POST_INC" => {
            kind == "update_expression" && {
                // The operator is the last child
                let child_count = node.child_count();
                let op_text = if child_count > 0 {
                    node.child((child_count - 1) as u32)
                        .map(|c| &source[c.start_byte()..c.end_byte()])
                } else {
                    None
                };
                match token.name.as_str() {
                    "POST_DEC" => op_text == Some("--"),
                    "POST_INC" => op_text == Some("++"),
                    _ => false,
                }
            }
        }

        // Method and constructor
        "METHOD_CALL" => kind == "method_invocation",
        "METHOD_DEF" => kind == "method_declaration",
        "CTOR_DEF" => kind == "constructor_declaration",

        // Literals
        "STRING_LITERAL" => kind == "string_literal",
        "NUM_INT" => kind == "decimal_integer_literal",
        "NUM_FLOAT" => kind == "decimal_floating_point_literal",
        "NUM_LONG" => kind == "decimal_integer_literal" && text().ends_with('L'),
        "NUM_DOUBLE" => kind == "decimal_floating_point_literal" && text().ends_with('D'),
        "CHAR_LITERAL" => kind == "character_literal",
        "LITERAL_TRUE" => kind == "true",
        "LITERAL_FALSE" => kind == "false",

        // Comparison operators
        "EQUAL" => is_binary_op(node, source, "=="),
        "NOT_EQUAL" => is_binary_op(node, source, "!="),
        "LT" => is_binary_op(node, source, "<"),
        "GT" => is_binary_op(node, source, ">"),
        "LE" => is_binary_op(node, source, "<="),
        "GE" => is_binary_op(node, source, ">="),

        // Logical operators
        "LAND" => is_binary_op(node, source, "&&"),
        "LOR" => is_binary_op(node, source, "||"),
        "LNOT" => {
            kind == "unary_expression" && {
                node.child(0)
                    .is_some_and(|c| &source[c.start_byte()..c.end_byte()] == "!")
            }
        }

        // Bitwise operators
        "BAND" => is_binary_op(node, source, "&"),
        "BOR" => is_binary_op(node, source, "|"),
        "BXOR" => is_binary_op(node, source, "^"),
        "BNOT" => {
            kind == "unary_expression" && {
                node.child(0)
                    .is_some_and(|c| &source[c.start_byte()..c.end_byte()] == "~")
            }
        }

        // Arithmetic operators
        "PLUS" => is_binary_op(node, source, "+"),
        "MINUS" => is_binary_op(node, source, "-"),
        "STAR" => is_binary_op(node, source, "*"),
        "DIV" => is_binary_op(node, source, "/"),
        "MOD" => is_binary_op(node, source, "%"),
        "SL" => is_binary_op(node, source, "<<"),
        "SR" => is_binary_op(node, source, ">>"),
        "BSR" => is_binary_op(node, source, ">>>"),

        // Empty statement - a semicolon in a statement context, not a syntax separator
        "EMPTY_STAT" => kind == ";" && is_empty_statement(node),

        // Definition tokens
        "VARIABLE_DEF" => kind == "local_variable_declaration" || kind == "field_declaration",
        "CLASS_DEF" => kind == "class_declaration",
        "INTERFACE_DEF" => kind == "interface_declaration",
        "ENUM_DEF" => kind == "enum_declaration",
        "ANNOTATION_DEF" => kind == "annotation_type_declaration",
        "PACKAGE_DEF" => kind == "package_declaration",
        "IMPORT" => kind == "import_declaration",

        // Block/brace tokens
        "OBJBLOCK" => {
            kind == "class_body"
                || kind == "interface_body"
                || kind == "enum_body"
                || kind == "annotation_type_body"
        }
        "SLIST" => kind == "block",
        "LCURLY" => kind == "{",
        "RCURLY" => kind == "}",
        "LPAREN" => kind == "(",
        "RPAREN" => kind == ")",
        "LBRACK" => kind == "[",
        "RBRACK" => kind == "]",
        "SEMI" => kind == ";",
        "COMMA" => kind == ",",
        "DOT" => kind == ".",

        // Expression types
        "EXPR" => kind == "expression_statement",
        "QUESTION" => kind == "ternary_expression",
        "LITERAL_INSTANCEOF" => kind == "instanceof_expression",
        "TYPECAST" => kind == "cast_expression",
        "ARRAY_INIT" => kind == "array_initializer",
        "ARRAY_DECLARATOR" => kind == "array_type",
        "INDEX_OP" => kind == "array_access",

        // Misc
        "LABELED_STAT" => kind == "labeled_statement",
        "LITERAL_SYNCHRONIZED" => kind == "synchronized_statement",

        _ => false,
    }
}

/// Check if a `;` node represents an empty statement (not a syntax separator or terminator).
fn is_empty_statement(node: &Node) -> bool {
    let Some(parent) = node.parent() else {
        return false;
    };
    match parent.kind() {
        // Standalone ; in a block is always an empty statement
        "block" | "switch_block_statement_group" => true,
        // Empty body of for/while/if
        "for_statement" => {
            // Only the ; after ) is the empty body; ; inside () is for-loop syntax
            let mut cursor = parent.walk();
            let mut after_close_paren = false;
            for child in parent.children(&mut cursor) {
                if child.kind() == ")" {
                    after_close_paren = true;
                }
                if after_close_paren && child.id() == node.id() {
                    return true;
                }
            }
            false
        }
        "if_statement" | "while_statement" => true,
        "do_statement" => {
            // Only the ; immediately after "do" is the empty body
            // The ; after while(...) is the do-while terminator
            let mut cursor = parent.walk();
            for child in parent.children(&mut cursor) {
                if child.kind() == "while" {
                    // If we haven't found our ; yet, it must be after while → terminator
                    return false;
                }
                if child.id() == node.id() {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn is_binary_op(node: &Node, source: &str, op: &str) -> bool {
    node.kind() == "binary_expression"
        && node
            .child_by_field_name("operator")
            .is_some_and(|o| &source[o.start_byte()..o.end_byte()] == op)
}

fn is_assignment_op(node: &Node, source: &str, op: &str) -> bool {
    node.kind() == "assignment_expression"
        && node
            .child_by_field_name("operator")
            .is_some_and(|o| &source[o.start_byte()..o.end_byte()] == op)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str, rule: &DescendantToken) -> Vec<(usize, String)> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push((loc.line.get(), d.kind.body.clone()));
            }
        }
        violations
    }

    #[test]
    fn test_return_from_finally() {
        let source = r#"
class Test {
    public void foo() {
        try {
            System.currentTimeMillis();
        } finally {
            return;
        }
    }
}
"#;
        let rule = DescendantToken {
            parent_tokens: vec![CheckstyleToken {
                name: "LITERAL_FINALLY".to_string(),
            }],
            limited_tokens: vec![CheckstyleToken {
                name: "LITERAL_RETURN".to_string(),
            }],
            maximum_number: 0,
            maximum_message: Some("Return from finally is not allowed.".to_string()),
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].1.contains("Return from finally"));
    }

    #[test]
    fn test_missing_switch_default() {
        let source = r#"
class Test {
    public void foo() {
        int i = 1;
        switch (i) {
            case 1: i++; break;
            case 2: i--; break;
        }
    }
}
"#;
        let rule = DescendantToken {
            parent_tokens: vec![CheckstyleToken {
                name: "LITERAL_SWITCH".to_string(),
            }],
            limited_tokens: vec![CheckstyleToken {
                name: "LITERAL_DEFAULT".to_string(),
            }],
            minimum_number: 1,
            maximum_depth: 2,
            minimum_message: Some("switch without \"default\" clause.".to_string()),
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_no_violation_with_default() {
        let source = r#"
class Test {
    public void foo() {
        int i = 1;
        switch (i) {
            case 1: i++; break;
            default: return;
        }
    }
}
"#;
        let rule = DescendantToken {
            parent_tokens: vec![CheckstyleToken {
                name: "LITERAL_SWITCH".to_string(),
            }],
            limited_tokens: vec![CheckstyleToken {
                name: "LITERAL_DEFAULT".to_string(),
            }],
            minimum_number: 1,
            maximum_depth: 2,
            minimum_message: Some("switch without \"default\" clause.".to_string()),
            ..Default::default()
        };
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }
}
