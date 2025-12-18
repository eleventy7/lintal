//! WhitespaceAround rule implementation.
//!
//! Checks that tokens are surrounded by whitespace. This is a port of the
//! checkstyle WhitespaceAround check for 100% compatibility.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextSize;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for WhitespaceAround rule.
#[derive(Debug, Clone)]
pub struct WhitespaceAround {
    /// Allow empty constructor bodies: `Foo() {}`
    pub allow_empty_constructors: bool,
    /// Allow empty method bodies: `void foo() {}`
    pub allow_empty_methods: bool,
    /// Allow empty class/interface/enum bodies: `class Foo {}`
    pub allow_empty_types: bool,
    /// Allow empty loop bodies: `while (true) {}`
    pub allow_empty_loops: bool,
    /// Allow empty lambda bodies: `() -> {}`
    pub allow_empty_lambdas: bool,
    /// Allow empty catch blocks: `catch (Exception e) {}`
    pub allow_empty_catches: bool,
    /// Ignore whitespace around colon in enhanced for loop
    pub ignore_enhanced_for_colon: bool,
    /// Check whitespace around `<` in generics (GENERIC_START token)
    pub check_generic_start: bool,
    /// Check whitespace around `>` in generics (GENERIC_END token)
    pub check_generic_end: bool,
    /// Check whitespace around `?` in generics (WILDCARD_TYPE token)
    pub check_wildcard_type: bool,
}

impl Default for WhitespaceAround {
    fn default() -> Self {
        Self {
            allow_empty_constructors: false,
            allow_empty_methods: false,
            allow_empty_types: false,
            allow_empty_loops: false,
            allow_empty_lambdas: false,
            allow_empty_catches: false,
            ignore_enhanced_for_colon: true,
            // Generics tokens are NOT checked by default (matches checkstyle)
            check_generic_start: false,
            check_generic_end: false,
            check_wildcard_type: false,
        }
    }
}

impl FromConfig for WhitespaceAround {
    const MODULE_NAME: &'static str = "WhitespaceAround";

    fn from_config(properties: &Properties) -> Self {
        // Check if tokens property includes generics tokens
        let tokens = properties.get("tokens").copied().unwrap_or("");
        let check_generic_start = tokens.contains("GENERIC_START");
        let check_generic_end = tokens.contains("GENERIC_END");
        let check_wildcard_type = tokens.contains("WILDCARD_TYPE");

        Self {
            allow_empty_constructors: properties
                .get("allowEmptyConstructors")
                .map(|v| *v == "true")
                .unwrap_or(false),
            allow_empty_methods: properties
                .get("allowEmptyMethods")
                .map(|v| *v == "true")
                .unwrap_or(false),
            allow_empty_types: properties
                .get("allowEmptyTypes")
                .map(|v| *v == "true")
                .unwrap_or(false),
            allow_empty_loops: properties
                .get("allowEmptyLoops")
                .map(|v| *v == "true")
                .unwrap_or(false),
            allow_empty_lambdas: properties
                .get("allowEmptyLambdas")
                .map(|v| *v == "true")
                .unwrap_or(false),
            allow_empty_catches: properties
                .get("allowEmptyCatches")
                .map(|v| *v == "true")
                .unwrap_or(false),
            ignore_enhanced_for_colon: properties
                .get("ignoreEnhancedForColon")
                .map(|v| *v == "true")
                .unwrap_or(true),
            check_generic_start,
            check_generic_end,
            check_wildcard_type,
        }
    }
}

/// Violation for missing whitespace before a token.
#[derive(Debug, Clone)]
pub struct MissingWhitespaceBefore {
    pub token: String,
}

impl Violation for MissingWhitespaceBefore {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Missing whitespace before `{}`", self.token)
    }
}

/// Violation for missing whitespace after a token.
#[derive(Debug, Clone)]
pub struct MissingWhitespaceAfter {
    pub token: String,
}

impl Violation for MissingWhitespaceAfter {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Missing whitespace after `{}`", self.token)
    }
}

impl Rule for WhitespaceAround {
    fn name(&self) -> &'static str {
        "WhitespaceAround"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            // Binary expressions: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, etc.
            "binary_expression" => {
                if let Some(op) = find_binary_operator(node) {
                    diagnostics.extend(check_whitespace_around(ctx, &op));
                }
            }

            // Assignment expressions: =, +=, -=, *=, /=, etc.
            "assignment_expression" => {
                if let Some(op) = find_assignment_operator(node) {
                    diagnostics.extend(check_whitespace_around(ctx, &op));
                }
            }

            // Variable declarator with initializer: int x = 1
            "variable_declarator" => {
                if let Some(op) = find_equals_in_declarator(node) {
                    diagnostics.extend(check_whitespace_around(ctx, &op));
                }
            }

            // Ternary expression: a ? b : c
            "ternary_expression" => {
                diagnostics.extend(self.check_ternary(ctx, node));
            }

            // Keywords that should be followed by whitespace
            "if_statement" => {
                if let Some(kw) = find_keyword(node, "if") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "while_statement" => {
                if let Some(kw) = find_keyword(node, "while") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "do_statement" => {
                // Check 'do' keyword
                if let Some(kw) = find_keyword(node, "do") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
                // Check 'while' in do-while (it's the DO_WHILE token in checkstyle)
                if let Some(kw) = find_keyword(node, "while") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "for_statement" => {
                if let Some(kw) = find_keyword(node, "for") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "enhanced_for_statement" => {
                if let Some(kw) = find_keyword(node, "for") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
                // Check colon in enhanced for, unless ignored
                if !self.ignore_enhanced_for_colon
                    && let Some(colon) = find_child_by_kind(node, ":")
                {
                    diagnostics.extend(check_whitespace_around(ctx, &colon));
                }
            }
            "switch_expression" | "switch_statement" => {
                if let Some(kw) = find_keyword(node, "switch") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "synchronized_statement" => {
                if let Some(kw) = find_keyword(node, "synchronized") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "try_statement" => {
                if let Some(kw) = find_keyword(node, "try") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "try_with_resources_statement" => {
                if let Some(kw) = find_keyword(node, "try") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "catch_clause" => {
                if let Some(kw) = find_keyword(node, "catch") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "finally_clause" => {
                if let Some(kw) = find_keyword(node, "finally") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
            }
            "return_statement" => {
                diagnostics.extend(self.check_return(ctx, node));
            }
            "assert_statement" => {
                if let Some(kw) = find_keyword(node, "assert") {
                    diagnostics.extend(check_whitespace_after(ctx, &kw));
                }
                // Also check colon in assert with message: assert cond : "message"
                if let Some(colon) = find_child_by_kind(node, ":") {
                    diagnostics.extend(check_whitespace_around(ctx, &colon));
                }
            }

            // Block/braces - check { and }
            "block" => {
                diagnostics.extend(self.check_block(ctx, node));
            }
            "constructor_body" => {
                diagnostics.extend(self.check_constructor_body(ctx, node));
            }
            "class_body" | "interface_body" | "enum_body" | "annotation_type_body" => {
                diagnostics.extend(self.check_type_body(ctx, node));
            }

            // Lambda expression
            "lambda_expression" => {
                diagnostics.extend(self.check_lambda(ctx, node));
            }

            // Spread parameter (varargs): String... args
            // Note: ELLIPSIS is NOT in checkstyle's default tokens for WhitespaceAround,
            // so we don't check whitespace around "..." by default.
            // "spread_parameter" => { ... }

            // Type bounds with & (e.g., T extends A & B)
            "type_bound" => {
                for child in node.children() {
                    if child.kind() == "&" {
                        diagnostics.extend(check_whitespace_around(ctx, &child));
                    }
                }
            }

            // Guard clause with 'when' keyword (Java 21 pattern matching)
            // e.g., case Integer i when (i > 0) -> {}
            "guard" => {
                if let Some(when_kw) = find_keyword(node, "when") {
                    diagnostics.extend(check_whitespace_around(ctx, &when_kw));
                }
            }

            // Generic type arguments: List<String>, Map<K, V>
            // Only checked when check_generic_start/check_generic_end are enabled
            "type_arguments" => {
                for child in node.children() {
                    match child.kind() {
                        "<" if self.check_generic_start => {
                            diagnostics.extend(check_whitespace_around(ctx, &child));
                        }
                        ">" if self.check_generic_end => {
                            diagnostics.extend(check_whitespace_around(ctx, &child));
                        }
                        _ => {}
                    }
                }
            }

            // Wildcard type: <?>, <? extends T>, <? super T>
            // Only checked when check_wildcard_type is enabled
            "wildcard" => {
                if self.check_wildcard_type
                    && let Some(question) = find_child_by_kind(node, "?")
                {
                    diagnostics.extend(check_whitespace_around(ctx, &question));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

impl WhitespaceAround {
    /// Check ternary expression: a ? b : c
    fn check_ternary(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        for child in node.children() {
            if child.kind() == "?" || child.kind() == ":" {
                diagnostics.extend(check_whitespace_around(ctx, &child));
            }
        }

        diagnostics
    }

    /// Check return statement - only if it has an expression
    fn check_return(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // return; - no check needed
        // return expr; - check whitespace after 'return'
        let has_expression = node.children().any(|c| !matches!(c.kind(), "return" | ";"));

        if has_expression && let Some(kw) = find_keyword(node, "return") {
            return check_whitespace_after(ctx, &kw);
        }

        vec![]
    }

    /// Check a block for brace whitespace
    fn check_block(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        let is_empty = is_empty_block(node);
        let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("");

        // Determine if this empty block should be allowed
        let allow_empty = match parent_kind {
            "method_declaration" => self.allow_empty_methods,
            "constructor_declaration" => self.allow_empty_constructors,
            "while_statement" | "for_statement" | "enhanced_for_statement" | "do_statement" => {
                self.allow_empty_loops
            }
            "lambda_expression" => self.allow_empty_lambdas,
            "catch_clause" => self.allow_empty_catches,
            _ => false,
        };

        if is_empty && allow_empty {
            return vec![];
        }

        // Check opening brace
        if let Some(open) = find_child_by_kind(node, "{") {
            diagnostics.extend(check_brace_whitespace(ctx, &open, is_empty));
        }

        // Check closing brace
        if let Some(close) = find_child_by_kind(node, "}") {
            diagnostics.extend(check_closing_brace_whitespace(ctx, &close, is_empty, node));
        }

        diagnostics
    }

    /// Check constructor body for brace whitespace
    fn check_constructor_body(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let is_empty = is_empty_block(node);

        if is_empty && self.allow_empty_constructors {
            return vec![];
        }

        let mut diagnostics = vec![];

        if let Some(open) = find_child_by_kind(node, "{") {
            diagnostics.extend(check_brace_whitespace(ctx, &open, is_empty));
        }

        if let Some(close) = find_child_by_kind(node, "}") {
            diagnostics.extend(check_closing_brace_whitespace(ctx, &close, is_empty, node));
        }

        diagnostics
    }

    /// Check type body (class/interface/enum) for brace whitespace
    fn check_type_body(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let is_empty = is_empty_type_body(node);

        if is_empty && self.allow_empty_types {
            return vec![];
        }

        let mut diagnostics = vec![];

        if let Some(open) = find_child_by_kind(node, "{") {
            diagnostics.extend(check_brace_whitespace(ctx, &open, is_empty));
        }

        if let Some(close) = find_child_by_kind(node, "}") {
            diagnostics.extend(check_closing_brace_whitespace(ctx, &close, is_empty, node));
        }

        diagnostics
    }

    /// Check lambda expression for arrow and braces
    fn check_lambda(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check arrow ->
        if let Some(arrow) = find_child_by_kind(node, "->") {
            diagnostics.extend(check_whitespace_around(ctx, &arrow));
        }

        // Check block body if present
        if let Some(body) = node.child_by_field_name("body")
            && body.kind() == "block"
        {
            let is_empty = is_empty_block(&body);

            if !(is_empty && self.allow_empty_lambdas) {
                if let Some(open) = find_child_by_kind(&body, "{") {
                    diagnostics.extend(check_brace_whitespace(ctx, &open, is_empty));
                }

                if let Some(close) = find_child_by_kind(&body, "}") {
                    diagnostics
                        .extend(check_closing_brace_whitespace(ctx, &close, is_empty, &body));
                }
            }
        }

        diagnostics
    }
}

/// Check whitespace around a token and return any violations.
fn check_whitespace_around(ctx: &CheckContext, token: &CstNode) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];
    diagnostics.extend(check_whitespace_before(ctx, token));
    diagnostics.extend(check_whitespace_after(ctx, token));
    diagnostics
}

/// Check whitespace before a token.
fn check_whitespace_before(ctx: &CheckContext, token: &CstNode) -> Vec<Diagnostic> {
    let range = token.range();
    let text = token.text();
    let before_pos = range.start();

    if before_pos > TextSize::new(0) {
        let char_before = ctx
            .source()
            .get(usize::from(before_pos) - 1..usize::from(before_pos))
            .unwrap_or("");
        if !char_before.chars().next().is_some_and(char::is_whitespace) {
            return vec![
                Diagnostic::new(
                    MissingWhitespaceBefore {
                        token: text.to_string(),
                    },
                    range,
                )
                .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), before_pos))),
            ];
        }
    }

    vec![]
}

/// Check whitespace after a token.
fn check_whitespace_after(ctx: &CheckContext, token: &CstNode) -> Vec<Diagnostic> {
    let range = token.range();
    let text = token.text();
    let after_pos = range.end();

    let char_after = ctx
        .source()
        .get(usize::from(after_pos)..usize::from(after_pos) + 1)
        .unwrap_or("");

    if !char_after.chars().next().is_some_and(char::is_whitespace) {
        return vec![
            Diagnostic::new(
                MissingWhitespaceAfter {
                    token: text.to_string(),
                },
                range,
            )
            .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), after_pos))),
        ];
    }

    vec![]
}

/// Check whitespace for opening brace.
fn check_brace_whitespace(ctx: &CheckContext, brace: &CstNode, is_empty: bool) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    // Check whitespace before {
    diagnostics.extend(check_whitespace_before(ctx, brace));

    // Check whitespace after { (unless empty block, then we check differently)
    if !is_empty {
        diagnostics.extend(check_whitespace_after(ctx, brace));
    } else {
        // For empty blocks, { must be followed by whitespace
        // (allowEmpty* already handled by early return in check_block)
        let after_pos = brace.range().end();
        let char_after = ctx
            .source()
            .get(usize::from(after_pos)..usize::from(after_pos) + 1)
            .unwrap_or("");

        if let Some(c) = char_after.chars().next()
            && !c.is_whitespace()
        {
            diagnostics.push(
                Diagnostic::new(
                    MissingWhitespaceAfter {
                        token: "{".to_string(),
                    },
                    brace.range(),
                )
                .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), after_pos))),
            );
        }
    }

    diagnostics
}

/// Check whitespace for closing brace.
fn check_closing_brace_whitespace(
    ctx: &CheckContext,
    brace: &CstNode,
    is_empty: bool,
    _parent: &CstNode,
) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    // Check whitespace before } (unless empty block)
    if !is_empty {
        diagnostics.extend(check_whitespace_before(ctx, brace));
    } else {
        // For empty blocks, } must be preceded by whitespace
        // (allowEmpty* already handled by early return in check_block)
        let before_pos = brace.range().start();
        if before_pos > TextSize::new(0) {
            let char_before = ctx
                .source()
                .get(usize::from(before_pos) - 1..usize::from(before_pos))
                .unwrap_or("");

            if let Some(c) = char_before.chars().next()
                && !c.is_whitespace()
            {
                diagnostics.push(
                    Diagnostic::new(
                        MissingWhitespaceBefore {
                            token: "}".to_string(),
                        },
                        brace.range(),
                    )
                    .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), before_pos))),
                );
            }
        }
    }

    // Check whitespace after } - but not for certain contexts
    // (e.g., }); or },  or }. are OK for anonymous inner classes)
    let after_pos = brace.range().end();
    let char_after = ctx
        .source()
        .get(usize::from(after_pos)..usize::from(after_pos) + 1)
        .unwrap_or("");

    if let Some(c) = char_after.chars().next() {
        // These are OK after }: ) ; , .
        if !c.is_whitespace() && !matches!(c, ')' | ';' | ',' | '.') {
            diagnostics.push(
                Diagnostic::new(
                    MissingWhitespaceAfter {
                        token: "}".to_string(),
                    },
                    brace.range(),
                )
                .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), after_pos))),
            );
        }
    }

    diagnostics
}

/// Find a specific keyword in a node's children.
fn find_keyword<'a>(node: &CstNode<'a>, keyword: &str) -> Option<CstNode<'a>> {
    node.children().find(|c| c.kind() == keyword)
}

/// Find a child by its kind.
fn find_child_by_kind<'a>(node: &CstNode<'a>, kind: &str) -> Option<CstNode<'a>> {
    node.children().find(|c| c.kind() == kind)
}

/// Check if a block is empty (only contains { and }).
fn is_empty_block(node: &CstNode) -> bool {
    let children: Vec<_> = node.children().collect();
    // Empty block has exactly 2 children: { and }
    children.len() == 2
        && children.first().map(|c| c.kind()) == Some("{")
        && children.last().map(|c| c.kind()) == Some("}")
}

/// Check if a type body is empty.
fn is_empty_type_body(node: &CstNode) -> bool {
    let children: Vec<_> = node.children().collect();
    children.len() == 2
        && children.first().map(|c| c.kind()) == Some("{")
        && children.last().map(|c| c.kind()) == Some("}")
}

/// Find the operator node in a binary expression.
fn find_binary_operator<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
    for child in node.children() {
        let kind = child.kind();
        if matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "=="
                | "!="
                | "<"
                | ">"
                | "<="
                | ">="
                | "&&"
                | "||"
                | "&"
                | "|"
                | "^"
                | "<<"
                | ">>"
                | ">>>"
                | "instanceof"
        ) {
            return Some(child);
        }
    }
    None
}

/// Find the operator node in an assignment expression.
fn find_assignment_operator<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
    for child in node.children() {
        let kind = child.kind();
        if matches!(
            kind,
            "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "&=" | "|=" | "^=" | "<<=" | ">>=" | ">>>="
        ) {
            return Some(child);
        }
    }
    None
}

/// Find the = in a variable declarator with initializer.
fn find_equals_in_declarator<'a>(node: &CstNode<'a>) -> Option<CstNode<'a>> {
    node.children().find(|c| c.kind() == "=")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let rule = WhitespaceAround::default();
        let ctx = CheckContext::new(source);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_binary_operator() {
        let diagnostics = check_source("class Foo { int x = 1+2; }");
        assert!(diagnostics.len() >= 2); // Missing before and after +
    }

    #[test]
    fn test_assignment_operator() {
        let diagnostics = check_source("class Foo { void m() { int x; x=1; } }");
        assert!(diagnostics.len() >= 2); // Missing before and after =
    }

    #[test]
    fn test_if_keyword() {
        let diagnostics = check_source("class Foo { void m() { if(true) {} } }");
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("if")));
    }

    #[test]
    fn test_synchronized_keyword() {
        let diagnostics = check_source("class Foo { void m() { synchronized(this) {} } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("synchronized"))
        );
    }

    #[test]
    fn test_try_catch() {
        let diagnostics = check_source("class Foo { void m() { try{ } catch(Exception e){ } } }");
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("try")));
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("catch")));
    }

    #[test]
    fn test_ternary_operator() {
        let diagnostics = check_source("class Foo { int x = true?1:2; }");
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("?")));
        assert!(diagnostics.iter().any(|d| d.kind.body.contains(":")));
    }

    #[test]
    fn test_return_with_value() {
        let diagnostics = check_source("class Foo { int m() { return(1); } }");
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("return")));
    }

    #[test]
    fn test_return_without_value() {
        // return; should not trigger a violation
        let diagnostics = check_source("class Foo { void m() { return; } }");
        assert!(!diagnostics.iter().any(|d| d.kind.body.contains("return")));
    }

    #[test]
    fn test_lambda_arrow() {
        let diagnostics = check_source("class Foo { Runnable r = ()->{}; }");
        assert!(diagnostics.iter().any(|d| d.kind.body.contains("->")));
    }

    #[test]
    fn test_fix_inserts_space_before() {
        let diagnostics = check_source("class Foo { int x = 1+2; }");
        let plus_before = diagnostics
            .iter()
            .find(|d| d.kind.body.contains("before") && d.kind.body.contains("+"))
            .expect("Should have 'before +' diagnostic");

        let fix = plus_before.fix.as_ref().expect("Should have fix");
        let edits = fix.edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].content(), Some(" "));
        assert!(edits[0].is_insertion());
    }

    #[test]
    fn test_fix_inserts_space_after() {
        let diagnostics = check_source("class Foo { int x = 1+2; }");
        let plus_after = diagnostics
            .iter()
            .find(|d| d.kind.body.contains("after") && d.kind.body.contains("+"))
            .expect("Should have 'after +' diagnostic");

        let fix = plus_after.fix.as_ref().expect("Should have fix");
        let edits = fix.edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].content(), Some(" "));
        assert!(edits[0].is_insertion());
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { void m() { if(true){ }else{ } } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic '{}' should have a fix",
                d.kind.body
            );
        }
    }
}
