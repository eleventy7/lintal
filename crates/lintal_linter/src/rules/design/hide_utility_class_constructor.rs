//! HideUtilityClassConstructor rule implementation.
//!
//! Checks that utility classes (classes with only static methods/fields)
//! do not have a public or default constructor.
//!
//! Checkstyle equivalent: HideUtilityClassConstructorCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: utility class should not have public/default constructor.
#[derive(Debug, Clone)]
pub struct HideUtilityClassConstructorViolation;

impl Violation for HideUtilityClassConstructorViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Utility classes should not have a public or default constructor.".to_string()
    }
}

/// Configuration for HideUtilityClassConstructor rule.
#[derive(Debug, Clone, Default)]
pub struct HideUtilityClassConstructor;

const RELEVANT_KINDS: &[&str] = &["class_declaration"];

impl FromConfig for HideUtilityClassConstructor {
    const MODULE_NAME: &'static str = "HideUtilityClassConstructor";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for HideUtilityClassConstructor {
    fn name(&self) -> &'static str {
        "HideUtilityClassConstructor"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "class_declaration" {
            return vec![];
        }

        // Skip abstract classes
        if let Some(modifiers) = node.children().find(|c| c.kind() == "modifiers")
            && modifiers.children().any(|m| m.kind() == "abstract")
        {
            return vec![];
        }

        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        // Analyze class members
        let mut has_static_member = false;
        let mut has_instance_method = false;
        let mut has_instance_field = false;
        let mut constructors: Vec<CstNode> = vec![];
        let mut has_main_method = false;

        for child in body.children() {
            match child.kind() {
                "method_declaration" => {
                    let is_static = self.has_static_modifier(ctx, &child);
                    if is_static {
                        has_static_member = true;
                        // Check for main method (public static void main)
                        if self.is_main_method(ctx, &child) {
                            has_main_method = true;
                        }
                    } else {
                        has_instance_method = true;
                    }
                }
                "field_declaration" => {
                    let is_static = self.has_static_modifier(ctx, &child);
                    if is_static {
                        has_static_member = true;
                    } else {
                        has_instance_field = true;
                    }
                }
                "constructor_declaration" => {
                    constructors.push(child);
                }
                // Inner classes/interfaces don't count as instance members
                "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "record_declaration"
                | "annotation_type_declaration" => {}
                // Static initializers count as static
                "static_initializer" => {
                    has_static_member = true;
                }
                _ => {}
            }
        }

        // Not a utility class if:
        // - Has instance methods
        // - Has instance fields
        // - Has no static members at all
        // - Is a main class (has main method)
        if has_instance_method || has_instance_field || !has_static_member || has_main_method {
            return vec![];
        }

        // It's a utility class. Check constructors.
        if constructors.is_empty() {
            // No explicit constructor → implicit public constructor → violation
            let Some(name_node) = node.child_by_field_name("name") else {
                return vec![];
            };
            return vec![Diagnostic::new(
                HideUtilityClassConstructorViolation,
                name_node.range(),
            )];
        }

        // Check if all constructors are private or protected
        let all_non_public = constructors
            .iter()
            .all(|ctor| self.is_non_public_constructor(ctor));
        if all_non_public {
            return vec![];
        }

        // Has public or default constructor → violation
        let Some(name_node) = node.child_by_field_name("name") else {
            return vec![];
        };
        vec![Diagnostic::new(
            HideUtilityClassConstructorViolation,
            name_node.range(),
        )]
    }
}

impl HideUtilityClassConstructor {
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

    fn is_non_public_constructor(&self, ctor: &CstNode) -> bool {
        for child in ctor.children() {
            if child.kind() == "modifiers" {
                return child
                    .children()
                    .any(|m| m.kind() == "private" || m.kind() == "protected");
            }
        }
        false
    }

    fn is_main_method(&self, ctx: &CheckContext, method: &CstNode) -> bool {
        // Check method name is "main"
        let Some(name_node) = method.child_by_field_name("name") else {
            return false;
        };
        let name = &ctx.source()[name_node.range()];
        if name != "main" {
            return false;
        }

        // Check it's public static void main(String[] args)
        let has_public = self.has_specific_modifier(method, "public");
        let is_static = self.has_static_modifier(ctx, method);
        let is_void = method.children().any(|c| c.kind() == "void_type");

        has_public && is_static && is_void
    }

    fn has_specific_modifier(&self, node: &CstNode, modifier_kind: &str) -> bool {
        for child in node.children() {
            if child.kind() == "modifiers" {
                return child.children().any(|m| m.kind() == modifier_kind);
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
        let rule = HideUtilityClassConstructor;
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
    fn test_utility_class_no_constructor() {
        let source = r#"
class Utils {
    static void helper() {}
    static int VALUE = 42;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_utility_class_with_private_constructor() {
        let source = r#"
class Utils {
    private Utils() {}
    static void helper() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_utility_class_with_public_constructor() {
        let source = r#"
class Utils {
    public Utils() {}
    static void helper() {}
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_not_utility_class_has_instance_method() {
        let source = r#"
class Service {
    static void helper() {}
    void process() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_not_utility_class_has_instance_field() {
        let source = r#"
class Container {
    static void helper() {}
    int value;
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_abstract_class_skipped() {
        let source = r#"
abstract class Base {
    static void helper() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_no_static_members() {
        let source = r#"
class Regular {
    void method() {}
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_empty_class() {
        let source = r#"
class Empty {
}
"#;
        assert!(check_source(source).is_empty());
    }
}
