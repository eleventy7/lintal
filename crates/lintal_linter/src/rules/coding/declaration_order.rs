//! DeclarationOrder rule implementation.
//!
//! Checks that class/interface members are declared in the correct order:
//! 1. Static fields
//! 2. Instance fields
//! 3. Constructors
//! 4. Methods
//!
//! Checkstyle equivalent: DeclarationOrderCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: static variable in wrong order.
#[derive(Debug, Clone)]
pub struct StaticVariableOrderViolation;

impl Violation for StaticVariableOrderViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Static variable definition in wrong order.".to_string()
    }
}

/// Violation: instance variable in wrong order.
#[derive(Debug, Clone)]
pub struct InstanceVariableOrderViolation;

impl Violation for InstanceVariableOrderViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Instance variable definition in wrong order.".to_string()
    }
}

/// Violation: constructor in wrong order.
#[derive(Debug, Clone)]
pub struct ConstructorOrderViolation;

impl Violation for ConstructorOrderViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Constructor definition in wrong order.".to_string()
    }
}

/// Violation: variable access modifier in wrong order.
#[derive(Debug, Clone)]
pub struct VariableAccessOrderViolation;

impl Violation for VariableAccessOrderViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Variable access definition in wrong order.".to_string()
    }
}

/// Configuration for DeclarationOrder rule.
#[derive(Debug, Clone, Default)]
pub struct DeclarationOrder {
    pub ignore_constructors: bool,
    pub ignore_modifiers: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
];

impl FromConfig for DeclarationOrder {
    const MODULE_NAME: &'static str = "DeclarationOrder";

    fn from_config(properties: &Properties) -> Self {
        let ignore_constructors = properties
            .get("ignoreConstructors")
            .is_some_and(|v| *v == "true");
        let ignore_modifiers = properties
            .get("ignoreModifiers")
            .is_some_and(|v| *v == "true");

        Self {
            ignore_constructors,
            ignore_modifiers,
        }
    }
}

/// Member category in declaration order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MemberCategory {
    StaticField = 1,
    InstanceField = 2,
    Constructor = 3,
    Method = 4,
}

/// Access modifier visibility level (lower = more visible).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum AccessLevel {
    Public = 1,
    Protected = 2,
    PackagePrivate = 3,
    Private = 4,
}

impl Rule for DeclarationOrder {
    fn name(&self) -> &'static str {
        "DeclarationOrder"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();
        if kind != "class_declaration"
            && kind != "interface_declaration"
            && kind != "enum_declaration"
            && kind != "record_declaration"
        {
            return vec![];
        }

        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        let mut highest_category = MemberCategory::StaticField;
        // Track highest access level seen for each category (lower = more visible)
        let mut highest_static_access = AccessLevel::Public;
        let mut highest_instance_access = AccessLevel::Public;
        let mut diagnostics = vec![];

        for child in body.children() {
            let category = match child.kind() {
                "field_declaration" => {
                    if self.has_static_modifier(ctx, &child) {
                        Some(MemberCategory::StaticField)
                    } else {
                        Some(MemberCategory::InstanceField)
                    }
                }
                "constructor_declaration" => {
                    if self.ignore_constructors {
                        None
                    } else {
                        Some(MemberCategory::Constructor)
                    }
                }
                "method_declaration" => Some(MemberCategory::Method),
                // Nested types are not checked for ordering in checkstyle's DeclarationOrder
                "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "annotation_type_declaration" => None,
                // Static/instance initializers are not checked
                "static_initializer" => None,
                "block" => {
                    // Instance initializer blocks are not checked
                    None
                }
                _ => None,
            };

            let Some(cat) = category else {
                continue;
            };

            if cat < highest_category {
                // Out of order (wrong category)
                let range = child.range();
                match cat {
                    MemberCategory::StaticField => {
                        diagnostics.push(Diagnostic::new(StaticVariableOrderViolation, range));
                    }
                    MemberCategory::InstanceField => {
                        diagnostics.push(Diagnostic::new(InstanceVariableOrderViolation, range));
                    }
                    MemberCategory::Constructor => {
                        diagnostics.push(Diagnostic::new(ConstructorOrderViolation, range));
                    }
                    MemberCategory::Method => {}
                }
            } else {
                // Same or higher category — check access modifier ordering within category
                if !self.ignore_modifiers && child.kind() == "field_declaration" {
                    let access = self.get_access_level(ctx, &child);
                    match cat {
                        MemberCategory::StaticField => {
                            if access < highest_static_access {
                                // More visible modifier after less visible → out of order
                                diagnostics.push(Diagnostic::new(
                                    VariableAccessOrderViolation,
                                    child.range(),
                                ));
                            } else {
                                highest_static_access = access;
                            }
                        }
                        MemberCategory::InstanceField => {
                            if access < highest_instance_access {
                                diagnostics.push(Diagnostic::new(
                                    VariableAccessOrderViolation,
                                    child.range(),
                                ));
                            } else {
                                highest_instance_access = access;
                            }
                        }
                        _ => {}
                    }
                }
                highest_category = cat;
            }
        }

        diagnostics
    }
}

impl DeclarationOrder {
    fn get_access_level(&self, ctx: &CheckContext, node: &CstNode) -> AccessLevel {
        for child in node.children() {
            if child.kind() == "modifiers" {
                for modifier in child.children() {
                    match modifier.kind() {
                        "public" => return AccessLevel::Public,
                        "protected" => return AccessLevel::Protected,
                        "private" => return AccessLevel::Private,
                        "modifier" => {
                            let text = &ctx.source()[modifier.range()];
                            match text {
                                "public" => return AccessLevel::Public,
                                "protected" => return AccessLevel::Protected,
                                "private" => return AccessLevel::Private,
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
                return AccessLevel::PackagePrivate;
            }
        }
        AccessLevel::PackagePrivate
    }

    fn has_static_modifier(&self, ctx: &CheckContext, node: &CstNode) -> bool {
        for child in node.children() {
            if child.kind() == "modifiers" {
                for modifier in child.children() {
                    if modifier.kind() == "static"
                        || (modifier.kind() == "modifier"
                            && &ctx.source()[modifier.range()] == "static")
                    {
                        return true;
                    }
                }
                return false;
            }
        }
        false
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
        let rule = DeclarationOrder::default();
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
    fn test_correct_order_no_violation() {
        let source = r#"
class Test {
    static int A = 1;
    int b;
    Test() {}
    void method() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_instance_field_after_method() {
        let source = r#"
class Test {
    void method() {}
    int b;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_static_field_after_constructor() {
        let source = r#"
class Test {
    Test() {}
    static int A = 1;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_constructor_after_method() {
        let source = r#"
class Test {
    void method() {}
    Test() {}
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_multiple_methods_no_violation() {
        let source = r#"
class Test {
    static int A = 1;
    int b;
    Test() {}
    void method1() {}
    void method2() {}
}
"#;
        assert!(check_source(source).is_empty());
    }
}
