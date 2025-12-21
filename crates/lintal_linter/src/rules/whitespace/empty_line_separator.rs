//! EmptyLineSeparator rule implementation.
//!
//! Checks that class members are separated by empty lines.
//!
//! Checkstyle equivalent: EmptyLineSeparatorCheck

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: element should be separated from previous line.
#[derive(Debug, Clone)]
pub struct ShouldBeSeparated {
    pub element: String,
}

impl Violation for ShouldBeSeparated {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be separated from previous line.", self.element)
    }
}

/// Violation: element has too many empty lines before it.
#[derive(Debug, Clone)]
pub struct TooManyEmptyLines {
    pub element: String,
}

impl Violation for TooManyEmptyLines {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' has more than 1 empty lines before.", self.element)
    }
}

/// Token types that can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmptyLineSeparatorToken {
    PackageDef,
    Import,
    StaticImport,
    ClassDef,
    InterfaceDef,
    EnumDef,
    StaticInit,
    InstanceInit,
    MethodDef,
    CtorDef,
    VariableDef,
    RecordDef,
    CompactCtorDef,
}

impl EmptyLineSeparatorToken {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "PACKAGE_DEF" => Some(Self::PackageDef),
            "IMPORT" => Some(Self::Import),
            "STATIC_IMPORT" => Some(Self::StaticImport),
            "CLASS_DEF" => Some(Self::ClassDef),
            "INTERFACE_DEF" => Some(Self::InterfaceDef),
            "ENUM_DEF" => Some(Self::EnumDef),
            "STATIC_INIT" => Some(Self::StaticInit),
            "INSTANCE_INIT" => Some(Self::InstanceInit),
            "METHOD_DEF" => Some(Self::MethodDef),
            "CTOR_DEF" => Some(Self::CtorDef),
            "VARIABLE_DEF" => Some(Self::VariableDef),
            "RECORD_DEF" => Some(Self::RecordDef),
            "COMPACT_CTOR_DEF" => Some(Self::CompactCtorDef),
            _ => None,
        }
    }

    fn to_checkstyle_name(self) -> &'static str {
        match self {
            Self::PackageDef => "PACKAGE_DEF",
            Self::Import => "IMPORT",
            Self::StaticImport => "STATIC_IMPORT",
            Self::ClassDef => "CLASS_DEF",
            Self::InterfaceDef => "INTERFACE_DEF",
            Self::EnumDef => "ENUM_DEF",
            Self::StaticInit => "STATIC_INIT",
            Self::InstanceInit => "INSTANCE_INIT",
            Self::MethodDef => "METHOD_DEF",
            Self::CtorDef => "CTOR_DEF",
            Self::VariableDef => "VARIABLE_DEF",
            Self::RecordDef => "RECORD_DEF",
            Self::CompactCtorDef => "COMPACT_CTOR_DEF",
        }
    }

    fn default_tokens() -> HashSet<Self> {
        [
            Self::PackageDef,
            Self::Import,
            Self::StaticImport,
            Self::ClassDef,
            Self::InterfaceDef,
            Self::EnumDef,
            Self::StaticInit,
            Self::InstanceInit,
            Self::MethodDef,
            Self::CtorDef,
            Self::VariableDef,
            Self::RecordDef,
            Self::CompactCtorDef,
        ]
        .into_iter()
        .collect()
    }
}

/// Configuration for EmptyLineSeparator rule.
#[derive(Debug, Clone)]
pub struct EmptyLineSeparator {
    pub allow_no_empty_line_between_fields: bool,
    pub allow_multiple_empty_lines: bool,
    pub allow_multiple_empty_lines_inside_class_members: bool,
    pub tokens: HashSet<EmptyLineSeparatorToken>,
}

impl Default for EmptyLineSeparator {
    fn default() -> Self {
        Self {
            allow_no_empty_line_between_fields: false,
            allow_multiple_empty_lines: true,
            allow_multiple_empty_lines_inside_class_members: true,
            tokens: EmptyLineSeparatorToken::default_tokens(),
        }
    }
}

impl FromConfig for EmptyLineSeparator {
    const MODULE_NAME: &'static str = "EmptyLineSeparator";

    fn from_config(properties: &Properties) -> Self {
        let allow_no_empty_line_between_fields = properties
            .get("allowNoEmptyLineBetweenFields")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let allow_multiple_empty_lines = properties
            .get("allowMultipleEmptyLines")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let allow_multiple_empty_lines_inside_class_members = properties
            .get("allowMultipleEmptyLinesInsideClassMembers")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let tokens = properties
            .get("tokens")
            .map(|v| {
                v.split(',')
                    .filter_map(|s| EmptyLineSeparatorToken::from_str(s.trim()))
                    .collect()
            })
            .unwrap_or_else(EmptyLineSeparatorToken::default_tokens);

        Self {
            allow_no_empty_line_between_fields,
            allow_multiple_empty_lines,
            allow_multiple_empty_lines_inside_class_members,
            tokens,
        }
    }
}

impl Rule for EmptyLineSeparator {
    fn name(&self) -> &'static str {
        "EmptyLineSeparator"
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();

        // Only process container bodies
        if kind != "class_body" && kind != "interface_body" && kind != "enum_body" {
            return vec![];
        }

        let ts_node = node.inner();
        let mut diagnostics = vec![];

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node.children(&mut cursor).collect();

        // Track previous non-comment element
        let mut prev_end_line: Option<usize> = None;
        let mut prev_was_field = false;

        for child in &children {
            // Skip braces and extra nodes
            if child.kind() == "{" || child.kind() == "}" || child.is_extra() {
                continue;
            }

            // Comments don't count as "previous" - they attach to next element
            if child.kind() == "line_comment" || child.kind() == "block_comment" {
                continue;
            }

            let token_type = self.node_to_token(child.kind());

            // Skip if this token type is not being checked
            if let Some(token) = token_type
                && !self.tokens.contains(&token)
            {
                // Still track it for prev_end_line
                prev_end_line = Some(child.end_position().row);
                prev_was_field = token == EmptyLineSeparatorToken::VariableDef;
                continue;
            }

            // Check if blank line is needed
            if let Some(prev_line) = prev_end_line {
                let current_start_line = self.find_start_line_before_comments(&children, child);
                let empty_lines = current_start_line.saturating_sub(prev_line + 1);

                // Check for field-to-field transition
                let is_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
                let field_to_field = prev_was_field && is_field;

                if empty_lines == 0 {
                    // Skip violation if field-to-field and allowed
                    if field_to_field && self.allow_no_empty_line_between_fields {
                        // OK, no violation
                    } else if let Some(token) = token_type {
                        let start = lintal_text_size::TextSize::from(child.start_byte() as u32);
                        let end = lintal_text_size::TextSize::from(child.start_byte() as u32 + 1);
                        diagnostics.push(Diagnostic::new(
                            ShouldBeSeparated {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            lintal_text_size::TextRange::new(start, end),
                        ));
                    }
                } else if empty_lines > 1 && !self.allow_multiple_empty_lines
                    && let Some(token) = token_type
                {
                    let start = lintal_text_size::TextSize::from(child.start_byte() as u32);
                    let end = lintal_text_size::TextSize::from(child.start_byte() as u32 + 1);
                    diagnostics.push(Diagnostic::new(
                        TooManyEmptyLines {
                            element: token.to_checkstyle_name().to_string(),
                        },
                        lintal_text_size::TextRange::new(start, end),
                    ));
                }
            }

            prev_end_line = Some(child.end_position().row);
            prev_was_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
        }

        diagnostics
    }
}

impl EmptyLineSeparator {
    fn node_to_token(&self, kind: &str) -> Option<EmptyLineSeparatorToken> {
        match kind {
            "package_declaration" => Some(EmptyLineSeparatorToken::PackageDef),
            "import_declaration" => Some(EmptyLineSeparatorToken::Import),
            "class_declaration" => Some(EmptyLineSeparatorToken::ClassDef),
            "interface_declaration" => Some(EmptyLineSeparatorToken::InterfaceDef),
            "enum_declaration" => Some(EmptyLineSeparatorToken::EnumDef),
            "static_initializer" => Some(EmptyLineSeparatorToken::StaticInit),
            "block" => Some(EmptyLineSeparatorToken::InstanceInit), // instance init block
            "method_declaration" => Some(EmptyLineSeparatorToken::MethodDef),
            "constructor_declaration" => Some(EmptyLineSeparatorToken::CtorDef),
            "field_declaration" => Some(EmptyLineSeparatorToken::VariableDef),
            "record_declaration" => Some(EmptyLineSeparatorToken::RecordDef),
            "compact_constructor_declaration" => Some(EmptyLineSeparatorToken::CompactCtorDef),
            _ => None,
        }
    }

    fn find_start_line_before_comments(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
    ) -> usize {
        // Find comments immediately before this node
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Walk backwards to find first comment in sequence before target
            let mut first_comment_line = target.start_position().row;
            for i in (0..idx).rev() {
                let prev = &children[i];
                if prev.kind() == "line_comment" || prev.kind() == "block_comment" {
                    first_comment_line = prev.start_position().row;
                } else if prev.kind() != "{" && prev.kind() != "}" && !prev.is_extra() {
                    break;
                }
            }
            first_comment_line
        } else {
            target.start_position().row
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = EmptyLineSeparator::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_with_config(source: &str, rule: EmptyLineSeparator) -> Vec<Diagnostic> {
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
    fn test_method_needs_blank_line() {
        let source = r#"
class Test {
    void method1() {}
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "method2 should need blank line before it"
        );
    }

    #[test]
    fn test_method_has_blank_line_ok() {
        let source = r#"
class Test {
    void method1() {}

    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "method2 has blank line, should be OK"
        );
    }

    #[test]
    fn test_constructor_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    Test() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.iter().any(|d| d.kind.body.contains("CTOR_DEF")),
            "constructor should need blank line"
        );
    }

    #[test]
    fn test_field_needs_blank_line_default() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "field y should need blank line (default config)"
        );
    }

    #[test]
    fn test_field_no_blank_line_allowed() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let rule = EmptyLineSeparator {
            allow_no_empty_line_between_fields: true,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics.is_empty(),
            "fields without blank lines should be OK when allowNoEmptyLineBetweenFields=true"
        );
    }

    #[test]
    fn test_static_init_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    static {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("STATIC_INIT")),
            "static init should need blank line"
        );
    }

    #[test]
    fn test_comment_before_method_ok() {
        let source = r#"
class Test {
    void method1() {}

    // comment before method2
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "blank line before comment should satisfy requirement"
        );
    }

    #[test]
    fn test_first_member_no_violation() {
        let source = r#"
class Test {
    void method1() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "first member should not need blank line"
        );
    }
}
