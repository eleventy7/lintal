//! HiddenField rule implementation.
//!
//! Checks that a local variable or method parameter does not shadow
//! a field defined in the same class.
//!
//! Checkstyle equivalent: HiddenFieldCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: variable hides a field.
#[derive(Debug, Clone)]
pub struct HiddenFieldViolation {
    pub name: String,
}

impl Violation for HiddenFieldViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("'{}' hides a field.", self.name)
    }
}

/// Configuration for HiddenField rule.
#[derive(Debug, Clone, Default)]
pub struct HiddenField {
    pub ignore_constructor_parameter: bool,
    pub ignore_setter: bool,
    pub setter_can_return_its_class: bool,
    pub ignore_abstract_methods: bool,
    pub ignore_format: Option<Regex>,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "lambda_expression",
    "class_declaration",
    "enum_declaration",
    "record_declaration",
];

impl FromConfig for HiddenField {
    const MODULE_NAME: &'static str = "HiddenField";

    fn from_config(properties: &Properties) -> Self {
        let ignore_constructor_parameter = properties
            .get("ignoreConstructorParameter")
            .is_some_and(|v| *v == "true");
        let ignore_setter = properties.get("ignoreSetter").is_some_and(|v| *v == "true");
        let setter_can_return_its_class = properties
            .get("setterCanReturnItsClass")
            .is_some_and(|v| *v == "true");
        let ignore_abstract_methods = properties
            .get("ignoreAbstractMethods")
            .is_some_and(|v| *v == "true");
        let ignore_format = properties
            .get("ignoreFormat")
            .and_then(|v| Regex::new(v).ok());

        Self {
            ignore_constructor_parameter,
            ignore_setter,
            setter_can_return_its_class,
            ignore_abstract_methods,
            ignore_format,
        }
    }
}

/// A field with its static/instance status.
#[derive(Debug, Clone)]
struct FieldInfo {
    name: String,
    is_static: bool,
}

impl Rule for HiddenField {
    fn name(&self) -> &'static str {
        "HiddenField"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();

        // Handle class/enum/record declarations — check initializer blocks
        if kind == "class_declaration" || kind == "enum_declaration" || kind == "record_declaration"
        {
            return self.check_initializer_blocks(ctx, node);
        }

        if kind != "method_declaration"
            && kind != "constructor_declaration"
            && kind != "lambda_expression"
        {
            return vec![];
        }

        // Determine if we're in a static context
        let is_static_context = kind == "method_declaration" && self.has_static_modifier(node);

        // Collect field names from enclosing class(es) with static info
        let all_fields = self.collect_all_enclosing_fields(ctx, node);
        // Filter fields based on context
        let field_names: Vec<String> = self.filter_fields_for_context(
            &all_fields,
            is_static_context,
            self.is_in_static_inner_class(node),
        );
        if field_names.is_empty() {
            return vec![];
        }

        let is_constructor = kind == "constructor_declaration";

        // Check if this is an abstract method (no body)
        if kind == "method_declaration" && self.ignore_abstract_methods {
            let has_body = node.children().any(|c| c.kind() == "block");
            if !has_body {
                return vec![];
            }
        }

        // Get the class name for setter detection
        let class_name = if self.setter_can_return_its_class {
            self.get_enclosing_class_name(ctx, node)
        } else {
            None
        };

        let mut diagnostics = vec![];

        // Check formal parameters
        if kind != "lambda_expression"
            && let Some(params) = node.child_by_field_name("parameters")
        {
            for param in params.children() {
                if param.kind() != "formal_parameter" {
                    continue;
                }
                let Some(name_node) = param.child_by_field_name("name") else {
                    continue;
                };
                let param_name = &ctx.source()[name_node.range()];

                if !field_names.contains(&param_name.to_string()) {
                    continue;
                }

                if self.should_skip_format(param_name) {
                    continue;
                }

                // Skip constructor params if configured
                if is_constructor && self.ignore_constructor_parameter {
                    continue;
                }

                // Skip setter params if configured
                if !is_constructor
                    && self.ignore_setter
                    && self.is_setter_method(ctx, node, param_name, class_name.as_deref())
                {
                    continue;
                }

                diagnostics.push(Diagnostic::new(
                    HiddenFieldViolation {
                        name: param_name.to_string(),
                    },
                    name_node.range(),
                ));
            }
        }

        // Check local variable declarations in the body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block_for_locals(ctx, &body, &field_names, &mut diagnostics);
        }

        diagnostics
    }
}

impl HiddenField {
    /// Check initializer blocks (instance and static) within a class for hidden fields.
    fn check_initializer_blocks(
        &self,
        ctx: &CheckContext,
        class_node: &CstNode,
    ) -> Vec<Diagnostic> {
        let Some(body) = class_node.child_by_field_name("body") else {
            return vec![];
        };

        let all_fields = self.collect_fields_from_class_and_outers(ctx, class_node);
        let mut diagnostics = vec![];

        for child in body.children() {
            match child.kind() {
                "block" => {
                    // Instance initializer block
                    let field_names = self.filter_fields_for_context(&all_fields, false, false);
                    self.check_block_for_locals(ctx, &child, &field_names, &mut diagnostics);
                }
                "static_initializer" => {
                    // Static initializer — only static fields can be hidden
                    let field_names = self.filter_fields_for_context(&all_fields, true, false);
                    // The static_initializer contains a block child
                    for sc in child.children() {
                        if sc.kind() == "block" {
                            self.check_block_for_locals(ctx, &sc, &field_names, &mut diagnostics);
                        }
                    }
                }
                _ => {}
            }
        }

        diagnostics
    }

    /// Collect fields from all enclosing scopes (class bodies, enum bodies, enum constant bodies).
    fn collect_all_enclosing_fields(&self, ctx: &CheckContext, node: &CstNode) -> Vec<FieldInfo> {
        let mut fields = vec![];
        let mut current = node.parent();

        while let Some(parent) = current {
            match parent.kind() {
                "class_declaration" | "enum_declaration" | "record_declaration" => {
                    self.collect_fields_from_class(ctx, &parent, &mut fields);
                }
                "class_body" => {
                    // Enum constant anonymous class body — collect fields from it
                    if let Some(gp) = parent.parent()
                        && gp.kind() == "enum_constant"
                    {
                        self.collect_fields_from_body_node(ctx, &parent, &mut fields);
                    }
                }
                _ => {}
            }
            current = parent.parent();
        }

        fields
    }

    /// Collect fields from a class and all its outer classes.
    fn collect_fields_from_class_and_outers(
        &self,
        ctx: &CheckContext,
        class_node: &CstNode,
    ) -> Vec<FieldInfo> {
        let mut fields = vec![];

        // Collect fields from the immediate class
        self.collect_fields_from_class(ctx, class_node, &mut fields);

        // Walk up to outer classes and collect their fields too
        let mut current = class_node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "class_declaration" | "enum_declaration" | "record_declaration" => {
                    self.collect_fields_from_class(ctx, &parent, &mut fields);
                }
                _ => {}
            }
            current = parent.parent();
        }

        fields
    }

    /// Collect fields from a single class/enum with static/instance info.
    fn collect_fields_from_class(
        &self,
        ctx: &CheckContext,
        class_node: &CstNode,
        fields: &mut Vec<FieldInfo>,
    ) {
        let Some(body) = class_node.child_by_field_name("body") else {
            return;
        };

        for child in body.children() {
            if child.kind() == "field_declaration" {
                self.extract_field_info(ctx, &child, fields);
            }
            // For enums: fields/constructors/methods are inside enum_body_declarations
            if child.kind() == "enum_body_declarations" {
                for inner in child.children() {
                    if inner.kind() == "field_declaration" {
                        self.extract_field_info(ctx, &inner, fields);
                    }
                }
            }
        }
    }

    /// Collect fields from a body node (e.g., enum constant's class_body).
    fn collect_fields_from_body_node(
        &self,
        ctx: &CheckContext,
        body: &CstNode,
        fields: &mut Vec<FieldInfo>,
    ) {
        for child in body.children() {
            if child.kind() == "field_declaration" {
                self.extract_field_info(ctx, &child, fields);
            }
        }
    }

    /// Extract field info from a field_declaration node.
    fn extract_field_info(
        &self,
        ctx: &CheckContext,
        field_decl: &CstNode,
        fields: &mut Vec<FieldInfo>,
    ) {
        let is_static = self.has_static_modifier(field_decl);
        for decl in field_decl.children() {
            if decl.kind() == "variable_declarator"
                && let Some(name_node) = decl.child_by_field_name("name")
            {
                let name = &ctx.source()[name_node.range()];
                fields.push(FieldInfo {
                    name: name.to_string(),
                    is_static,
                });
            }
        }
    }

    /// Filter fields based on the context (static method, static inner class).
    fn filter_fields_for_context(
        &self,
        fields: &[FieldInfo],
        is_static_method: bool,
        is_static_inner: bool,
    ) -> Vec<String> {
        fields
            .iter()
            .filter(|f| {
                if is_static_method || is_static_inner {
                    // In static context, only static fields can be hidden
                    f.is_static
                } else {
                    true
                }
            })
            .map(|f| f.name.clone())
            .collect()
    }

    /// Check if a method/field has the static modifier.
    fn has_static_modifier(&self, node: &CstNode) -> bool {
        for child in node.children() {
            if child.kind() == "modifiers" {
                return child.children().any(|m| m.kind() == "static");
            }
        }
        false
    }

    /// Check if the node is inside a static inner class.
    fn is_in_static_inner_class(&self, node: &CstNode) -> bool {
        let Some(class_node) = self.find_enclosing_class(node) else {
            return false;
        };
        // Check if this class is a static inner class
        if self.has_static_modifier(&class_node) {
            return true;
        }
        // In Java, inner classes inside interfaces/enums are implicitly static
        if let Some(parent) = class_node.parent()
            && let Some(grandparent) = parent.parent()
            && grandparent.kind() == "interface_declaration"
        {
            return true;
        }
        false
    }

    /// Find the enclosing class, enum, or record declaration.
    fn find_enclosing_class<'a>(&self, node: &CstNode<'a>) -> Option<CstNode<'a>> {
        let mut current = node.parent();
        while let Some(parent) = current {
            match parent.kind() {
                "class_declaration" | "enum_declaration" | "record_declaration" => {
                    return Some(parent);
                }
                _ => {
                    current = parent.parent();
                }
            }
        }
        None
    }

    /// Get the name of the enclosing class.
    fn get_enclosing_class_name(&self, ctx: &CheckContext, node: &CstNode) -> Option<String> {
        let class_node = self.find_enclosing_class(node)?;
        let name_node = class_node.child_by_field_name("name")?;
        Some(ctx.source()[name_node.range()].to_string())
    }

    /// Check if a method is a setter for the given parameter name.
    fn is_setter_method(
        &self,
        ctx: &CheckContext,
        method: &CstNode,
        param_name: &str,
        class_name: Option<&str>,
    ) -> bool {
        let Some(name_node) = method.child_by_field_name("name") else {
            return false;
        };
        let method_name = &ctx.source()[name_node.range()];

        // Method name must be "set" + capitalized param name
        let expected_setter_name = format!("set{}", capitalize(param_name));
        if method_name != expected_setter_name {
            return false;
        }

        // Must have exactly 1 parameter
        let Some(params) = method.child_by_field_name("parameters") else {
            return false;
        };
        let param_count = params
            .children()
            .filter(|p| p.kind() == "formal_parameter")
            .count();
        if param_count != 1 {
            return false;
        }

        // Return type must be void (or the class type if setterCanReturnItsClass)
        if let Some(return_type) = method.child_by_field_name("type") {
            let return_type_text = &ctx.source()[return_type.range()];
            if return_type_text == "void" {
                return true;
            }
            if let Some(cn) = class_name
                && return_type_text == cn
            {
                return true;
            }
            return false;
        }

        // void_type is a separate node kind
        for child in method.children() {
            if child.kind() == "void_type" {
                return true;
            }
        }

        false
    }

    /// Recursively check a block for local variable declarations that hide fields.
    fn check_block_for_locals(
        &self,
        ctx: &CheckContext,
        block: &CstNode,
        field_names: &[String],
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for child in block.children() {
            match child.kind() {
                "local_variable_declaration" => {
                    for decl_child in child.children() {
                        if decl_child.kind() == "variable_declarator"
                            && let Some(name_node) = decl_child.child_by_field_name("name")
                        {
                            let var_name = &ctx.source()[name_node.range()];
                            if field_names.contains(&var_name.to_string())
                                && !self.should_skip_format(var_name)
                            {
                                diagnostics.push(Diagnostic::new(
                                    HiddenFieldViolation {
                                        name: var_name.to_string(),
                                    },
                                    name_node.range(),
                                ));
                            }
                        }
                    }
                }
                "block"
                | "if_statement"
                | "for_statement"
                | "enhanced_for_statement"
                | "while_statement"
                | "do_statement"
                | "try_statement"
                | "try_with_resources_statement"
                | "switch_block_statement_group"
                | "switch_expression"
                | "synchronized_statement" => {
                    self.check_block_for_locals(ctx, &child, field_names, diagnostics);
                }
                _ => {}
            }
        }
    }

    fn should_skip_format(&self, name: &str) -> bool {
        if let Some(ref pattern) = self.ignore_format {
            return pattern.is_match(name);
        }
        false
    }
}

/// Capitalize the first letter of a string.
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str) -> Vec<usize> {
        check_source_with_config(source, &HiddenField::default())
    }

    fn check_source_with_config(source: &str, rule: &HiddenField) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
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
    fn test_param_hides_field() {
        let source = r#"
class Test {
    int value;
    void method(int value) {}
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], 4);
    }

    #[test]
    fn test_local_var_hides_field() {
        let source = r#"
class Test {
    int value;
    void method() {
        int value = 5;
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], 5);
    }

    #[test]
    fn test_no_hiding() {
        let source = r#"
class Test {
    int value;
    void method(int other) {
        int another = 5;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_ignore_constructor_parameter() {
        let source = r#"
class Test {
    int value;
    Test(int value) {}
}
"#;
        // Without config: violation
        let v = check_source(source);
        assert_eq!(v.len(), 1);

        // With ignoreConstructorParameter: no violation
        let rule = HiddenField {
            ignore_constructor_parameter: true,
            ..Default::default()
        };
        let v = check_source_with_config(source, &rule);
        assert!(v.is_empty());
    }

    #[test]
    fn test_ignore_setter() {
        let source = r#"
class Test {
    int value;
    void setValue(int value) {}
}
"#;
        // Without config: violation
        let v = check_source(source);
        assert_eq!(v.len(), 1);

        // With ignoreSetter: no violation
        let rule = HiddenField {
            ignore_setter: true,
            ..Default::default()
        };
        let v = check_source_with_config(source, &rule);
        assert!(v.is_empty());
    }

    #[test]
    fn test_setter_returning_class() {
        let source = r#"
class Test {
    int value;
    Test setValue(int value) { return this; }
}
"#;
        // With ignoreSetter but not setterCanReturnItsClass: violation
        let rule = HiddenField {
            ignore_setter: true,
            ..Default::default()
        };
        let v = check_source_with_config(source, &rule);
        assert_eq!(v.len(), 1);

        // With both: no violation
        let rule = HiddenField {
            ignore_setter: true,
            setter_can_return_its_class: true,
            ..Default::default()
        };
        let v = check_source_with_config(source, &rule);
        assert!(v.is_empty());
    }

    // Regression: enum fields in enum_body_declarations should be detected
    #[test]
    fn test_enum_field_hidden_by_method_param() {
        let source = r#"
enum MyEnum {
    A, B;
    int value;
    void method(int value) {}
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    // Regression: enum constructor parameter hides enum field
    #[test]
    fn test_enum_constructor_hides_field() {
        let source = r#"
enum MyEnum {
    A(1);
    int value;
    MyEnum(int value) {}
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    // Regression: enum constant anonymous body field hidden by method local
    #[test]
    fn test_enum_constant_body_field_hidden() {
        let source = r#"
enum MyEnum {
    A {
        int x;
        void method() {
            int x = 0;
        }
    };
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    // Regression: static method in enum should only see static fields
    #[test]
    fn test_enum_static_method_hides_static_field() {
        let source = r#"
enum MyEnum {
    A;
    static int count;
    int value;
    static void method() {
        int count = 0;
        int value = 0;
    }
}
"#;
        let v = check_source(source);
        // Only count is hidden (static field in static method), not value (instance field)
        assert_eq!(v.len(), 1);
    }
}
