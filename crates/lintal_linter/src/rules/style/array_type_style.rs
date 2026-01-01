//! ArrayTypeStyle rule implementation.
//!
//! Checks the style of array type definitions. Java style (`int[] nums`) is preferred
//! over C style (`int nums[]`). Method return types must always use Java style.
//!
//! Checkstyle equivalent: ArrayTypeStyleCheck
//!
//! ## Examples
//!
//! ```java
//! // Java style (default, preferred)
//! int[] nums;
//! String[] args;
//!
//! // C style (violation by default)
//! int nums[];
//! String args[];
//!
//! // Method return type (always violation if C style)
//! byte getData()[] { ... }  // violation
//! byte[] getData() { ... }  // ok
//! ```

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: array brackets at illegal position.
#[derive(Debug, Clone)]
pub struct ArrayTypeStyleViolation;

impl Violation for ArrayTypeStyleViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Array brackets at illegal position.".to_string()
    }
}

/// Configuration for ArrayTypeStyle rule.
#[derive(Debug, Clone)]
pub struct ArrayTypeStyle {
    /// If true (default), enforce Java style (int[] nums).
    /// If false, enforce C style (int nums[]).
    pub java_style: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "variable_declarator",
    "formal_parameter",
];

impl Default for ArrayTypeStyle {
    fn default() -> Self {
        Self { java_style: true }
    }
}

impl FromConfig for ArrayTypeStyle {
    const MODULE_NAME: &'static str = "ArrayTypeStyle";

    fn from_config(properties: &Properties) -> Self {
        let java_style = properties
            .get("javaStyle")
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);

        Self { java_style }
    }
}

impl Rule for ArrayTypeStyle {
    fn name(&self) -> &'static str {
        "ArrayTypeStyle"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();

        // Handle method return types - always must be Java style
        if kind == "method_declaration" {
            return self.check_method_return_type(ctx, node);
        }

        // Handle variable declarations (local, field)
        if kind == "variable_declarator" {
            return self.check_variable_declarator(node);
        }

        // Handle formal parameters (method/constructor parameters)
        if kind == "formal_parameter" {
            return self.check_formal_parameter(node);
        }

        vec![]
    }
}

impl ArrayTypeStyle {
    /// Check method return type - brackets after method name are always a violation.
    fn check_method_return_type(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Look for dimensions after the method parameters
        // Method pattern: type name(params) dimensions { body }
        // We need to find any "dimensions" node that comes after the formal_parameters

        let mut found_params = false;
        let mut diagnostics = vec![];

        for child in node.children() {
            if child.kind() == "formal_parameters" {
                found_params = true;
                continue;
            }

            // After parameters, any dimensions node is a violation
            if found_params && child.kind() == "dimensions" {
                let dimensions_range = child.range();

                // Find where to insert the brackets (after the type, before method name)
                if let Some(fix) = self.create_method_return_fix(ctx, node, &child) {
                    diagnostics.push(
                        Diagnostic::new(ArrayTypeStyleViolation, dimensions_range).with_fix(fix),
                    );
                } else {
                    diagnostics.push(Diagnostic::new(ArrayTypeStyleViolation, dimensions_range));
                }
            }
        }

        diagnostics
    }

    /// Check variable declarators for C-style array declarations.
    fn check_variable_declarator(&self, node: &CstNode) -> Vec<Diagnostic> {
        // Look for dimensions child in the variable declarator
        // C-style: `int nums[]` has dimensions after the variable name
        // Java-style: `int[] nums` has dimensions in the type

        for child in node.children() {
            if child.kind() == "dimensions" {
                // This is C-style array declaration
                if self.java_style {
                    // Violation: should use Java style
                    let dimensions_range = child.range();
                    let dimensions_text = child.text();

                    // Try to create a fix
                    if let Some(fix) = self.create_variable_fix(node, &child, dimensions_text) {
                        return vec![
                            Diagnostic::new(ArrayTypeStyleViolation, dimensions_range)
                                .with_fix(fix),
                        ];
                    } else {
                        return vec![Diagnostic::new(ArrayTypeStyleViolation, dimensions_range)];
                    }
                }
            }
        }

        // If java_style is false, we need to check for Java-style declarations
        // and flag them. But this is complex because the dimensions are in the type,
        // not in the variable_declarator. We'd need to check the parent.
        if !self.java_style
            && let Some(parent) = node.parent()
            && (parent.kind() == "local_variable_declaration"
                || parent.kind() == "field_declaration")
        {
            for sibling in parent.children() {
                if sibling.kind() == "array_type" {
                    // Java style used, but we want C style
                    // Find the dimensions part of the array_type
                    for type_child in sibling.children() {
                        if type_child.kind() == "dimensions" {
                            return vec![Diagnostic::new(
                                ArrayTypeStyleViolation,
                                type_child.range(),
                            )];
                        }
                    }
                }
            }
        }

        vec![]
    }

    /// Check formal parameters for C-style array declarations.
    fn check_formal_parameter(&self, node: &CstNode) -> Vec<Diagnostic> {
        // For formal parameters like `String args[]`, the dimensions appear as a child
        // of the formal_parameter node, after the identifier

        for child in node.children() {
            if child.kind() == "dimensions" {
                // This is C-style array declaration in parameter
                if self.java_style {
                    let dimensions_range = child.range();
                    let dimensions_text = child.text();

                    // Try to create a fix
                    if let Some(fix) = self.create_parameter_fix(node, &child, dimensions_text) {
                        return vec![
                            Diagnostic::new(ArrayTypeStyleViolation, dimensions_range)
                                .with_fix(fix),
                        ];
                    } else {
                        return vec![Diagnostic::new(ArrayTypeStyleViolation, dimensions_range)];
                    }
                }
            }
        }

        // If java_style is false, check for Java-style declarations (array_type)
        if !self.java_style {
            for child in node.children() {
                if child.kind() == "array_type" {
                    // Find the dimensions part of the array_type
                    for type_child in child.children() {
                        if type_child.kind() == "dimensions" {
                            return vec![Diagnostic::new(
                                ArrayTypeStyleViolation,
                                type_child.range(),
                            )];
                        }
                    }
                }
            }
        }

        vec![]
    }

    /// Create a fix for C-style parameter declaration.
    fn create_parameter_fix(
        &self,
        param_node: &CstNode,
        dimensions_node: &CstNode,
        dimensions_text: &str,
    ) -> Option<Fix> {
        // Find the type node in the parameter
        let mut type_node = None;
        for child in param_node.children() {
            let kind = child.kind();
            if kind.ends_with("_type")
                || kind == "integral_type"
                || kind == "floating_point_type"
                || kind == "boolean_type"
                || kind == "type_identifier"
            {
                type_node = Some(child);
                break;
            }
        }

        let type_node = type_node?;
        let type_end = type_node.range().end();

        // Delete the dimensions from after the parameter name
        let dims_start = dimensions_node.range().start();
        let dims_end = dimensions_node.range().end();

        // Insert dimensions after type, delete from after parameter name
        Some(Fix::safe_edits(
            Edit::range_replacement(dimensions_text.to_string(), TextRange::empty(type_end)),
            [Edit::range_deletion(TextRange::new(dims_start, dims_end))],
        ))
    }

    /// Create a fix for method return type with brackets after name.
    fn create_method_return_fix(
        &self,
        ctx: &CheckContext,
        method_node: &CstNode,
        dimensions_node: &CstNode,
    ) -> Option<Fix> {
        let source = ctx.source();
        let dimensions_text = dimensions_node.text();

        // Find the type node and the identifier
        let mut type_node = None;
        let mut name_node = None;

        for child in method_node.children() {
            let kind = child.kind();
            // Type nodes: various primitive types, type_identifier, generic_type, array_type
            if kind.ends_with("_type")
                || kind == "void_type"
                || kind == "integral_type"
                || kind == "floating_point_type"
                || kind == "boolean_type"
                || kind == "type_identifier"
            {
                type_node = Some(child);
            } else if kind == "identifier" && type_node.is_some() {
                name_node = Some(child);
                break;
            }
        }

        let type_node = type_node?;
        let _name_node = name_node?;

        // Insert dimensions after the type, remove from after parameters
        let type_end = type_node.range().end();

        // Also need to handle any whitespace before dimensions
        let dims_start = dimensions_node.range().start();
        let dims_end = dimensions_node.range().end();

        // Check for whitespace before dimensions
        let before_dims = &source[..usize::from(dims_start)];
        let whitespace_start = before_dims.trim_end().len();

        let delete_range = TextRange::new(TextSize::new(whitespace_start as u32), dims_end);

        Some(Fix::safe_edits(
            Edit::range_replacement(dimensions_text.to_string(), TextRange::empty(type_end)),
            [Edit::range_deletion(delete_range)],
        ))
    }

    /// Create a fix for C-style variable declaration.
    fn create_variable_fix(
        &self,
        var_node: &CstNode,
        dimensions_node: &CstNode,
        dimensions_text: &str,
    ) -> Option<Fix> {
        // Find the parent declaration to get the type
        let parent = var_node.parent()?;
        let parent_kind = parent.kind();

        if parent_kind != "local_variable_declaration"
            && parent_kind != "field_declaration"
            && parent_kind != "formal_parameter"
        {
            return None;
        }

        // Find the type node in the parent
        let mut type_node = None;
        for child in parent.children() {
            let kind = child.kind();
            if kind.ends_with("_type")
                || kind == "integral_type"
                || kind == "floating_point_type"
                || kind == "boolean_type"
                || kind == "type_identifier"
            {
                type_node = Some(child);
                break;
            }
        }

        let type_node = type_node?;
        let type_end = type_node.range().end();

        // Delete the dimensions from after the variable name
        let dims_start = dimensions_node.range().start();
        let dims_end = dimensions_node.range().end();

        // Insert dimensions after type, delete from after variable name
        Some(Fix::safe_edits(
            Edit::range_replacement(dimensions_text.to_string(), TextRange::empty(type_end)),
            [Edit::range_deletion(TextRange::new(dims_start, dims_end))],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, ArrayTypeStyle::default())
    }

    fn check_source_with_config(source: &str, rule: ArrayTypeStyle) -> Vec<Diagnostic> {
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
    fn test_c_style_local_variable_violation() {
        let source = r#"
class Test {
    void foo() {
        int nums[];
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("illegal position"));
    }

    #[test]
    fn test_java_style_local_variable_ok() {
        let source = r#"
class Test {
    void foo() {
        int[] nums;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_c_style_field_violation() {
        let source = r#"
class Test {
    String strings[];
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_java_style_field_ok() {
        let source = r#"
class Test {
    String[] strings;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_method_return_type_c_style_violation() {
        let source = r#"
class Test {
    byte getData()[] {
        return null;
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_method_return_type_java_style_ok() {
        let source = r#"
class Test {
    byte[] getData() {
        return null;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_c_style_mode_flags_java_style() {
        let source = r#"
class Test {
    int[] nums;
}
"#;
        let rule = ArrayTypeStyle { java_style: false };
        let diagnostics = check_source_with_config(source, rule);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_c_style_mode_allows_c_style() {
        let source = r#"
class Test {
    void foo() {
        int nums[];
    }
}
"#;
        let rule = ArrayTypeStyle { java_style: false };
        let diagnostics = check_source_with_config(source, rule);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_multi_dimensional_array() {
        let source = r#"
class Test {
    int nums[][];
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let source = r#"
class Test {
    int a[];
    String b[];
    void foo() {
        int c[];
    }
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 3);
        for diag in &diagnostics {
            assert!(
                diag.fix.is_some(),
                "ArrayTypeStyle violations should have fixes"
            );
        }
    }
}
