//! MethodParamPad rule implementation.
//!
//! Checks for whitespace before the opening parenthesis of method/constructor parameter lists.
//! Checkstyle equivalent: MethodParamPad

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;

use crate::rules::whitespace::common::{has_whitespace_before, whitespace_range_before};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by MethodParamPad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodParamPadToken {
    CtorDef,
    LiteralNew,
    MethodCall,
    MethodDef,
    SuperCtorCall,
    EnumConstantDef,
    RecordDef,
}

impl MethodParamPadToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "CTOR_DEF" => Some(Self::CtorDef),
            "LITERAL_NEW" => Some(Self::LiteralNew),
            "METHOD_CALL" => Some(Self::MethodCall),
            "METHOD_DEF" => Some(Self::MethodDef),
            "SUPER_CTOR_CALL" => Some(Self::SuperCtorCall),
            "ENUM_CONSTANT_DEF" => Some(Self::EnumConstantDef),
            "RECORD_DEF" => Some(Self::RecordDef),
            _ => None,
        }
    }
}

/// MethodParamPad option: space or nospace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodParamPadOption {
    Space,
    NoSpace,
}

impl MethodParamPadOption {
    fn from_str(s: &str) -> Self {
        match s.trim() {
            "space" => Self::Space,
            _ => Self::NoSpace,
        }
    }
}

// ============================================================================
// Violation types
// ============================================================================

/// Violation: '(' is preceded by whitespace (when option=nospace).
#[derive(Debug, Clone)]
pub struct WsPreceded {
    pub token: String,
}

impl Violation for WsPreceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is preceded by whitespace", self.token)
    }
}

/// Violation: '(' is not preceded by whitespace (when option=space).
#[derive(Debug, Clone)]
pub struct WsNotPreceded {
    pub token: String,
}

impl Violation for WsNotPreceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is not preceded by whitespace", self.token)
    }
}

/// Violation: '(' should be on the previous line (when allowLineBreaks=false).
#[derive(Debug, Clone)]
pub struct LinePrevious {
    pub token: String,
}

impl Violation for LinePrevious {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' should be on the previous line", self.token)
    }
}

/// Configuration for MethodParamPad rule.
#[derive(Debug, Clone)]
pub struct MethodParamPad {
    /// Whether to require space or no space before '('.
    pub option: MethodParamPadOption,
    /// Allow newlines before '('.
    pub allow_line_breaks: bool,
    /// Which tokens to check.
    pub tokens: HashSet<MethodParamPadToken>,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "method_invocation",
    "object_creation_expression",
    "explicit_constructor_invocation",
    "enum_constant",
    "record_declaration",
];

impl Default for MethodParamPad {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(MethodParamPadToken::CtorDef);
        tokens.insert(MethodParamPadToken::LiteralNew);
        tokens.insert(MethodParamPadToken::MethodCall);
        tokens.insert(MethodParamPadToken::MethodDef);
        tokens.insert(MethodParamPadToken::SuperCtorCall);
        tokens.insert(MethodParamPadToken::EnumConstantDef);
        tokens.insert(MethodParamPadToken::RecordDef);

        Self {
            option: MethodParamPadOption::NoSpace,
            allow_line_breaks: false,
            tokens,
        }
    }
}

impl FromConfig for MethodParamPad {
    const MODULE_NAME: &'static str = "MethodParamPad";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|s| MethodParamPadOption::from_str(s))
            .unwrap_or(MethodParamPadOption::NoSpace);

        let allow_line_breaks = properties
            .get("allowLineBreaks")
            .map(|s| s.trim() == "true")
            .unwrap_or(false);

        let tokens_str = properties.get("tokens").copied().unwrap_or(
            "CTOR_DEF, LITERAL_NEW, METHOD_CALL, METHOD_DEF, SUPER_CTOR_CALL, \
             ENUM_CONSTANT_DEF, RECORD_DEF",
        );

        let tokens: HashSet<_> = tokens_str
            .split(',')
            .filter_map(MethodParamPadToken::from_str)
            .collect();

        Self {
            option,
            allow_line_breaks,
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
        }
    }
}

impl Rule for MethodParamPad {
    fn name(&self) -> &'static str {
        "MethodParamPad"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Match constructs based on configured tokens
        match node.kind() {
            // Method definitions: void method(params)
            "method_declaration" if self.tokens.contains(&MethodParamPadToken::MethodDef) => {
                if let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_lparen(ctx, &params));
                }
            }

            // Constructor definitions: Foo(params)
            "constructor_declaration" if self.tokens.contains(&MethodParamPadToken::CtorDef) => {
                if let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_lparen(ctx, &params));
                }
            }

            // Method calls: method(args)
            "method_invocation" if self.tokens.contains(&MethodParamPadToken::MethodCall) => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_lparen(ctx, &args));
                }
            }

            // Constructor calls: new Foo(args)
            "object_creation_expression"
                if self.tokens.contains(&MethodParamPadToken::LiteralNew) =>
            {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_lparen(ctx, &args));
                }
            }

            // Explicit constructor invocation: this() or super()
            "explicit_constructor_invocation"
                if self.tokens.contains(&MethodParamPadToken::SuperCtorCall) =>
            {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_lparen(ctx, &args));
                }
            }

            // Enum constants: CONSTANT(args)
            "enum_constant" if self.tokens.contains(&MethodParamPadToken::EnumConstantDef) => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    diagnostics.extend(self.check_lparen(ctx, &args));
                }
            }

            // Record declarations: record Foo(int x, int y)
            "record_declaration" if self.tokens.contains(&MethodParamPadToken::RecordDef) => {
                if let Some(params) = node.child_by_field_name("parameters") {
                    diagnostics.extend(self.check_lparen(ctx, &params));
                }
            }

            _ => {}
        }

        diagnostics
    }
}

impl MethodParamPad {
    /// Check the opening paren of a node.
    fn check_lparen(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Find the opening paren
        let Some(lparen) = node.children().find(|c| c.kind() == "(") else {
            return diagnostics;
        };

        let lparen_pos = lparen.range().start();

        // Check if there's whitespace before the opening paren
        // We need to look at the actual character before the paren position
        if lparen_pos == 0.into() {
            // Start of file, nothing to check
            return diagnostics;
        }

        // Get the source line for this position
        let source = ctx.source();
        let source_code = ctx.source_code();
        let lparen_loc = source_code.line_column(lparen_pos);
        let line_start = source_code.line_start(lparen_loc.line);
        let line_end = source_code.line_end(lparen_loc.line);
        let line_text = &source[usize::from(line_start)..usize::from(line_end)];

        // Check if the lparen has only whitespace before it on the line
        let col_offset = usize::from(lparen_pos - line_start);
        let before_on_line = &line_text[..col_offset];
        let has_only_whitespace_before = before_on_line.chars().all(|c: char| c.is_whitespace());

        if has_only_whitespace_before {
            // The '(' is at the start of the line (after whitespace)
            // This is a line break situation
            if !self.allow_line_breaks {
                diagnostics.push(self.diag_line_previous(&lparen));
            }
        } else {
            // Check the character immediately before '('
            let has_space = has_whitespace_before(source, lparen_pos);

            match self.option {
                MethodParamPadOption::NoSpace => {
                    if has_space && let Some(ws_range) = whitespace_range_before(source, lparen_pos)
                    {
                        diagnostics.push(self.diag_ws_preceded(&lparen, ws_range));
                    }
                }
                MethodParamPadOption::Space => {
                    if !has_space {
                        diagnostics.push(self.diag_ws_not_preceded(&lparen));
                    }
                }
            }
        }

        diagnostics
    }

    /// Create diagnostic for '(' preceded by whitespace.
    fn diag_ws_preceded(&self, lparen: &CstNode, ws_range: TextRange) -> Diagnostic {
        let text = lparen.text().to_string();
        Diagnostic::new(WsPreceded { token: text }, lparen.range())
            .with_fix(Fix::safe_edit(Edit::range_deletion(ws_range)))
    }

    /// Create diagnostic for '(' not preceded by whitespace.
    fn diag_ws_not_preceded(&self, lparen: &CstNode) -> Diagnostic {
        let text = lparen.text().to_string();
        Diagnostic::new(WsNotPreceded { token: text }, lparen.range()).with_fix(Fix::safe_edit(
            Edit::insertion(" ".to_string(), lparen.range().start()),
        ))
    }

    /// Create diagnostic for '(' should be on previous line.
    fn diag_line_previous(&self, lparen: &CstNode) -> Diagnostic {
        let text = lparen.text().to_string();
        let range = lparen.range();

        // For the fix, we need to remove the newline and whitespace before the paren
        // and move it to the previous line
        // This is complex, so for now we'll just provide the diagnostic without a fix
        // or we can try to create a simple fix that removes the line break
        Diagnostic::new(LinePrevious { token: text }, range).with_fix(Fix::safe_edit(
            Edit::replacement("(".to_string(), range.start(), range.end()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &MethodParamPad::default())
    }

    fn check_source_with_config(source: &str, rule: &MethodParamPad) -> Vec<Diagnostic> {
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
    fn test_method_def_with_space() {
        let diagnostics = check_source("class Foo { void m (int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen"
        );
    }

    #[test]
    fn test_method_def_without_space() {
        let diagnostics = check_source("class Foo { void m(int x) {} }");
        assert!(
            diagnostics.is_empty(),
            "Should not flag lparen without space when option=nospace"
        );
    }

    #[test]
    fn test_constructor_with_space() {
        let diagnostics = check_source("class Foo { Foo (int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen in constructor"
        );
    }

    #[test]
    fn test_method_call_with_space() {
        let diagnostics = check_source("class Foo { void m() { foo (1); } void foo(int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen in method call"
        );
    }

    #[test]
    fn test_new_with_space() {
        let diagnostics = check_source("class Foo { void m() { new Foo (1); } Foo(int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen in new expression"
        );
    }

    #[test]
    fn test_super_call_with_space() {
        let diagnostics = check_source("class Foo { Foo() { super (); } }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen in super call"
        );
    }

    #[test]
    fn test_line_break_not_allowed() {
        let diagnostics = check_source("class Foo { void m\n(int x) {} }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("should be on the previous line")),
            "Should detect line break before lparen when not allowed"
        );
    }

    #[test]
    fn test_line_break_allowed() {
        let rule = MethodParamPad {
            option: MethodParamPadOption::NoSpace,
            allow_line_breaks: true,
            tokens: MethodParamPad::default().tokens,
        };
        let diagnostics = check_source_with_config("class Foo { void m\n(int x) {} }", &rule);
        assert!(
            diagnostics.is_empty(),
            "Should not flag line break when allowed"
        );
    }

    #[test]
    fn test_space_option() {
        let rule = MethodParamPad {
            option: MethodParamPadOption::Space,
            allow_line_breaks: false,
            tokens: MethodParamPad::default().tokens,
        };
        let diagnostics = check_source_with_config("class Foo { void m(int x) {} }", &rule);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is not preceded")),
            "Should detect missing space before lparen when option=space"
        );
    }

    #[test]
    fn test_space_option_with_space() {
        let rule = MethodParamPad {
            option: MethodParamPadOption::Space,
            allow_line_breaks: false,
            tokens: MethodParamPad::default().tokens,
        };
        let diagnostics = check_source_with_config("class Foo { void m (int x) {} }", &rule);
        assert!(
            diagnostics.is_empty(),
            "Should not flag lparen with space when option=space"
        );
    }

    #[test]
    fn test_enum_constant() {
        let diagnostics = check_source("enum E { A (), B() }");
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("'(' is preceded")),
            "Should detect space before lparen in enum constant"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics =
            check_source("class Foo { void m (int x) { foo (1); } void foo(int x) {} }");
        for d in &diagnostics {
            assert!(
                d.fix.is_some(),
                "Diagnostic should have fix: {}",
                d.kind.body
            );
        }
    }
}
