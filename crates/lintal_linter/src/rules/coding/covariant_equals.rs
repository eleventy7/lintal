//! CovariantEquals rule implementation.
//!
//! Checks that classes defining a covariant equals() method also
//! override equals(java.lang.Object).
//!
//! Checkstyle equivalent: CovariantEqualsCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: covariant equals without equals(Object).
#[derive(Debug, Clone)]
pub struct CovariantEqualsViolation;

impl Violation for CovariantEqualsViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Covariant equals without overriding equals(java.lang.Object).".to_string()
    }
}

/// Configuration for CovariantEquals rule.
#[derive(Debug, Clone, Default)]
pub struct CovariantEquals;

const RELEVANT_KINDS: &[&str] = &[
    "class_declaration",
    "record_declaration",
    "enum_declaration",
    "object_creation_expression",
];

impl FromConfig for CovariantEquals {
    const MODULE_NAME: &'static str = "CovariantEquals";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for CovariantEquals {
    fn name(&self) -> &'static str {
        "CovariantEquals"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();

        // Handle anonymous classes (object_creation_expression with class_body)
        if kind == "object_creation_expression" {
            let body = node.children().find(|c| c.kind() == "class_body");
            let Some(body) = body else {
                return vec![];
            };
            return self.check_body(ctx, &body);
        }

        if kind != "class_declaration" && kind != "record_declaration" && kind != "enum_declaration"
        {
            return vec![];
        }

        // Skip abstract classes
        if kind == "class_declaration"
            && let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
            && modifiers.children().any(|m| m.kind() == "abstract")
        {
            return vec![];
        }

        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        self.check_body(ctx, &body)
    }
}

impl CovariantEquals {
    /// Check a class/enum/anonymous body for covariant equals.
    fn check_body(&self, ctx: &CheckContext, body: &CstNode) -> Vec<Diagnostic> {
        let mut has_object_equals = false;
        let mut covariant_equals_nodes: Vec<CstNode> = vec![];

        let methods: Vec<CstNode> = Self::collect_methods(body);

        for child in methods {
            let Some(name_node) = child.child_by_field_name("name") else {
                continue;
            };
            let name = &ctx.source()[name_node.range()];
            if name != "equals" {
                continue;
            }

            let Some(params) = child.child_by_field_name("parameters") else {
                continue;
            };

            let formal_params: Vec<CstNode> = params
                .children()
                .filter(|p| p.kind() == "formal_parameter")
                .collect();

            if formal_params.len() != 1 {
                continue;
            }

            let param = &formal_params[0];
            let Some(type_node) = param.child_by_field_name("type") else {
                continue;
            };

            let type_text = &ctx.source()[type_node.range()];
            if type_text == "Object" || type_text == "java.lang.Object" {
                has_object_equals = true;
            } else {
                covariant_equals_nodes.push(name_node);
            }
        }

        if has_object_equals || covariant_equals_nodes.is_empty() {
            return vec![];
        }

        covariant_equals_nodes
            .into_iter()
            .map(|n| Diagnostic::new(CovariantEqualsViolation, n.range()))
            .collect()
    }

    /// Collect method declarations from a type body.
    /// For enums, methods are inside `enum_body_declarations`.
    fn collect_methods<'a>(body: &CstNode<'a>) -> Vec<CstNode<'a>> {
        let mut methods = vec![];
        for child in body.children() {
            if child.kind() == "method_declaration" {
                methods.push(child);
            } else if child.kind() == "enum_body_declarations" {
                for inner in child.children() {
                    if inner.kind() == "method_declaration" {
                        methods.push(inner);
                    }
                }
            }
        }
        methods
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = CovariantEquals;
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push(loc.line.get());
            }
        }
        violations
    }

    #[test]
    fn test_covariant_equals_without_object_equals() {
        let source = r#"
class Foo {
    boolean equals(Foo other) {
        return true;
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_covariant_equals_with_object_equals() {
        let source = r#"
class Foo {
    boolean equals(Foo other) {
        return true;
    }
    boolean equals(Object other) {
        return true;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_only_object_equals() {
        let source = r#"
class Foo {
    boolean equals(Object other) {
        return true;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_no_equals() {
        let source = r#"
class Foo {
    void method() {}
}
"#;
        assert!(check_source(source).is_empty());
    }
}
