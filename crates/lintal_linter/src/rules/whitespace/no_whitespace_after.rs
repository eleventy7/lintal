//! NoWhitespaceAfter rule implementation.
//!
//! Checks that there is no whitespace after specific tokens.
//! Checkstyle equivalent: NoWhitespaceAfter

use std::collections::HashSet;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;

use crate::rules::whitespace::common::{diag_followed, whitespace_range_after};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by NoWhitespaceAfter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoWhitespaceAfterToken {
    ArrayInit,
    At,
    Inc,
    Dec,
    UnaryMinus,
    UnaryPlus,
    Bnot,
    Lnot,
    Dot,
    ArrayDeclarator,
    IndexOp,
    Typecast,
    LiteralSynchronized,
    MethodRef,
}

impl NoWhitespaceAfterToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "ARRAY_INIT" => Some(Self::ArrayInit),
            "AT" => Some(Self::At),
            "INC" => Some(Self::Inc),
            "DEC" => Some(Self::Dec),
            "UNARY_MINUS" => Some(Self::UnaryMinus),
            "UNARY_PLUS" => Some(Self::UnaryPlus),
            "BNOT" => Some(Self::Bnot),
            "LNOT" => Some(Self::Lnot),
            "DOT" => Some(Self::Dot),
            "ARRAY_DECLARATOR" => Some(Self::ArrayDeclarator),
            "INDEX_OP" => Some(Self::IndexOp),
            "TYPECAST" => Some(Self::Typecast),
            "LITERAL_SYNCHRONIZED" => Some(Self::LiteralSynchronized),
            "METHOD_REF" => Some(Self::MethodRef),
            _ => None,
        }
    }
}

/// Configuration for NoWhitespaceAfter rule.
#[derive(Debug, Clone)]
pub struct NoWhitespaceAfter {
    /// Which tokens to check.
    pub tokens: HashSet<NoWhitespaceAfterToken>,
    /// Allow line breaks after token.
    pub allow_line_breaks: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "{",
    "@",
    "++",
    "--",
    "-",
    "+",
    "~",
    "!",
    ".",
    "dimensions",
    "dimensions_expr",
    "array_access",
    "cast_expression",
    "synchronized_statement",
    "::",
];

impl Default for NoWhitespaceAfter {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(NoWhitespaceAfterToken::ArrayInit);
        tokens.insert(NoWhitespaceAfterToken::At);
        tokens.insert(NoWhitespaceAfterToken::Inc);
        tokens.insert(NoWhitespaceAfterToken::Dec);
        tokens.insert(NoWhitespaceAfterToken::UnaryMinus);
        tokens.insert(NoWhitespaceAfterToken::UnaryPlus);
        tokens.insert(NoWhitespaceAfterToken::Bnot);
        tokens.insert(NoWhitespaceAfterToken::Lnot);
        tokens.insert(NoWhitespaceAfterToken::Dot);
        tokens.insert(NoWhitespaceAfterToken::ArrayDeclarator);
        tokens.insert(NoWhitespaceAfterToken::IndexOp);
        Self {
            tokens,
            allow_line_breaks: true,
        }
    }
}

impl FromConfig for NoWhitespaceAfter {
    const MODULE_NAME: &'static str = "NoWhitespaceAfter";

    fn from_config(properties: &Properties) -> Self {
        let tokens_str = properties.get("tokens").copied().unwrap_or("");
        let tokens: HashSet<_> = if tokens_str.is_empty() {
            Self::default().tokens
        } else {
            tokens_str
                .split(',')
                .filter_map(NoWhitespaceAfterToken::from_str)
                .collect()
        };

        let allow_line_breaks = properties
            .get("allowLineBreaks")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(true);

        Self {
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
            allow_line_breaks,
        }
    }
}

impl Rule for NoWhitespaceAfter {
    fn name(&self) -> &'static str {
        "NoWhitespaceAfter"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            // Array initializer: int[] a = {}
            "{" if self.tokens.contains(&NoWhitespaceAfterToken::ArrayInit) => {
                if is_array_init(node)
                    && let Some(ws_range) = self.check_whitespace_after(ctx, node)
                {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Annotation: @Override, @SuppressWarnings
            "@" if self.tokens.contains(&NoWhitespaceAfterToken::At) => {
                if let Some(ws_range) = self.check_whitespace_after(ctx, node) {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Increment: ++i (prefix only, not i++)
            "++" if self.tokens.contains(&NoWhitespaceAfterToken::Inc) => {
                // Only check prefix increment - the operator comes BEFORE its operand
                // For postfix (i++), what follows is the next token, not the operand
                if is_prefix_update_op(node)
                    && let Some(ws_range) = self.check_whitespace_after(ctx, node)
                {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Decrement: --i (prefix only, not i--)
            "--" if self.tokens.contains(&NoWhitespaceAfterToken::Dec) => {
                // Only check prefix decrement - the operator comes BEFORE its operand
                if is_prefix_update_op(node)
                    && let Some(ws_range) = self.check_whitespace_after(ctx, node)
                {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Unary minus: -x
            "-" if self.tokens.contains(&NoWhitespaceAfterToken::UnaryMinus) => {
                if is_unary_op(node)
                    && let Some(ws_range) = self.check_whitespace_after(ctx, node)
                {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Unary plus: +x
            "+" if self.tokens.contains(&NoWhitespaceAfterToken::UnaryPlus) => {
                if is_unary_op(node)
                    && let Some(ws_range) = self.check_whitespace_after(ctx, node)
                {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Bitwise NOT: ~x
            "~" if self.tokens.contains(&NoWhitespaceAfterToken::Bnot) => {
                if let Some(ws_range) = self.check_whitespace_after(ctx, node) {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Logical NOT: !x
            "!" if self.tokens.contains(&NoWhitespaceAfterToken::Lnot) => {
                if let Some(ws_range) = self.check_whitespace_after(ctx, node) {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Dot: obj.field
            "." if self.tokens.contains(&NoWhitespaceAfterToken::Dot) => {
                if let Some(ws_range) = self.check_whitespace_after(ctx, node) {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            // Array declarator: int[] or int arr[]
            "dimensions"
                if self
                    .tokens
                    .contains(&NoWhitespaceAfterToken::ArrayDeclarator) =>
            {
                diagnostics.extend(self.check_array_declarator(ctx, node));
            }

            // Array creation with expression: new int[50]
            "dimensions_expr"
                if self
                    .tokens
                    .contains(&NoWhitespaceAfterToken::ArrayDeclarator) =>
            {
                diagnostics.extend(self.check_array_declarator(ctx, node));
            }

            // Array index: arr[0]
            "array_access" if self.tokens.contains(&NoWhitespaceAfterToken::IndexOp) => {
                diagnostics.extend(self.check_index_op(ctx, node));
            }

            // Typecast: (Type) value
            "cast_expression" if self.tokens.contains(&NoWhitespaceAfterToken::Typecast) => {
                if let Some(rparen) = node.children().find(|c| c.kind() == ")")
                    && let Some(ws_range) = self.check_whitespace_after(ctx, &rparen)
                {
                    diagnostics.push(diag_followed(&rparen, ws_range));
                }
            }

            // Synchronized statement: synchronized(this) {}
            "synchronized_statement"
                if self
                    .tokens
                    .contains(&NoWhitespaceAfterToken::LiteralSynchronized) =>
            {
                if let Some(kw) = node.children().find(|c| c.kind() == "synchronized")
                    && let Some(ws_range) = self.check_whitespace_after(ctx, &kw)
                {
                    diagnostics.push(diag_followed(&kw, ws_range));
                }
            }

            // Method reference: String::new
            "::" if self.tokens.contains(&NoWhitespaceAfterToken::MethodRef) => {
                if let Some(ws_range) = self.check_whitespace_after(ctx, node) {
                    diagnostics.push(diag_followed(node, ws_range));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

impl NoWhitespaceAfter {
    /// Check if there's unwanted whitespace after a token.
    /// Returns the range of whitespace if found.
    fn check_whitespace_after(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
    ) -> Option<lintal_text_size::TextRange> {
        let after_pos = node.range().end();
        let source = ctx.source();

        // Check for whitespace after the token
        if let Some(ws_range) = whitespace_range_after(source, after_pos) {
            // If we allow line breaks, only report non-newline whitespace
            if self.allow_line_breaks {
                // Check if whitespace is only newline(s)
                let ws_text = &source[usize::from(ws_range.start())..usize::from(ws_range.end())];
                if ws_text.chars().all(|c| c == '\n' || c == '\r') {
                    return None;
                }
            }
            Some(ws_range)
        } else {
            None
        }
    }

    /// Check array declarator: int[], int arr[], new int[]
    /// Special handling: checks whitespace BEFORE the bracket, not after.
    fn check_array_declarator(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let source = ctx.source();

        // Find all "[" tokens in dimensions
        for bracket in node.children().filter(|c| c.kind() == "[") {
            let before_pos = bracket.range().start();

            // Check if there's an annotation before the bracket
            // If so, skip this bracket (per checkstyle spec)
            if has_annotation_before(node, &bracket) {
                continue;
            }

            // Check for whitespace BEFORE the bracket
            if let Some(ws_range) = whitespace_range_before(source, before_pos) {
                // If we allow line breaks, only report non-newline whitespace
                if self.allow_line_breaks {
                    let ws_text =
                        &source[usize::from(ws_range.start())..usize::from(ws_range.end())];
                    if ws_text.chars().all(|c| c == '\n' || c == '\r') {
                        continue;
                    }
                }

                // Find what token precedes the whitespace
                let ws_start = ws_range.start();
                if let Some(prev_token) = find_token_before(ctx, node, ws_start) {
                    diagnostics.push(diag_followed(&prev_token, ws_range));
                }
            }
        }

        diagnostics
    }

    /// Check array index operation: arr[0]
    /// Special handling: checks whitespace BEFORE the bracket, not after.
    fn check_index_op(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let source = ctx.source();

        // Find the "[" token
        if let Some(bracket) = node.children().find(|c| c.kind() == "[") {
            let before_pos = bracket.range().start();

            // Check for whitespace BEFORE the bracket
            if let Some(ws_range) = whitespace_range_before(source, before_pos) {
                // Find what token precedes the whitespace
                let ws_start = ws_range.start();
                if let Some(prev_token) = find_token_before(ctx, node, ws_start) {
                    // If we allow line breaks, only report non-newline whitespace
                    if self.allow_line_breaks {
                        let ws_text =
                            &source[usize::from(ws_range.start())..usize::from(ws_range.end())];
                        if ws_text.chars().all(|c| c == '\n' || c == '\r') {
                            return diagnostics;
                        }
                    }
                    diagnostics.push(diag_followed(&prev_token, ws_range));
                }
            }
        }

        diagnostics
    }
}

/// Check if node is an array initializer (not just any brace).
fn is_array_init(node: &CstNode) -> bool {
    // Array initializer is a "{" that's a child of array_initializer
    node.parent()
        .is_some_and(|p| p.kind() == "array_initializer")
}

/// Check if a + or - is a unary operator (not binary).
fn is_unary_op(node: &CstNode) -> bool {
    node.parent()
        .is_some_and(|p| p.kind() == "unary_expression")
}

/// Check if ++ or -- is a prefix operator (++i, not i++).
/// In tree-sitter, update_expression contains the operator and operand.
/// For prefix, the operator comes first; for postfix, the operand comes first.
fn is_prefix_update_op(node: &CstNode) -> bool {
    if let Some(parent) = node.parent()
        && parent.kind() == "update_expression"
    {
        // If this node is the first child, it's prefix
        if let Some(first_child) = parent.children().next() {
            return first_child.range().start() == node.range().start();
        }
    }
    false
}

/// Check if there's an annotation before the bracket.
fn has_annotation_before(dimensions: &CstNode, bracket: &CstNode) -> bool {
    // Look for annotation nodes before this bracket
    if let Some(parent) = dimensions.parent() {
        for child in parent.children() {
            if child.range().end() <= bracket.range().start()
                && (child.kind() == "annotation" || child.kind() == "marker_annotation")
            {
                return true;
            }
        }
    }
    false
}

/// Find whitespace before a position.
fn whitespace_range_before(source: &str, pos: TextSize) -> Option<lintal_text_size::TextRange> {
    crate::rules::whitespace::common::whitespace_range_before(source, pos)
}

/// Find the token that precedes a given position.
/// For method declarations like `int get() []`, returns the method name "get"
/// even though the position is after the closing paren.
fn find_token_before<'a>(
    _ctx: &CheckContext,
    node: &'a CstNode,
    pos: TextSize,
) -> Option<CstNode<'a>> {
    // Look for the last non-whitespace token before pos
    let mut best: Option<CstNode> = None;

    // Walk all children and find the one closest before pos
    fn walk<'a>(n: &CstNode<'a>, pos: TextSize, best: &mut Option<CstNode<'a>>) {
        if n.range().end() <= pos && !n.kind().is_empty() && !n.text().trim().is_empty() {
            if let Some(current) = best {
                if n.range().end() > current.range().end() {
                    *best = Some(*n);
                }
            } else {
                *best = Some(*n);
            }
        }
        for child in n.children() {
            walk(&child, pos, best);
        }
    }

    // Start from parent to get broader context
    if let Some(parent) = node.parent() {
        walk(&parent, pos, &mut best);
    }
    walk(node, pos, &mut best);

    // Special case: if best is ")" and it's part of a method declaration,
    // find the method identifier instead
    if let Some(ref token) = best
        && token.kind() == ")"
        && let Some(method_name) = find_method_name_for_paren(token)
    {
        return Some(method_name);
    }

    best
}

/// Find the method name identifier for a closing paren in a method declaration.
/// For `int get() []`, finds "get".
fn find_method_name_for_paren<'a>(rparen: &CstNode<'a>) -> Option<CstNode<'a>> {
    // The rparen is inside formal_parameters
    // formal_parameters is a child of method_declarator
    // method_declarator contains the identifier we need
    let params = rparen.parent()?;
    if params.kind() != "formal_parameters" {
        return None;
    }

    let declarator = params.parent()?;
    if declarator.kind() != "method_declarator" && declarator.kind() != "constructor_declarator" {
        return None;
    }

    // Find the identifier in this declarator (should be first child)
    declarator
        .children()
        .find(|child| child.kind() == "identifier")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &NoWhitespaceAfter::default())
    }

    fn check_source_with_config(source: &str, rule: &NoWhitespaceAfter) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_dot_with_space() {
        let diagnostics = check_source("class Foo { void m() { obj. toString(); } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains(".") && d.kind.body.contains("followed")),
            "Should detect dot with space: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_dot_without_space() {
        let diagnostics = check_source("class Foo { void m() { obj.toString(); } }");
        let dot_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("."))
            .collect();
        assert!(
            dot_violations.is_empty(),
            "Should not flag dot without space"
        );
    }

    #[test]
    fn test_annotation_with_space() {
        let diagnostics = check_source("@ interface Foo {}");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("@") && d.kind.body.contains("followed")),
            "Should detect @ with space: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_annotation_without_space() {
        let diagnostics = check_source("@interface Foo {}");
        let at_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("@"))
            .collect();
        assert!(at_violations.is_empty(), "Should not flag @ without space");
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { void m() { obj. toString(); } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }

    #[test]
    fn test_array_creation_with_space() {
        let diagnostics = check_source("class Foo { void m() { int[] a = new int [50]; } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("int") && d.kind.body.contains("followed")),
            "Should detect 'new int [50]' with space: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_array_creation_without_space() {
        let diagnostics = check_source("class Foo { void m() { int[] a = new int[50]; } }");
        let int_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("int"))
            .collect();
        assert!(
            int_violations.is_empty(),
            "Should not flag 'new int[50]' without space"
        );
    }
}
