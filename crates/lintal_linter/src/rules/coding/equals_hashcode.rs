//! EqualsHashCode rule implementation.
//!
//! Checks that classes defining equals() also define hashCode() and vice versa.
//!
//! Checkstyle equivalent: EqualsHashCodeCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: equals() without hashCode().
#[derive(Debug, Clone)]
pub struct EqualsWithoutHashCodeViolation;

impl Violation for EqualsWithoutHashCodeViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Definition of 'equals()' without corresponding definition of 'hashCode()'.".to_string()
    }
}

/// Violation: hashCode() without equals().
#[derive(Debug, Clone)]
pub struct HashCodeWithoutEqualsViolation;

impl Violation for HashCodeWithoutEqualsViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Definition of 'hashCode()' without corresponding definition of 'equals()'.".to_string()
    }
}

/// Configuration for EqualsHashCode rule.
#[derive(Debug, Clone, Default)]
pub struct EqualsHashCode;

const RELEVANT_KINDS: &[&str] = &[
    "class_declaration",
    "record_declaration",
    "enum_declaration",
];

impl FromConfig for EqualsHashCode {
    const MODULE_NAME: &'static str = "EqualsHashCode";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for EqualsHashCode {
    fn name(&self) -> &'static str {
        "EqualsHashCode"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();
        if kind != "class_declaration" && kind != "record_declaration" && kind != "enum_declaration"
        {
            return vec![];
        }

        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        let mut equals_node: Option<CstNode> = None;
        let mut hashcode_node: Option<CstNode> = None;

        // Collect methods from body - for enums, methods are inside enum_body_declarations
        let methods: Vec<CstNode> = Self::collect_methods(&body);

        for child in methods {
            let Some(name_node) = child.child_by_field_name("name") else {
                continue;
            };
            let name = &ctx.source()[name_node.range()];

            match name {
                "equals" => {
                    let Some(params) = child.child_by_field_name("parameters") else {
                        continue;
                    };
                    let formal_params: Vec<CstNode> = params
                        .children()
                        .filter(|p| p.kind() == "formal_parameter")
                        .collect();
                    if formal_params.len() == 1 {
                        let param = &formal_params[0];
                        if let Some(type_node) = param.child_by_field_name("type") {
                            let type_text = &ctx.source()[type_node.range()];
                            if type_text == "Object" || type_text == "java.lang.Object" {
                                equals_node = Some(name_node);
                            }
                        }
                    }
                }
                "hashCode" => {
                    let Some(params) = child.child_by_field_name("parameters") else {
                        continue;
                    };
                    let formal_params: Vec<CstNode> = params
                        .children()
                        .filter(|p| p.kind() == "formal_parameter")
                        .collect();
                    if formal_params.is_empty() {
                        hashcode_node = Some(name_node);
                    }
                }
                _ => {}
            }
        }

        let mut diagnostics = vec![];

        match (equals_node, hashcode_node) {
            (Some(eq), None) => {
                diagnostics.push(Diagnostic::new(EqualsWithoutHashCodeViolation, eq.range()));
            }
            (None, Some(hc)) => {
                diagnostics.push(Diagnostic::new(HashCodeWithoutEqualsViolation, hc.range()));
            }
            _ => {}
        }

        diagnostics
    }
}

impl EqualsHashCode {
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
        let rule = EqualsHashCode;
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
    fn test_equals_without_hashcode() {
        let source = r#"
class Foo {
    public boolean equals(Object o) {
        return true;
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_hashcode_without_equals() {
        let source = r#"
class Foo {
    public int hashCode() {
        return 42;
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_both_defined() {
        let source = r#"
class Foo {
    public boolean equals(Object o) {
        return true;
    }
    public int hashCode() {
        return 42;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_neither_defined() {
        let source = r#"
class Foo {
    void method() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_equals_wrong_signature() {
        let source = r#"
class Foo {
    public boolean equals(Foo other) {
        return true;
    }
    public int hashCode() {
        return 42;
    }
}
"#;
        // equals(Foo) is not equals(Object), so hashCode without equals
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }
}
