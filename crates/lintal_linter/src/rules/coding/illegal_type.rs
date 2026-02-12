//! IllegalType rule implementation.
//!
//! Checks that particular classes or interfaces are never used as types
//! in variable declarations, return types, and parameters.
//!
//! Checkstyle equivalent: IllegalTypeCheck

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;
use tree_sitter::Node;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: usage of illegal type.
#[derive(Debug, Clone)]
pub struct IllegalTypeViolation {
    pub name: String,
}

impl Violation for IllegalTypeViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("Usage of type '{}' is not allowed.", self.name)
    }
}

/// Violation: abstract class name does not match format.
#[derive(Debug, Clone)]
pub struct IllegalAbstractClassNameViolation {
    pub name: String,
    pub format: String,
}

impl Violation for IllegalAbstractClassNameViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("Name '{}' must match pattern '{}'.", self.name, self.format)
    }
}

/// Configuration for IllegalType rule.
#[derive(Debug, Clone)]
pub struct IllegalType {
    pub illegal_class_names: HashSet<String>,
    pub legal_abstract_class_names: HashSet<String>,
    pub ignored_method_names: HashSet<String>,
    pub validate_abstract_class_names: bool,
    pub illegal_abstract_class_name_format: Regex,
    pub illegal_abstract_class_name_format_str: String,
    pub member_modifiers: HashSet<String>,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "field_declaration",
    "local_variable_declaration",
    "formal_parameter",
    "class_declaration",
    "interface_declaration",
    "record_declaration",
    "annotation_type_element_declaration",
    "method_invocation",
    "method_reference",
];

impl Default for IllegalType {
    fn default() -> Self {
        let mut illegal = HashSet::new();
        for name in &[
            "HashMap",
            "HashSet",
            "LinkedHashMap",
            "LinkedHashSet",
            "TreeMap",
            "TreeSet",
            "java.util.HashMap",
            "java.util.HashSet",
            "java.util.LinkedHashMap",
            "java.util.LinkedHashSet",
            "java.util.TreeMap",
            "java.util.TreeSet",
        ] {
            illegal.insert(name.to_string());
        }

        let mut ignored = HashSet::new();
        ignored.insert("getEnvironment".to_string());
        ignored.insert("getInitialContext".to_string());

        Self {
            illegal_class_names: illegal,
            legal_abstract_class_names: HashSet::new(),
            ignored_method_names: ignored,
            validate_abstract_class_names: false,
            illegal_abstract_class_name_format: Regex::new(r"^(.*[.])?Abstract.*$").unwrap(),
            illegal_abstract_class_name_format_str: r"^(.*[.])?Abstract.*$".to_string(),
            member_modifiers: HashSet::new(),
        }
    }
}

impl FromConfig for IllegalType {
    const MODULE_NAME: &'static str = "IllegalType";

    fn from_config(properties: &Properties) -> Self {
        let default = Self::default();

        let illegal_class_names = properties
            .get("illegalClassNames")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or(default.illegal_class_names);

        let legal_abstract_class_names = properties
            .get("legalAbstractClassNames")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or(default.legal_abstract_class_names);

        let ignored_method_names = properties
            .get("ignoredMethodNames")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or(default.ignored_method_names);

        let validate_abstract_class_names = properties
            .get("validateAbstractClassNames")
            .is_some_and(|v| *v == "true");

        let illegal_abstract_class_name_format_str = properties
            .get("illegalAbstractClassNameFormat")
            .map(|v| v.to_string())
            .unwrap_or(default.illegal_abstract_class_name_format_str);

        let illegal_abstract_class_name_format =
            Regex::new(&illegal_abstract_class_name_format_str)
                .unwrap_or(default.illegal_abstract_class_name_format);

        let member_modifiers = properties
            .get("memberModifiers")
            .map(|v| {
                v.split(',')
                    .map(|s| checkstyle_modifier_to_keyword(s.trim()))
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or(default.member_modifiers);

        Self {
            illegal_class_names,
            legal_abstract_class_names,
            ignored_method_names,
            validate_abstract_class_names,
            illegal_abstract_class_name_format,
            illegal_abstract_class_name_format_str,
            member_modifiers,
        }
    }
}

/// Convert checkstyle modifier token names to Java keywords.
fn checkstyle_modifier_to_keyword(token: &str) -> &str {
    match token {
        "LITERAL_PUBLIC" => "public",
        "LITERAL_PROTECTED" => "protected",
        "LITERAL_PRIVATE" => "private",
        "LITERAL_STATIC" => "static",
        "ABSTRACT" => "abstract",
        "FINAL" | "LITERAL_FINAL" => "final",
        "LITERAL_SYNCHRONIZED" => "synchronized",
        "LITERAL_TRANSIENT" => "transient",
        "LITERAL_VOLATILE" => "volatile",
        "STRICTFP" => "strictfp",
        "" => "",
        _ => token,
    }
}

impl Rule for IllegalType {
    fn name(&self) -> &'static str {
        "IllegalType"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();
        let source = ctx.source();
        let ts_node = node.inner();

        match kind {
            "method_declaration" => self.check_method(source, &ts_node),
            "field_declaration" | "local_variable_declaration" => {
                self.check_variable(source, &ts_node)
            }
            "formal_parameter" => self.check_parameter(source, &ts_node),
            "class_declaration" | "interface_declaration" | "record_declaration" => {
                self.check_type_declaration(source, &ts_node)
            }
            "annotation_type_element_declaration" => {
                self.check_annotation_element(source, &ts_node)
            }
            "method_invocation" | "method_reference" => {
                self.check_type_arguments_in_expr(source, &ts_node)
            }
            _ => vec![],
        }
    }
}

impl IllegalType {
    fn check_method(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Get method name
        let method_name = node
            .child_by_field_name("name")
            .map(|n| &source[n.start_byte()..n.end_byte()]);

        // Skip ignored methods
        if let Some(name) = method_name
            && self.ignored_method_names.contains(name)
        {
            return vec![];
        }

        // Skip @Override methods
        if has_override_annotation(node, source) {
            return vec![];
        }

        // Check member modifier filter
        if !self.member_modifiers.is_empty() && !self.has_matching_modifier(node, source) {
            return vec![];
        }

        // Check return type only (parameters are handled by check_parameter)
        if let Some(type_node) = node.child_by_field_name("type") {
            self.check_type_node(source, &type_node, &mut diagnostics);
        }

        // Check type parameters
        if let Some(type_params) = node.child_by_field_name("type_parameters") {
            self.check_type_parameters_node(source, &type_params, &mut diagnostics);
        }

        diagnostics
    }

    fn check_variable(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check member modifier filter
        if !self.member_modifiers.is_empty() && !self.has_matching_modifier(node, source) {
            return vec![];
        }

        if let Some(type_node) = node.child_by_field_name("type") {
            self.check_type_node(source, &type_node, &mut diagnostics);
        }

        diagnostics
    }

    fn check_parameter(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        // Walk up to find enclosing method
        let mut parent = node.parent();
        while let Some(p) = parent {
            if p.kind() == "formal_parameters" {
                parent = p.parent();
                continue;
            }
            if p.kind() == "method_declaration" {
                // Check if method is ignored
                if let Some(name_node) = p.child_by_field_name("name") {
                    let name = &source[name_node.start_byte()..name_node.end_byte()];
                    if self.ignored_method_names.contains(name) {
                        return vec![];
                    }
                }
                // Check if @Override
                if has_override_annotation(&p, source) {
                    return vec![];
                }
            }
            break;
        }

        // Check member modifier filter against the enclosing declaration
        if !self.member_modifiers.is_empty()
            && let Some(p) = node.parent()
            && let Some(pp) = p.parent()
            && !self.has_matching_modifier(&pp, source)
        {
            return vec![];
        }

        let mut diagnostics = vec![];
        if let Some(type_node) = node.child_by_field_name("type") {
            self.check_type_node(source, &type_node, &mut diagnostics);
        }
        diagnostics
    }

    fn check_type_declaration(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check member modifier filter
        if !self.member_modifiers.is_empty() && !self.has_matching_modifier(node, source) {
            return vec![];
        }

        // Check abstract class name validation
        if self.validate_abstract_class_names {
            self.check_abstract_class_name(source, node, &mut diagnostics);
        }

        // Iterate children to handle superclass, super_interfaces, extends_interfaces, type_parameters
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "superclass" => {
                    // class extends: contains "extends" keyword + type
                    let mut inner = child.walk();
                    for sc in child.children(&mut inner) {
                        if sc.is_named() {
                            self.check_type_node(source, &sc, &mut diagnostics);
                        }
                    }
                }
                "super_interfaces" | "extends_interfaces" => {
                    // implements clause (class) or extends clause (interface)
                    // contains keyword + type_list
                    let mut inner = child.walk();
                    for sc in child.children(&mut inner) {
                        if sc.kind() == "type_list" {
                            self.check_type_list(source, &sc, &mut diagnostics);
                        }
                    }
                }
                "type_parameters" => {
                    self.check_type_parameters_node(source, &child, &mut diagnostics);
                }
                _ => {}
            }
        }

        diagnostics
    }

    fn check_annotation_element(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        // Check member modifier filter
        if !self.member_modifiers.is_empty() && !self.has_matching_modifier(node, source) {
            return vec![];
        }

        if let Some(type_node) = node.child_by_field_name("type") {
            self.check_type_node(source, &type_node, &mut diagnostics);
        }

        diagnostics
    }

    fn check_type_list(&self, source: &str, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.check_type_node(source, &child, diagnostics);
            }
        }
    }

    fn check_type_parameters_node(
        &self,
        source: &str,
        node: &Node,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_parameter" {
                let mut inner = child.walk();
                for param_child in child.children(&mut inner) {
                    match param_child.kind() {
                        "type_identifier" => {
                            // The type parameter name (e.g., "Foo" in <Foo extends Bar>)
                            let name = &source[param_child.start_byte()..param_child.end_byte()];
                            if self.is_illegal_type(name) {
                                let range = CstNode::new(param_child, source).range();
                                diagnostics.push(Diagnostic::new(
                                    IllegalTypeViolation {
                                        name: name.to_string(),
                                    },
                                    range,
                                ));
                            }
                        }
                        "type_bound" => {
                            let mut bound_cursor = param_child.walk();
                            for bound_type in param_child.children(&mut bound_cursor) {
                                if bound_type.is_named() {
                                    self.check_type_node(source, &bound_type, diagnostics);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Check if a type name is illegal (either in illegal_class_names or matches abstract format).
    fn is_illegal_type(&self, name: &str) -> bool {
        if self.illegal_class_names.contains(name) {
            return true;
        }
        if self.validate_abstract_class_names
            && self.illegal_abstract_class_name_format.is_match(name)
            && !self.legal_abstract_class_names.contains(name)
        {
            return true;
        }
        false
    }

    /// Check type arguments in method invocations and method references.
    /// Note: memberModifiers filter does not apply to METHOD_CALL/METHOD_REF tokens.
    fn check_type_arguments_in_expr(&self, source: &str, node: &Node) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "type_arguments" {
                self.check_type_arguments(source, &child, &mut diagnostics);
            }
        }

        // For method_reference, also check the type part (e.g., Foo<Boolean> in Foo<Boolean>::method)
        if node.kind() == "method_reference" {
            let mut cursor2 = node.walk();
            for child in node.children(&mut cursor2) {
                if child.kind() == "generic_type"
                    || child.kind() == "type_identifier"
                    || child.kind() == "scoped_type_identifier"
                {
                    // Only check type_arguments within generic_type, not the base type itself
                    if child.kind() == "generic_type" {
                        let mut inner = child.walk();
                        for gc in child.children(&mut inner) {
                            if gc.kind() == "type_arguments" {
                                self.check_type_arguments(source, &gc, &mut diagnostics);
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Recursively check a type node for illegal types.
    fn check_type_node(&self, source: &str, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        match node.kind() {
            "type_identifier" => {
                let name = &source[node.start_byte()..node.end_byte()];
                if self.is_illegal_type(name) {
                    let range = CstNode::new(*node, source).range();
                    diagnostics.push(Diagnostic::new(
                        IllegalTypeViolation {
                            name: name.to_string(),
                        },
                        range,
                    ));
                }
            }
            "scoped_type_identifier" => {
                // Build fully qualified name
                let full_name = &source[node.start_byte()..node.end_byte()];
                if self.is_illegal_type(full_name) {
                    let range = CstNode::new(*node, source).range();
                    diagnostics.push(Diagnostic::new(
                        IllegalTypeViolation {
                            name: full_name.to_string(),
                        },
                        range,
                    ));
                } else if let Some(last) = node.child_by_field_name("name") {
                    // Also check the last component (simple name)
                    let simple_name = &source[last.start_byte()..last.end_byte()];
                    if self.is_illegal_type(simple_name) {
                        let range = CstNode::new(last, source).range();
                        diagnostics.push(Diagnostic::new(
                            IllegalTypeViolation {
                                name: simple_name.to_string(),
                            },
                            range,
                        ));
                    }
                }
            }
            "generic_type" => {
                // Check the base type
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "type_identifier" | "scoped_type_identifier" => {
                            self.check_type_node(source, &child, diagnostics);
                        }
                        "type_arguments" => {
                            self.check_type_arguments(source, &child, diagnostics);
                        }
                        _ => {}
                    }
                }
            }
            "array_type" => {
                // Check element type
                if let Some(element) = node.child_by_field_name("element") {
                    self.check_type_node(source, &element, diagnostics);
                }
            }
            "wildcard" => {
                // Check bound type (? extends X or ? super X)
                // tree-sitter wildcard children: ?, extends/super keyword, type
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.is_named() && child.kind() != "super" && child.kind() != "extends" {
                        self.check_type_node(source, &child, diagnostics);
                    }
                }
            }
            _ => {}
        }
    }

    fn check_type_arguments(&self, source: &str, node: &Node, diagnostics: &mut Vec<Diagnostic>) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                self.check_type_node(source, &child, diagnostics);
            }
        }
    }

    fn check_abstract_class_name(
        &self,
        source: &str,
        node: &Node,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Only check class declarations
        if node.kind() != "class_declaration" {
            return;
        }

        // Check if class has abstract modifier
        let is_abstract = has_modifier(node, source, "abstract");
        if !is_abstract {
            return;
        }

        if let Some(name_node) = node.child_by_field_name("name") {
            let name = &source[name_node.start_byte()..name_node.end_byte()];

            // Skip if in legal abstract class names
            if self.legal_abstract_class_names.contains(name) {
                return;
            }

            // Check if name matches the required format
            if !self.illegal_abstract_class_name_format.is_match(name) {
                let range = CstNode::new(name_node, source).range();
                diagnostics.push(Diagnostic::new(
                    IllegalAbstractClassNameViolation {
                        name: name.to_string(),
                        format: self.illegal_abstract_class_name_format_str.clone(),
                    },
                    range,
                ));
            }
        }
    }

    /// Check if a node has a modifier matching the member_modifiers filter.
    fn has_matching_modifier(&self, node: &Node, source: &str) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modifiers" {
                let mut mod_cursor = child.walk();
                for modifier in child.children(&mut mod_cursor) {
                    let mod_text = &source[modifier.start_byte()..modifier.end_byte()];
                    if self.member_modifiers.contains(mod_text) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Check if a node has the @Override annotation.
fn has_override_annotation(node: &Node, source: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                if modifier.kind() == "marker_annotation" || modifier.kind() == "annotation" {
                    let text = &source[modifier.start_byte()..modifier.end_byte()];
                    if text == "@Override" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Check if a node has a specific modifier keyword.
fn has_modifier(node: &Node, source: &str, keyword: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut mod_cursor = child.walk();
            for modifier in child.children(&mut mod_cursor) {
                let text = &source[modifier.start_byte()..modifier.end_byte()];
                if text == keyword {
                    return true;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str, rule: &IllegalType) -> Vec<(usize, String)> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let line_index = LineIndex::from_source_text(source);
        let source_code = SourceCode::new(source, &line_index);

        let mut violations = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            for d in rule.check(&ctx, &node) {
                let loc = source_code.line_column(d.range.start());
                violations.push((loc.line.get(), d.kind.body.clone()));
            }
        }
        violations
    }

    #[test]
    fn test_default_illegal_types() {
        let source = r#"
class Test {
    private HashMap<String, String> map;
    private TreeSet<String> set;
}
"#;
        let rule = IllegalType::default();
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 2);
    }

    #[test]
    fn test_override_ignored() {
        let source = r#"
class Test {
    @Override
    public HashMap<String, String> foo() { return null; }
}
"#;
        let rule = IllegalType::default();
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_ignored_method_name() {
        let source = r#"
class Test {
    private TreeSet getEnvironment() { return null; }
}
"#;
        let rule = IllegalType::default();
        let violations = check_source(source, &rule);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_fully_qualified_name() {
        let source = r#"
class Test {
    private java.util.TreeSet table1() { return null; }
}
"#;
        let rule = IllegalType::default();
        let violations = check_source(source, &rule);
        assert_eq!(violations.len(), 1);
    }
}
