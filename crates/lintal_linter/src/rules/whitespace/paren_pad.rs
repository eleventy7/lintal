//! ParenPad rule implementation.
//!
//! Checks for whitespace padding inside parentheses.
//! Checkstyle equivalent: ParenPad

use std::collections::HashSet;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::rules::whitespace::common::{
    char_after, char_before, diag_followed, diag_not_followed, diag_not_preceded, diag_preceded,
    has_whitespace_after, has_whitespace_before, whitespace_range_after, whitespace_range_before,
};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by ParenPad.
///
/// Note: Checkstyle's DOT token is not included here because tree-sitter doesn't have
/// a "DOT" node type. Method chains like `obj.method(args)` are represented as
/// `method_invocation` nodes, which are already handled by the MethodCall token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParenPadToken {
    Annotation,
    AnnotationFieldDef,
    CtorCall,
    CtorDef,
    EnumConstantDef,
    Expr,
    LiteralCatch,
    LiteralDo,
    LiteralFor,
    LiteralIf,
    LiteralNew,
    LiteralSwitch,
    LiteralSynchronized,
    LiteralWhile,
    MethodCall,
    MethodDef,
    Question,
    ResourceSpecification,
    SuperCtorCall,
    Lambda,
    RecordDef,
}

impl ParenPadToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "ANNOTATION" => Some(Self::Annotation),
            "ANNOTATION_FIELD_DEF" => Some(Self::AnnotationFieldDef),
            "CTOR_CALL" => Some(Self::CtorCall),
            "CTOR_DEF" => Some(Self::CtorDef),
            "ENUM_CONSTANT_DEF" => Some(Self::EnumConstantDef),
            "EXPR" => Some(Self::Expr),
            "LITERAL_CATCH" => Some(Self::LiteralCatch),
            "LITERAL_DO" => Some(Self::LiteralDo),
            "LITERAL_FOR" => Some(Self::LiteralFor),
            "LITERAL_IF" => Some(Self::LiteralIf),
            "LITERAL_NEW" => Some(Self::LiteralNew),
            "LITERAL_SWITCH" => Some(Self::LiteralSwitch),
            "LITERAL_SYNCHRONIZED" => Some(Self::LiteralSynchronized),
            "LITERAL_WHILE" => Some(Self::LiteralWhile),
            "METHOD_CALL" => Some(Self::MethodCall),
            "METHOD_DEF" => Some(Self::MethodDef),
            "QUESTION" => Some(Self::Question),
            "RESOURCE_SPECIFICATION" => Some(Self::ResourceSpecification),
            "SUPER_CTOR_CALL" => Some(Self::SuperCtorCall),
            "LAMBDA" => Some(Self::Lambda),
            "RECORD_DEF" => Some(Self::RecordDef),
            _ => None,
        }
    }
}

/// ParenPad option: space or nospace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParenPadOption {
    Space,
    NoSpace,
}

impl ParenPadOption {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "space" => Self::Space,
            _ => Self::NoSpace,
        }
    }
}

/// Configuration for ParenPad rule.
#[derive(Debug, Clone)]
pub struct ParenPad {
    /// Whether to require space or no space inside parens.
    pub option: ParenPadOption,
    /// Which tokens to check.
    pub tokens: HashSet<ParenPadToken>,
}

impl Default for ParenPad {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(ParenPadToken::Annotation);
        tokens.insert(ParenPadToken::AnnotationFieldDef);
        tokens.insert(ParenPadToken::CtorCall);
        tokens.insert(ParenPadToken::CtorDef);
        tokens.insert(ParenPadToken::EnumConstantDef);
        tokens.insert(ParenPadToken::Expr);
        tokens.insert(ParenPadToken::LiteralCatch);
        tokens.insert(ParenPadToken::LiteralDo);
        tokens.insert(ParenPadToken::LiteralFor);
        tokens.insert(ParenPadToken::LiteralIf);
        tokens.insert(ParenPadToken::LiteralNew);
        tokens.insert(ParenPadToken::LiteralSwitch);
        tokens.insert(ParenPadToken::LiteralSynchronized);
        tokens.insert(ParenPadToken::LiteralWhile);
        tokens.insert(ParenPadToken::MethodCall);
        tokens.insert(ParenPadToken::MethodDef);
        tokens.insert(ParenPadToken::Question);
        tokens.insert(ParenPadToken::ResourceSpecification);
        tokens.insert(ParenPadToken::SuperCtorCall);
        tokens.insert(ParenPadToken::Lambda);
        tokens.insert(ParenPadToken::RecordDef);

        Self {
            option: ParenPadOption::NoSpace,
            tokens,
        }
    }
}

impl FromConfig for ParenPad {
    const MODULE_NAME: &'static str = "ParenPad";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|s| ParenPadOption::from_str(s))
            .unwrap_or(ParenPadOption::NoSpace);

        let tokens_str = properties.get("tokens").copied().unwrap_or(
            "ANNOTATION, ANNOTATION_FIELD_DEF, CTOR_CALL, CTOR_DEF, \
             ENUM_CONSTANT_DEF, EXPR, LITERAL_CATCH, LITERAL_DO, LITERAL_FOR, LITERAL_IF, \
             LITERAL_NEW, LITERAL_SWITCH, LITERAL_SYNCHRONIZED, LITERAL_WHILE, METHOD_CALL, \
             METHOD_DEF, QUESTION, RESOURCE_SPECIFICATION, SUPER_CTOR_CALL, LAMBDA, RECORD_DEF",
        );

        let tokens: HashSet<_> = tokens_str
            .split(',')
            .filter_map(ParenPadToken::from_str)
            .collect();

        Self {
            option,
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
        }
    }
}

impl Rule for ParenPad {
    fn name(&self) -> &'static str {
        "ParenPad"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Match constructs based on configured tokens
        match node.kind() {
            // Parenthesized expressions: (expr)
            "parenthesized_expression" if self.tokens.contains(&ParenPadToken::Expr) => {
                diagnostics.extend(self.check_parens(ctx, node));
            }

            // Method calls: method(args)
            "method_invocation" if self.tokens.contains(&ParenPadToken::MethodCall) => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_parens(ctx, &args));
                }
            }

            // Method definitions: void method(params)
            "method_declaration" | "constructor_declaration"
                if self.tokens.contains(&ParenPadToken::MethodDef)
                    || self.tokens.contains(&ParenPadToken::CtorDef) =>
            {
                let should_check = match node.kind() {
                    "constructor_declaration" => self.tokens.contains(&ParenPadToken::CtorDef),
                    "method_declaration" => self.tokens.contains(&ParenPadToken::MethodDef),
                    _ => false,
                };

                if should_check && let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_parens(ctx, &params));
                }
            }

            // If statements: if (condition)
            "if_statement" if self.tokens.contains(&ParenPadToken::LiteralIf) => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    diagnostics.extend(self.check_parens(ctx, &condition));
                }
            }

            // While loops: while (condition)
            "while_statement" if self.tokens.contains(&ParenPadToken::LiteralWhile) => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    diagnostics.extend(self.check_parens(ctx, &condition));
                }
            }

            // For loops: for (init; cond; update)
            "for_statement" if self.tokens.contains(&ParenPadToken::LiteralFor) => {
                // For statement has direct lparen/rparen children
                // But if the update section is empty (for(;;) or for(init;cond;)),
                // the whitespace before ) is controlled by EmptyForIteratorPad, not ParenPad
                let has_empty_iterator = node.child_by_field_name("update").is_none();
                diagnostics.extend(self.check_for_parens(ctx, node, has_empty_iterator));
            }

            // Enhanced for: for (Type item : collection)
            "enhanced_for_statement" if self.tokens.contains(&ParenPadToken::LiteralFor) => {
                diagnostics.extend(self.check_parens(ctx, node));
            }

            // Do-while: do {...} while (condition)
            "do_statement" if self.tokens.contains(&ParenPadToken::LiteralDo) => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    diagnostics.extend(self.check_parens(ctx, &condition));
                }
            }

            // Switch: switch (expr)
            "switch_expression" | "switch_statement"
                if self.tokens.contains(&ParenPadToken::LiteralSwitch) =>
            {
                if let Some(condition) = node.child_by_field_name("condition") {
                    diagnostics.extend(self.check_parens(ctx, &condition));
                }
            }

            // Synchronized: synchronized (obj)
            "synchronized_statement"
                if self.tokens.contains(&ParenPadToken::LiteralSynchronized) =>
            {
                diagnostics.extend(self.check_parens(ctx, node));
            }

            // Catch: catch (Exception e)
            "catch_clause" if self.tokens.contains(&ParenPadToken::LiteralCatch) => {
                if let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_parens(ctx, &params));
                }
            }

            // Constructor calls: new Foo() or this() or super()
            "object_creation_expression"
                if self.tokens.contains(&ParenPadToken::LiteralNew)
                    || self.tokens.contains(&ParenPadToken::CtorCall) =>
            {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_parens(ctx, &args));
                }
            }

            // Explicit constructor invocation: this() or super()
            "explicit_constructor_invocation"
                if self.tokens.contains(&ParenPadToken::CtorCall)
                    || self.tokens.contains(&ParenPadToken::SuperCtorCall) =>
            {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_parens(ctx, &args));
                }
            }

            // Ternary operator: condition ? true_val : false_val
            "ternary_expression" if self.tokens.contains(&ParenPadToken::Question) => {
                // Ternary itself doesn't have parens, but check any parenthesized parts
                // The checkstyle QUESTION token checks parens around ternary conditions
                if let Some(condition) = node.child_by_field_name("condition")
                    && condition.kind() == "parenthesized_expression"
                {
                    diagnostics.extend(self.check_parens(ctx, &condition));
                }
            }

            // Try-with-resources: try (Resource r = ...)
            "try_with_resources_statement"
                if self.tokens.contains(&ParenPadToken::ResourceSpecification) =>
            {
                if let Some(resources) = node.child_by_field_name("resources") {
                    diagnostics.extend(self.check_parens(ctx, &resources));
                }
            }

            // Lambda expressions: (param) -> body
            "lambda_expression" if self.tokens.contains(&ParenPadToken::Lambda) => {
                if let Some(params) = node.child_by_field_name("parameters")
                    && (params.kind() == "formal_parameters"
                        || params.kind() == "inferred_parameters")
                {
                    diagnostics.extend(self.check_parens(ctx, &params));
                }
            }

            // Record declarations: record Foo(int x, int y)
            "record_declaration" if self.tokens.contains(&ParenPadToken::RecordDef) => {
                if let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_parens(ctx, &params));
                }
            }

            // Compact constructor for records
            "compact_constructor_declaration"
                if self.tokens.contains(&ParenPadToken::RecordDef) =>
            {
                // Compact constructor doesn't have parens, so nothing to check
            }

            // Annotations: @Annotation(value)
            "annotation" | "marker_annotation" | "annotation_type_declaration"
                if self.tokens.contains(&ParenPadToken::Annotation)
                    || self.tokens.contains(&ParenPadToken::AnnotationFieldDef) =>
            {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_parens(ctx, &args));
                }
            }

            // Enum constants: CONSTANT(args)
            "enum_constant" if self.tokens.contains(&ParenPadToken::EnumConstantDef) => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_parens(ctx, &args));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

impl ParenPad {
    /// Check parens of a node that contains '(' and ')'.
    fn check_parens(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the opening and closing parens
        let lparen = node.children().find(|c| c.kind() == "(");
        let rparen = node.children().find(|c| c.kind() == ")");

        if let Some(lparen) = lparen {
            diagnostics.extend(self.check_lparen(ctx, &lparen));
        }

        if let Some(rparen) = rparen {
            diagnostics.extend(self.check_rparen(ctx, &rparen));
        }

        diagnostics
    }

    /// Check parens of a for statement, with special handling for empty iterator sections.
    fn check_for_parens(
        &self,
        ctx: &CheckContext,
        node: &CstNode,
        has_empty_iterator: bool,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the opening and closing parens
        let lparen = node.children().find(|c| c.kind() == "(");
        let rparen = node.children().find(|c| c.kind() == ")");

        if let Some(lparen) = lparen {
            diagnostics.extend(self.check_lparen(ctx, &lparen));
        }

        // For the closing paren, skip if the for-iterator is empty
        // (EmptyForIteratorPad handles the whitespace in that case)
        if !has_empty_iterator && let Some(rparen) = rparen {
            diagnostics.extend(self.check_rparen(ctx, &rparen));
        }

        diagnostics
    }

    /// Check whitespace after opening paren.
    fn check_lparen(&self, ctx: &CheckContext, lparen: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let after_pos = lparen.range().end();

        // Check if the next char is a closing paren (empty parens)
        if let Some(next_char) = char_after(ctx.source(), after_pos) {
            if next_char == ')' {
                // Empty parens, don't check
                return diagnostics;
            }
            if next_char == '\n' {
                // Multi-line - checkstyle doesn't flag these
                return diagnostics;
            }
        }

        let has_space = has_whitespace_after(ctx.source(), after_pos);

        match self.option {
            ParenPadOption::NoSpace => {
                if has_space && let Some(ws_range) = whitespace_range_after(ctx.source(), after_pos)
                {
                    diagnostics.push(diag_followed(lparen, ws_range));
                }
            }
            ParenPadOption::Space => {
                if !has_space {
                    diagnostics.push(diag_not_followed(lparen));
                }
            }
        }

        diagnostics
    }

    /// Check whitespace before closing paren.
    fn check_rparen(&self, ctx: &CheckContext, rparen: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let before_pos = rparen.range().start();

        // Check if the previous char is an opening paren (empty parens)
        if let Some(prev_char) = char_before(ctx.source(), before_pos)
            && prev_char == '('
        {
            // Empty parens, don't check
            return diagnostics;
        }

        // Check if there's a newline in the whitespace before ) (multi-line expression)
        if let Some(ws_range) = whitespace_range_before(ctx.source(), before_pos) {
            let ws_text = &ctx.source()[ws_range];
            if ws_text.contains('\n') {
                // Multi-line - checkstyle doesn't flag these
                return diagnostics;
            }
        }

        let has_space = has_whitespace_before(ctx.source(), before_pos);

        match self.option {
            ParenPadOption::NoSpace => {
                if has_space
                    && let Some(ws_range) = whitespace_range_before(ctx.source(), before_pos)
                {
                    diagnostics.push(diag_preceded(rparen, ws_range));
                }
            }
            ParenPadOption::Space => {
                if !has_space {
                    diagnostics.push(diag_not_preceded(rparen));
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &ParenPad::default())
    }

    fn check_source_with_config(source: &str, rule: &ParenPad) -> Vec<Diagnostic> {
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
    fn test_nospace_with_space() {
        let diagnostics = check_source("class Foo { void m( int x ) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is followed")),
            "Should detect space after lparen"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("')' is preceded")),
            "Should detect space before rparen"
        );
    }

    #[test]
    fn test_nospace_without_space() {
        let diagnostics = check_source("class Foo { void m(int x) {} }");
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("'('") || d.kind.body.contains("')'"))
            .collect();
        assert!(
            paren_violations.is_empty(),
            "Should not flag parens without space"
        );
    }

    #[test]
    fn test_space_without_space() {
        let rule = ParenPad {
            option: ParenPadOption::Space,
            tokens: ParenPad::default().tokens,
        };
        let diagnostics = check_source_with_config("class Foo { void m(int x) {} }", &rule);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is not followed")),
            "Should detect missing space after lparen"
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("')' is not preceded")),
            "Should detect missing space before rparen"
        );
    }

    #[test]
    fn test_space_with_space() {
        let rule = ParenPad {
            option: ParenPadOption::Space,
            tokens: ParenPad::default().tokens,
        };
        let diagnostics = check_source_with_config("class Foo { void m( int x ) {} }", &rule);
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("'('") || d.kind.body.contains("')'"))
            .collect();
        assert!(
            paren_violations.is_empty(),
            "Should not flag parens with space when option=space"
        );
    }

    #[test]
    fn test_empty_parens() {
        let diagnostics = check_source("class Foo { void m() {} }");
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("'('") || d.kind.body.contains("')'"))
            .collect();
        assert!(paren_violations.is_empty(), "Should not flag empty parens");
    }

    #[test]
    fn test_if_statement() {
        let diagnostics = check_source("class Foo { void m() { if( true ) {} } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is followed")),
            "Should detect space after lparen in if"
        );
    }

    #[test]
    fn test_method_call() {
        let diagnostics = check_source("class Foo { void m() { foo( 1 ); } void foo(int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is followed")),
            "Should detect space after lparen in method call"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { void m( int x ) { if( true ) {} } }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }

    #[test]
    fn test_for_empty_iterator() {
        // For loops with empty update section should NOT be flagged by ParenPad
        // The space before ) is controlled by EmptyForIteratorPad
        let diagnostics = check_source("class Foo { void m() { for (int i = 0; i < 10; ) { } } }");
        let paren_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains("')'") && d.kind.body.contains("preceded"))
            .collect();

        assert!(
            paren_violations.is_empty(),
            "Should not flag ) in for loop with empty iterator. Found: {:?}",
            paren_violations
                .iter()
                .map(|d| &d.kind.body)
                .collect::<Vec<_>>()
        );
    }
}
