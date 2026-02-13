//! Indentation rule implementation.
//!
//! Checks correct indentation of Java code. This is a port of the
//! checkstyle Indentation check for 100% compatibility.

pub mod handlers;
pub mod indent_level;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

pub use handlers::{HandlerContext, IndentHandler};
pub use indent_level::IndentLevel;

/// Default indentation amount (matches checkstyle).
const DEFAULT_INDENTATION: i32 = 4;

/// Configuration for Indentation rule.
#[derive(Debug, Clone)]
pub struct Indentation {
    /// How far new indentation level should be indented when on the next line.
    pub basic_offset: i32,
    /// How far a brace should be indented when on the next line.
    pub brace_adjustment: i32,
    /// How far a case label should be indented when on next line.
    pub case_indent: i32,
    /// How far a throws clause should be indented when on next line.
    pub throws_indent: i32,
    /// How far an array initialization should be indented when on next line.
    pub array_init_indent: i32,
    /// How far continuation line should be indented when line-wrapping is present.
    pub line_wrapping_indentation: i32,
    /// Force strict indent level in line wrapping case.
    pub force_strict_condition: bool,
    /// The width of a tab character.
    pub tab_width: usize,
}

impl Default for Indentation {
    fn default() -> Self {
        Self {
            basic_offset: DEFAULT_INDENTATION,
            brace_adjustment: 0,
            case_indent: DEFAULT_INDENTATION,
            throws_indent: DEFAULT_INDENTATION,
            array_init_indent: DEFAULT_INDENTATION,
            line_wrapping_indentation: DEFAULT_INDENTATION,
            force_strict_condition: false,
            tab_width: 4,
        }
    }
}

impl FromConfig for Indentation {
    const MODULE_NAME: &'static str = "Indentation";

    fn from_config(properties: &Properties) -> Self {
        Self {
            basic_offset: properties
                .get("basicOffset")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_INDENTATION),
            brace_adjustment: properties
                .get("braceAdjustment")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            case_indent: properties
                .get("caseIndent")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_INDENTATION),
            throws_indent: properties
                .get("throwsIndent")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_INDENTATION),
            array_init_indent: properties
                .get("arrayInitIndent")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_INDENTATION),
            line_wrapping_indentation: properties
                .get("lineWrappingIndentation")
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_INDENTATION),
            force_strict_condition: properties
                .get("forceStrictCondition")
                .map(|v| *v == "true")
                .unwrap_or(false),
            tab_width: properties
                .get("tabWidth")
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
        }
    }
}

const RELEVANT_KINDS: &[&str] = &["program"];

impl Rule for Indentation {
    fn name(&self) -> &'static str {
        "Indentation"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // The indentation check is unique in that it needs to process
        // the entire file structure rather than individual nodes.
        // We only run on the program root to avoid duplicate checks.
        if node.kind() != "program" {
            return vec![];
        }

        let handler_ctx = HandlerContext::new(ctx.source(), self, self.tab_width);

        // Start with indent level 0 for the program root
        let root_indent = IndentLevel::new(0);

        // Check the root children
        self.check_program(&handler_ctx, node, &root_indent);

        handler_ctx.take_diagnostics()
    }
}

impl Indentation {
    /// Check indentation of program-level elements.
    fn check_program(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        for child in node.children() {
            match child.kind() {
                "package_declaration" => self.check_package_declaration(ctx, &child, indent),
                "import_declaration" => self.check_import_declaration(ctx, &child, indent),
                "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "annotation_type_declaration"
                | "record_declaration" => {
                    self.check_class_declaration(ctx, &child, indent);
                }
                _ => {}
            }
        }
    }

    /// Check indentation of package declaration.
    fn check_package_declaration(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        // Find the 'package' keyword - the node may start earlier if there are annotations
        let package_keyword = self.find_child(node, "package");
        let check_node = package_keyword.as_ref().unwrap_or(node);

        if ctx.is_on_start_of_line(check_node) {
            let actual = ctx.get_line_start(self.line_no(ctx, check_node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(check_node, "package def", actual, indent);
            }
        }

        // Check if the package name (scoped_identifier or identifier) is on a continuation line
        let package_line = self.line_no(ctx, check_node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Check ALL continuation lines within the package name.
        // A scoped_identifier can span multiple lines, so we collect the first
        // leaf node on each continuation line and check its indent.
        let mut checked_lines = std::collections::HashSet::new();
        for child in node.children() {
            if matches!(child.kind(), "scoped_identifier" | "identifier") {
                self.check_package_name_lines(
                    ctx,
                    &child,
                    package_line,
                    &line_wrapped_indent,
                    &mut checked_lines,
                );
            }
        }
    }

    /// Recursively check continuation lines within a package name node.
    fn check_package_name_lines(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        package_line: usize,
        expected: &IndentLevel,
        checked_lines: &mut std::collections::HashSet<usize>,
    ) {
        if node.children().next().is_none() {
            // Leaf node
            let line = self.line_no(ctx, node);
            if line > package_line && checked_lines.insert(line) && ctx.is_on_start_of_line(node) {
                let actual = ctx.get_line_start(line);
                if !ctx.is_indent_acceptable(actual, expected) {
                    ctx.log_child_error(node, "package def", actual, expected);
                }
            }
        } else {
            for child in node.children() {
                self.check_package_name_lines(ctx, &child, package_line, expected, checked_lines);
            }
        }
    }

    /// Check indentation of import declaration.
    fn check_import_declaration(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "import", actual, indent);
            }
        }

        // Check continuation lines in import name (e.g., import java.util\n  .RandomAccess;)
        let import_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);
        let mut checked_lines = std::collections::HashSet::new();
        for child in node.children() {
            if matches!(child.kind(), "scoped_identifier" | "identifier") {
                self.check_package_name_lines(
                    ctx,
                    &child,
                    import_line,
                    &line_wrapped_indent,
                    &mut checked_lines,
                );
            }
        }
    }

    /// Check indentation of class/interface/enum/annotation/record declaration.
    fn check_class_declaration(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        let type_name = match node.kind() {
            "class_declaration" => "class def",
            "interface_declaration" => "interface def",
            "enum_declaration" => "enum def",
            "annotation_type_declaration" => "annotation def",
            "record_declaration" => "record def",
            _ => "type def",
        };

        // Check the class keyword/modifiers indentation
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, type_name, actual, indent);
            }
        }

        // When the declaration starts with a javadoc comment, the node begins on the
        // javadoc line. Also check the first keyword modifier's line if it differs.
        if let Some(mods) = self.find_child(node, "modifiers")
            && let Some(first_keyword) = mods.children().find(|c| {
                !matches!(
                    c.kind(),
                    "annotation" | "marker_annotation" | "line_comment" | "block_comment"
                )
            })
        {
            let kw_line = self.line_no(ctx, &first_keyword);
            if kw_line != self.line_no(ctx, node) && ctx.is_on_start_of_line(&first_keyword) {
                let actual = ctx.get_line_start(kw_line);
                if !ctx.is_indent_exact(actual, indent) {
                    ctx.log_error(&first_keyword, type_name, actual, indent);
                }
            }
        }

        let decl_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Check if the first token (modifier or keyword) is at wrong indent
        // If so, checkstyle uses base indent for class keyword and skips permits/extends
        let first_token_wrong = if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(decl_line);
            !ctx.is_indent_exact(actual, indent)
        } else {
            false
        };

        // Check if there's a non-annotation modifier (like sealed, non-sealed, public, etc.)
        // Only these trigger line wrapping for class keyword on continuation line
        // Also filter out comments which can appear in modifiers
        let has_keyword_modifier = self.find_child(node, "modifiers").is_some_and(|mods| {
            mods.children().any(|c| {
                !matches!(
                    c.kind(),
                    "annotation" | "marker_annotation" | "line_comment" | "block_comment"
                )
            })
        });

        // Check annotations in modifiers for argument list continuation lines
        // For type declarations, check each annotation's indent strictly
        if let Some(mods) = self.find_child(node, "modifiers") {
            self.check_modifiers_annotations(ctx, &mods, indent, true);
        }

        // For annotation type declarations, always check the identifier
        // since @interface Name can span multiple lines even without modifiers
        let is_annotation_type = node.kind() == "annotation_type_declaration";

        // Check class declaration parts on continuation lines
        for child in node.children() {
            match child.kind() {
                // class keyword on continuation line - only check if there's a keyword modifier
                "class" | "interface" | "enum" | "record" => {
                    if has_keyword_modifier || is_annotation_type {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > decl_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            // For @interface split across lines, check against base indent
                            let expected = if first_token_wrong || is_annotation_type {
                                indent.clone()
                            } else {
                                line_wrapped_indent.clone()
                            };
                            if !ctx.is_indent_acceptable(actual, &expected) {
                                ctx.log_error(&child, child.kind(), actual, &expected);
                            }
                        }
                    }
                }
                // @interface token for annotation type declarations
                "@interface" => {
                    // Check if @interface is on a continuation line (after modifiers)
                    // Only expect line-wrapped indent if there are keyword modifiers (public, etc.)
                    // When only annotations are present, @interface stays at base indent
                    // Use strict checking (is_indent_exact) since the keyword is structural
                    let child_line = self.line_no(ctx, &child);
                    if child_line > decl_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        let expected = if first_token_wrong || !has_keyword_modifier {
                            indent.clone()
                        } else {
                            line_wrapped_indent.clone()
                        };
                        if !ctx.is_indent_exact(actual, &expected) {
                            ctx.log_error(&child, "@interface", actual, &expected);
                        }
                    }
                }
                // class name identifier on continuation line
                // For regular classes/interfaces, only check if there's a keyword modifier
                // For annotation types, always check (since @interface\nName is valid syntax)
                "identifier" => {
                    if has_keyword_modifier || is_annotation_type {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > decl_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            // For annotation types: identifier should be at same indent as @interface
                            // For other types: use line wrapping from declaration start
                            // When first token is wrong, expect identifier at base indent
                            let expected = if first_token_wrong || is_annotation_type {
                                indent.clone()
                            } else {
                                line_wrapped_indent.clone()
                            };
                            // When first token is wrong or annotation type, use strict checking
                            let is_acceptable = if first_token_wrong || is_annotation_type {
                                expected.is_acceptable(actual)
                            } else {
                                ctx.is_indent_acceptable(actual, &expected)
                            };
                            if !is_acceptable {
                                ctx.log_child_error(&child, type_name, actual, &expected);
                            }
                        }
                    }
                }
                // permits clause on continuation line - skip if first token was wrong (unless forceStrict)
                "permits" => {
                    // When first_token_wrong, only check if forceStrictCondition is enabled
                    // But expected indent is base indent, not line_wrapped
                    let expected = if first_token_wrong {
                        indent.clone()
                    } else {
                        line_wrapped_indent.clone()
                    };
                    if !first_token_wrong || self.force_strict_condition {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > decl_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, &expected) {
                                ctx.log_error(&child, "permits", actual, &expected);
                            }
                        }
                        // Also check type_list inside permits for continuation
                        self.check_permits_type_list(ctx, &child, child_line, &expected);
                    }
                }
                // extends clause on continuation line - skip if first token was wrong (unless forceStrict)
                "superclass" => {
                    let expected = if first_token_wrong {
                        indent.clone()
                    } else {
                        line_wrapped_indent.clone()
                    };
                    if !first_token_wrong || self.force_strict_condition {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > decl_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, &expected) {
                                ctx.log_error(&child, "extends", actual, &expected);
                            }
                        }
                    }
                }
                // Record formal_parameters (the parentheses containing record components)
                "formal_parameters" => {
                    let expected = if first_token_wrong {
                        indent.clone()
                    } else {
                        line_wrapped_indent.clone()
                    };
                    // Check closing paren if on its own line
                    if let Some(rparen) = self.find_child(&child, ")") {
                        let rparen_line = self.line_no(ctx, &rparen);
                        if rparen_line > decl_line && ctx.is_on_start_of_line(&rparen) {
                            let actual = ctx.column_from_node(&rparen);
                            // Closing paren should be at base indent
                            if !ctx.is_indent_acceptable(actual, indent) {
                                ctx.log_error(&rparen, "rparen", actual, indent);
                            }
                        }
                    }
                    // Check lparen if on its own line (for nested records)
                    if let Some(lparen) = self.find_child(&child, "(") {
                        let lparen_line = self.line_no(ctx, &lparen);
                        if lparen_line > decl_line && ctx.is_on_start_of_line(&lparen) {
                            let actual = ctx.column_from_node(&lparen);
                            if !ctx.is_indent_acceptable(actual, &expected) {
                                ctx.log_error(&lparen, "lparen", actual, &expected);
                            }
                        }
                    }
                }
                // implements clause on continuation line
                "super_interfaces" => {
                    // Always check super_interfaces, even if first token was wrong
                    // When first token is wrong, use base indent as expected
                    let expected = if first_token_wrong {
                        indent.clone()
                    } else {
                        line_wrapped_indent.clone()
                    };
                    // Check implements keyword
                    if let Some(impl_kw) = self.find_child(&child, "implements") {
                        let impl_line = self.line_no(ctx, &impl_kw);
                        if impl_line > decl_line && ctx.is_on_start_of_line(&impl_kw) {
                            let actual = ctx.get_line_start(impl_line);
                            if !ctx.is_indent_acceptable(actual, &expected) {
                                ctx.log_error(&impl_kw, "implements", actual, &expected);
                            }
                        }
                    }
                    // Check type_list items on continuation lines
                    self.check_super_interfaces_type_list(ctx, &child, decl_line, &expected);
                }
                _ => {}
            }
        }

        // Check class body with increased indentation
        if let Some(body) = self
            .find_child(node, "class_body")
            .or_else(|| self.find_child(node, "interface_body"))
            .or_else(|| self.find_child(node, "enum_body"))
            .or_else(|| self.find_child(node, "annotation_type_body"))
            .or_else(|| self.find_child(node, "record_declaration_body"))
        {
            self.check_class_body(ctx, &body, indent);
        }
    }

    /// Check indentation of class body.
    fn check_class_body(&self, ctx: &HandlerContext, node: &CstNode, parent_indent: &IndentLevel) {
        // Check braces
        self.check_braces(ctx, node, parent_indent);

        // Children should be indented by basic_offset from parent
        let child_indent = parent_indent.with_offset(self.basic_offset);

        for child in node.children() {
            match child.kind() {
                "{" | "}" => {} // Skip braces, already checked
                // field_declaration in classes, constant_declaration in annotation types
                "field_declaration" | "constant_declaration" => {
                    self.check_member_def(ctx, &child, &child_indent);
                }
                "method_declaration" | "constructor_declaration" => {
                    self.check_method_def(ctx, &child, &child_indent);
                }
                "compact_constructor_declaration" => {
                    self.check_compact_constructor(ctx, &child, &child_indent);
                }
                "class_declaration"
                | "interface_declaration"
                | "enum_declaration"
                | "annotation_type_declaration"
                | "record_declaration" => {
                    self.check_class_declaration(ctx, &child, &child_indent);
                }
                "static_initializer" => self.check_static_init(ctx, &child, &child_indent),
                "block" => {
                    // Instance initializer block at class level
                    // Uses strict brace checking - brace must be at member indent, not adjusted
                    self.check_instance_init_block(ctx, &child, &child_indent);
                }
                "enum_constant" => self.check_enum_constant(ctx, &child, &child_indent),
                "annotation_type_element_declaration" => {
                    self.check_annotation_element(ctx, &child, &child_indent);
                }
                _ => {}
            }
        }
    }

    /// Check type_list inside permits clause for continuation line violations.
    fn check_permits_type_list(
        &self,
        ctx: &HandlerContext,
        permits_node: &CstNode,
        permits_line: usize,
        expected: &IndentLevel,
    ) {
        // Look for type_list child inside permits
        for child in permits_node.children() {
            if child.kind() == "type_list" {
                // Check each type_identifier on continuation lines
                for type_child in child.children() {
                    if type_child.kind() == "type_identifier" {
                        let child_line = self.line_no(ctx, &type_child);
                        if child_line > permits_line && ctx.is_on_start_of_line(&type_child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, expected) {
                                // Report using the type name
                                ctx.log_error(&type_child, type_child.text(), actual, expected);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check type_list inside super_interfaces (implements) clause for continuation line violations.
    fn check_super_interfaces_type_list(
        &self,
        ctx: &HandlerContext,
        super_interfaces_node: &CstNode,
        decl_line: usize,
        expected: &IndentLevel,
    ) {
        // Look for type_list child inside super_interfaces
        for child in super_interfaces_node.children() {
            if child.kind() == "type_list" {
                // Check each type_identifier on continuation lines
                for type_child in child.children() {
                    if matches!(
                        type_child.kind(),
                        "type_identifier" | "scoped_type_identifier" | "generic_type"
                    ) {
                        let child_line = self.line_no(ctx, &type_child);
                        if child_line > decl_line && ctx.is_on_start_of_line(&type_child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, expected) {
                                ctx.log_error(&type_child, "implements", actual, expected);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check indentation of braces.
    /// When `strict_brace_adjust` is true, braces MUST be at adjusted position.
    /// When false, braces can be at base indent OR adjusted position.
    fn check_braces(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        self.check_braces_with_strictness(ctx, node, indent, false);
    }

    /// Check indentation of braces with configurable strictness for brace adjustment.
    fn check_braces_with_strictness(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
        strict_brace_adjust: bool,
    ) {
        let brace_indent = indent.with_offset(self.brace_adjustment);
        // When strict_brace_adjust is true and braceAdjustment != 0, only accept adjusted position
        // Otherwise accept both base and adjusted
        let acceptable = if strict_brace_adjust && self.brace_adjustment != 0 {
            brace_indent.clone()
        } else {
            indent.combine(&brace_indent)
        };

        for child in node.children() {
            if matches!(child.kind(), "{" | "}") && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !ctx.is_indent_exact(actual, &acceptable) {
                    let brace_type = if child.kind() == "{" {
                        "block lcurly"
                    } else {
                        "block rcurly"
                    };
                    // Report with the expected brace indent
                    let expected = if strict_brace_adjust && self.brace_adjustment != 0 {
                        &brace_indent
                    } else {
                        indent
                    };
                    ctx.log_error(&child, brace_type, actual, expected);
                }
            }
        }
    }

    /// Check indentation of a field declaration.
    fn check_member_def(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "member def", actual, indent);
            }
        }

        // Check annotations in modifiers for argument list continuation lines
        // Member declarations use lenient annotation checking
        if let Some(mods) = self.find_child(node, "modifiers") {
            self.check_modifiers_annotations(ctx, &mods, indent, false);
        }

        // Check if the type is on a continuation line after non-annotation modifiers
        // e.g., public\n  int x; - the type should be at line-wrapped indent
        // But NOT for @Foo\n String x; - annotations on separate lines are not continuations
        let decl_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // First, check if we have non-annotation modifiers on the declaration line
        let has_modifier_on_decl_line = self.find_child(node, "modifiers").is_some_and(|mods| {
            mods.children().any(|m| {
                // Look for non-annotation modifiers (public, private, static, etc.)
                m.kind() != "annotation"
                    && m.kind() != "marker_annotation"
                    && self.line_no(ctx, &m) == decl_line
            })
        });

        if has_modifier_on_decl_line {
            for child in node.children() {
                // Type nodes (primitive_type, generic_type, array_type, type_identifier, etc.)
                if matches!(
                    child.kind(),
                    "integral_type"
                        | "floating_point_type"
                        | "boolean_type"
                        | "void_type"
                        | "type_identifier"
                        | "generic_type"
                        | "array_type"
                        | "scoped_type_identifier"
                ) {
                    let type_line = self.line_no(ctx, &child);
                    if type_line > decl_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(type_line);
                        if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                            ctx.log_error(&child, "member def", actual, &line_wrapped_indent);
                        }
                    }
                    break; // Only check the first type child
                }
            }
        }

        // Check array dimensions on continuation lines (e.g., List<?>[\n] or int variable2[\n])
        self.check_type_dimensions_continuation(ctx, node, indent);

        // Check variable declarator value continuation for field declarations
        // This handles line-wrapped initializers like:
        //   int[][] array
        //       = new int[][] { ... }
        self.check_variable_declaration_continuation(ctx, node, indent);
    }

    /// Check array dimensions that appear on continuation lines in type declarations.
    /// This handles patterns like `List<?> [\n]` or `int variable\n[]`
    fn check_type_dimensions_continuation(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        let decl_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Check dimensions in array_type children
        for child in node.children() {
            if child.kind() == "array_type" {
                self.check_dimensions_in_node(ctx, &child, decl_line, &line_wrapped_indent);
            } else if child.kind() == "variable_declarator" {
                // Check dimensions in variable_declarator (C-style arrays like `int x[]`)
                self.check_dimensions_in_node(ctx, &child, decl_line, &line_wrapped_indent);
            }
        }
    }

    /// Check dimensions node within a type or declarator for continuation line violations.
    fn check_dimensions_in_node(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        decl_line: usize,
        line_wrapped_indent: &IndentLevel,
    ) {
        for child in node.children() {
            if child.kind() == "dimensions" {
                let dim_line = self.line_no(ctx, &child);
                if dim_line > decl_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(dim_line);
                    if !ctx.is_indent_acceptable(actual, line_wrapped_indent) {
                        ctx.log_error(&child, "member def", actual, line_wrapped_indent);
                    }
                } else {
                    // Check individual brackets within dimensions that are on continuation lines
                    for bracket in child.children() {
                        let bracket_line = self.line_no(ctx, &bracket);
                        if bracket_line > decl_line && ctx.is_on_start_of_line(&bracket) {
                            let actual = ctx.get_line_start(bracket_line);
                            if !ctx.is_indent_acceptable(actual, line_wrapped_indent) {
                                ctx.log_error(&bracket, "member def", actual, line_wrapped_indent);
                            }
                        }
                    }
                }
            } else if child.kind() == "generic_type" {
                // Recurse into generic types to find array_type
                self.check_dimensions_in_node(ctx, &child, decl_line, line_wrapped_indent);
            }
        }
    }

    /// Check indentation of compact constructor declaration (Java records).
    fn check_compact_constructor(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        // Check the declaration line (modifiers + identifier)
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "ctor def", actual, indent);
            }
        }

        // Check body block
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of method or constructor declaration.
    fn check_method_def(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        let type_name = if node.kind() == "constructor_declaration" {
            "ctor def"
        } else {
            "method def"
        };

        // Check method declaration line
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, type_name, actual, indent);
            }
        }

        let method_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Check annotations in modifiers for argument list continuation lines
        // Method declarations use lenient annotation checking
        if let Some(mods) = self.find_child(node, "modifiers") {
            self.check_modifiers_annotations(ctx, &mods, indent, false);
        }

        // Find the line of the first keyword modifier (not annotation or comment)
        // Only check continuation for parts that come after a keyword modifier on a different line
        let first_keyword_line = self.find_child(node, "modifiers").and_then(|mods| {
            mods.children()
                .filter(|c| {
                    !matches!(
                        c.kind(),
                        "annotation" | "marker_annotation" | "line_comment" | "block_comment"
                    )
                })
                .map(|c| self.line_no(ctx, &c))
                .next()
        });

        // If there's a keyword modifier, check for continuation from that line
        if let Some(keyword_line) = first_keyword_line {
            for child in node.children() {
                let child_line = self.line_no(ctx, &child);
                match child.kind() {
                    // Modifiers - check each KEYWORD modifier child for continuation (not annotations or comments)
                    "modifiers" => {
                        let mut is_first_keyword = true;
                        for mod_child in child.children() {
                            // Skip annotations and comments - only check keyword modifiers
                            if matches!(
                                mod_child.kind(),
                                "annotation"
                                    | "marker_annotation"
                                    | "line_comment"
                                    | "block_comment"
                            ) {
                                continue;
                            }
                            // Skip the first keyword modifier - only check subsequent ones
                            if is_first_keyword {
                                is_first_keyword = false;
                                continue;
                            }
                            let mod_line = self.line_no(ctx, &mod_child);
                            if mod_line > keyword_line && ctx.is_on_start_of_line(&mod_child) {
                                let actual = ctx.get_line_start(mod_line);
                                if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                    ctx.log_error(
                                        &mod_child,
                                        mod_child.kind(),
                                        actual,
                                        &line_wrapped_indent,
                                    );
                                }
                            }
                        }
                    }
                    // Return type on continuation line (after first keyword modifier)
                    "void_type"
                    | "type_identifier"
                    | "integral_type"
                    | "floating_point_type"
                    | "boolean_type"
                    | "generic_type"
                    | "array_type"
                    | "scoped_type_identifier" => {
                        if child_line > keyword_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                ctx.log_error(&child, child.kind(), actual, &line_wrapped_indent);
                            }
                        }
                    }
                    // Method name on continuation line (after first keyword modifier)
                    "identifier" => {
                        if child_line > keyword_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                ctx.log_error(&child, child.text(), actual, &line_wrapped_indent);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Check formal_parameters continuation lines
        if let Some(params) = self.find_child(node, "formal_parameters") {
            for child in params.children() {
                if matches!(
                    child.kind(),
                    "formal_parameter" | "spread_parameter" | "receiver_parameter"
                ) {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > method_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                            ctx.log_child_error(&child, type_name, actual, &line_wrapped_indent);
                        }
                    }
                }
            }
            // Check rparen of formal_parameters on its own line
            if let Some(rparen) = self.find_child(&params, ")") {
                let rparen_line = self.line_no(ctx, &rparen);
                if rparen_line > method_line && ctx.is_on_start_of_line(&rparen) {
                    let actual = ctx.column_from_node(&rparen);
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&rparen, "rparen", actual, indent);
                    }
                }
            }
        }

        // Check throws clause if present on a continuation line
        // Structure: method_declaration > throws(wrapper) > throws(keyword) + type_identifiers
        if let Some(throws_clause) = self.find_child(node, "throws") {
            // Find the throws keyword inside the wrapper to get its line
            let throws_keyword_line = throws_clause
                .children()
                .find(|c| c.kind() == "throws")
                .map(|kw| self.line_no(ctx, &kw))
                .unwrap_or(method_line);

            // Check throws keyword if on continuation line
            if throws_keyword_line > method_line {
                for child in throws_clause.children() {
                    if child.kind() == "throws" && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(throws_keyword_line);
                        let expected = indent.with_offset(self.throws_indent);
                        if !ctx.is_indent_acceptable(actual, &expected) {
                            ctx.log_error(&child, "throws", actual, &expected);
                        }
                    }
                }
            }

            // Check exception types on continuation lines
            for child in throws_clause.children() {
                if matches!(
                    child.kind(),
                    "type_identifier" | "scoped_type_identifier" | "generic_type"
                ) {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > throws_keyword_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        let expected = indent.with_offset(self.throws_indent);
                        if !ctx.is_indent_acceptable(actual, &expected) {
                            ctx.log_child_error(&child, "throws", actual, &expected);
                        }
                    }
                }
            }
        }

        // Check method body - can be "block" for methods or "constructor_body" for constructors
        if let Some(body) = self.find_child(node, "block") {
            self.check_block(ctx, &body, indent);
        } else if let Some(body) = self.find_child(node, "constructor_body") {
            self.check_constructor_body(ctx, &body, indent);
        }
    }

    /// Check indentation of constructor body.
    /// Similar to check_block but also handles explicit_constructor_invocation.
    fn check_constructor_body(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        parent_indent: &IndentLevel,
    ) {
        // Check braces
        self.check_braces(ctx, node, parent_indent);

        // Determine child indent accounting for braceAdjustment.
        // When the opening brace is on its own line and at the correct position
        // (parent + braceAdjustment), child indent = brace_col + basicOffset.
        let child_indent = if let Some(lcurly) = self.find_child(node, "{") {
            if ctx.is_on_start_of_line(&lcurly) && self.brace_adjustment != 0 {
                let brace_col = ctx.column_from_node(&lcurly);
                let expected_brace = parent_indent.with_offset(self.brace_adjustment);
                if ctx.is_indent_exact(brace_col, &expected_brace) {
                    IndentLevel::new(brace_col + self.basic_offset)
                } else {
                    parent_indent.with_offset(self.basic_offset)
                }
            } else {
                parent_indent.with_offset(self.basic_offset)
            }
        } else {
            parent_indent.with_offset(self.basic_offset)
        };

        for child in node.children() {
            match child.kind() {
                "{" | "}" => {} // Skip braces
                "explicit_constructor_invocation" => {
                    self.check_explicit_constructor_invocation(ctx, &child, &child_indent);
                }
                _ => self.check_statement(ctx, &child, &child_indent),
            }
        }
    }

    /// Check indentation of a block.
    /// `parent_line` is the line where the parent statement starts (for detecting continuation braces).
    fn check_block(&self, ctx: &HandlerContext, node: &CstNode, parent_indent: &IndentLevel) {
        self.check_block_with_parent_line(ctx, node, parent_indent, None);
    }

    /// Check indentation of a block with optional parent line info.
    /// When parent_line is provided and brace is on a different line, strict brace checking applies.
    fn check_block_with_parent_line(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        parent_indent: &IndentLevel,
        parent_line: Option<usize>,
    ) {
        // Determine if brace is on a continuation line (different line than parent)
        let lcurly = self.find_child(node, "{");
        let brace_on_continuation = parent_line.is_some_and(|parent_ln| {
            lcurly
                .as_ref()
                .is_some_and(|lc| self.line_no(ctx, lc) > parent_ln && ctx.is_on_start_of_line(lc))
        });

        // Check braces - use strict checking for continuation braces
        if brace_on_continuation {
            self.check_braces_with_strictness(ctx, node, parent_indent, true);
        } else {
            self.check_braces(ctx, node, parent_indent);
        }

        // Determine child indent based on brace position:
        // - If opening brace is on its own line AND at correct position, child indent is
        //   actual brace position + basicOffset
        // - If opening brace is at wrong position or on same line, use parent_indent + basicOffset
        //   (checkstyle uses expected parent, not wrong actual)
        let child_indent = if let Some(lcurly) = &lcurly {
            if ctx.is_on_start_of_line(lcurly) {
                let brace_col = ctx.column_from_node(lcurly);
                // When brace is on continuation line, expected position is parent + braceAdjustment
                // Otherwise expected position is just parent
                let expected_brace = if brace_on_continuation && self.brace_adjustment != 0 {
                    parent_indent.with_offset(self.brace_adjustment)
                } else {
                    parent_indent.clone()
                };
                // Check if brace is at correct position
                if ctx.is_indent_exact(brace_col, &expected_brace) {
                    // Brace at correct position - use actual position + basicOffset
                    IndentLevel::new(brace_col + self.basic_offset)
                } else {
                    // Brace at wrong position - use expected parent + basicOffset
                    parent_indent.with_offset(self.basic_offset)
                }
            } else {
                // Brace on same line - use parent + basicOffset
                parent_indent.with_offset(self.basic_offset)
            }
        } else {
            parent_indent.with_offset(self.basic_offset)
        };

        for child in node.children() {
            match child.kind() {
                "{" | "}" => {} // Skip braces
                _ => self.check_statement(ctx, &child, &child_indent),
            }
        }
    }

    /// Check indentation of a block inside a case statement.
    /// Uses strict brace adjustment checking when brace is on its own line.
    fn check_case_block(&self, ctx: &HandlerContext, node: &CstNode, case_indent: &IndentLevel) {
        // Determine if opening brace is on its own line
        let lcurly_on_own_line = self
            .find_child(node, "{")
            .is_some_and(|lcurly| ctx.is_on_start_of_line(&lcurly));

        if lcurly_on_own_line {
            // Brace on its own line: use strict adjustment checking
            // Braces must be at case + braceAdjustment
            self.check_braces_with_strictness(ctx, node, case_indent, true);
            // Child indent = case + braceAdjustment + basicOffset
            let child_indent = case_indent.with_offset(self.brace_adjustment + self.basic_offset);

            for child in node.children() {
                match child.kind() {
                    "{" | "}" => {} // Skip braces
                    _ => self.check_statement(ctx, &child, &child_indent),
                }
            }
        } else {
            // Brace on same line as case (e.g., "case X: {")
            // Closing brace should align with case, body at case + basicOffset
            // This is like a normal block with no brace adjustment
            let brace_indent = case_indent.clone();

            // Check closing brace at case indent
            if let Some(rcurly) = self.find_child(node, "}")
                && ctx.is_on_start_of_line(&rcurly)
            {
                let actual = ctx.column_from_node(&rcurly);
                if !ctx.is_indent_exact(actual, &brace_indent) {
                    ctx.log_error(&rcurly, "block rcurly", actual, &brace_indent);
                }
            }

            // Child indent = case + basicOffset
            let child_indent = case_indent.with_offset(self.basic_offset);

            for child in node.children() {
                match child.kind() {
                    "{" | "}" => {} // Skip braces
                    _ => self.check_statement(ctx, &child, &child_indent),
                }
            }
        }
    }

    /// Check indentation of a statement.
    fn check_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        match node.kind() {
            "local_variable_declaration" => {
                if ctx.is_on_start_of_line(node) {
                    let actual = ctx.get_line_start(self.line_no(ctx, node));
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_child_error(node, "block", actual, indent);
                    }
                }
                // Check variable declarator value continuation.
                // This handles line-wrapped initializers with proper indent.
                self.check_variable_declaration_continuation(ctx, node, indent);
                // Note: We don't call check_expression here for the whole declaration
                // because check_variable_declaration_continuation already handles
                // line-wrapped initializers with the correct (line-wrapped) indent.
            }
            "expression_statement"
            | "return_statement"
            | "throw_statement"
            | "break_statement"
            | "continue_statement"
            | "assert_statement" => {
                if ctx.is_on_start_of_line(node) {
                    let actual = ctx.get_line_start(self.line_no(ctx, node));
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_child_error(node, "block", actual, indent);
                    }
                }
                // Check expressions within the statement
                self.check_expression(ctx, node, indent);
            }
            "explicit_constructor_invocation" => {
                self.check_explicit_constructor_invocation(ctx, node, indent);
            }
            "yield_statement" => self.check_yield_statement(ctx, node, indent),
            "if_statement" => self.check_if_statement(ctx, node, indent),
            "for_statement" | "enhanced_for_statement" => {
                self.check_for_statement(ctx, node, indent)
            }
            "while_statement" => self.check_while_statement(ctx, node, indent),
            "do_statement" => self.check_do_while_statement(ctx, node, indent),
            "try_statement" | "try_with_resources_statement" => {
                self.check_try_statement(ctx, node, indent);
            }
            "switch_expression" | "switch_statement" => {
                self.check_switch_statement(ctx, node, indent)
            }
            "synchronized_statement" => self.check_synchronized_statement(ctx, node, indent),
            "labeled_statement" => self.check_labeled_statement(ctx, node, indent),
            "block" => self.check_block(ctx, node, indent),
            "class_declaration" => {
                // Local class declaration inside a method block
                self.check_class_declaration(ctx, node, indent);
            }
            _ => {}
        }
    }

    /// Check indentation of variable declaration value on continuation line.
    fn check_variable_declaration_continuation(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        let decl_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Find variable_declarator children and check if their value is on a continuation line
        for child in node.children() {
            if child.kind() == "variable_declarator" {
                // Find the value (after the = sign), skipping comments
                let mut equals_node: Option<CstNode> = None;
                for declarator_child in child.children() {
                    if declarator_child.kind() == "=" {
                        equals_node = Some(declarator_child);
                        continue;
                    }
                    // Skip comments
                    if matches!(declarator_child.kind(), "line_comment" | "block_comment") {
                        continue;
                    }
                    if equals_node.is_some() {
                        // This is the value/initializer
                        // Check if either the = or the value is on a continuation line
                        let value_line = self.line_no(ctx, &declarator_child);
                        let mut is_line_wrapped =
                            value_line > decl_line && ctx.is_on_start_of_line(&declarator_child);

                        // Also check if the = is on a continuation line (e.g., x\n= value)
                        if let Some(eq) = &equals_node {
                            let eq_line = self.line_no(ctx, eq);
                            if eq_line > decl_line && ctx.is_on_start_of_line(eq) {
                                let actual = ctx.get_line_start(eq_line);
                                if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                    ctx.log_error(eq, "assign", actual, &line_wrapped_indent);
                                }
                                is_line_wrapped = true;
                            }
                        }

                        // For text blocks (multiline string literals with """), check both opening
                        // and closing delimiter positions when they're on continuation lines.
                        // Text blocks can be direct initializers or inside parenthesized expressions.
                        let text_block_node = if declarator_child.kind() == "string_literal"
                            && declarator_child.children().any(|c| c.kind() == "\"\"\"")
                        {
                            Some(declarator_child)
                        } else if declarator_child.kind() == "parenthesized_expression" {
                            // Check for text block inside parenthesized expression
                            declarator_child.children().find(|c| {
                                c.kind() == "string_literal"
                                    && c.children().any(|gc| gc.kind() == "\"\"\"")
                            })
                        } else {
                            None
                        };

                        let is_text_block = text_block_node.is_some();
                        if let Some(tb) = text_block_node {
                            let delimiters: Vec<_> =
                                tb.children().filter(|c| c.kind() == "\"\"\"").collect();

                            // Check opening """ if on continuation line
                            if let Some(open) = delimiters.first() {
                                let open_line = self.line_no(ctx, open);
                                if open_line > decl_line && ctx.is_on_start_of_line(open) {
                                    let actual = ctx.get_line_start(open_line);
                                    if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                        ctx.log_error(
                                            open,
                                            "text block",
                                            actual,
                                            &line_wrapped_indent,
                                        );
                                    }
                                }
                            }

                            // Check closing """ if on its own line
                            if let Some(close) = delimiters.last() {
                                let close_line = self.line_no(ctx, close);
                                if close_line > decl_line && ctx.is_on_start_of_line(close) {
                                    let actual = ctx.get_line_start(close_line);
                                    if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                        ctx.log_error(
                                            close,
                                            "text block",
                                            actual,
                                            &line_wrapped_indent,
                                        );
                                    }
                                }
                            }
                        }

                        if !is_text_block && is_line_wrapped {
                            let actual = ctx.get_line_start(value_line);
                            // For array initializers, also accept base indent (not just line-wrapped)
                            let acceptable = if declarator_child.kind() == "array_initializer" {
                                line_wrapped_indent.combine(indent)
                            } else {
                                line_wrapped_indent.clone()
                            };
                            if !ctx.is_indent_acceptable(actual, &acceptable) {
                                // Use the first token's kind for the error message
                                let kind = declarator_child.kind();
                                let label = match kind {
                                    "method_invocation" => kind,
                                    "object_creation_expression" => "new",
                                    "array_creation_expression" => "new",
                                    _ => kind,
                                };
                                ctx.log_error(
                                    &declarator_child,
                                    label,
                                    actual,
                                    &line_wrapped_indent,
                                );
                            }
                            // For line-wrapped initializers, check nested expressions.
                            // Array initializers can use statement indent for braces, so pass combined.
                            // Other expressions use line-wrapped indent.
                            let expr_indent = if declarator_child.kind() == "array_initializer" {
                                line_wrapped_indent.combine(indent)
                            } else {
                                line_wrapped_indent.clone()
                            };
                            self.check_expression(ctx, &declarator_child, &expr_indent);
                        } else if !is_text_block {
                            // For non-line-wrapped, non-text-block initializers, check with statement indent
                            // For array_creation_expression in variable init, use arrayInitIndent for elements
                            if declarator_child.kind() == "array_creation_expression" {
                                self.check_array_creation_expression_with_context(
                                    ctx,
                                    &declarator_child,
                                    indent,
                                    true, // in_variable_init
                                );
                            } else {
                                self.check_expression(ctx, &declarator_child, indent);
                            }
                        }
                        break;
                    }
                }
            }
        }
    }

    /// Check indentation of if statement.
    fn check_if_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'if' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "if", actual, indent);
            }
        }

        let if_line = self.line_no(ctx, node);

        // Check condition for patterns and expressions that may span lines
        if let Some(condition) = node.child_by_field_name("condition") {
            let line_wrapped = indent.with_offset(self.line_wrapping_indentation);

            // Check opening paren if on its own line - should be at line-wrapped indent
            if let Some(lparen) = condition.children().find(|c| c.kind() == "(") {
                let lparen_line = self.line_no(ctx, &lparen);
                if lparen_line > if_line && ctx.is_on_start_of_line(&lparen) {
                    let actual = ctx.column_from_node(&lparen);
                    if !ctx.is_indent_acceptable(actual, &line_wrapped) {
                        ctx.log_error(&lparen, "lparen", actual, &line_wrapped);
                    }
                }
            }

            // Check content inside the parenthesized condition if on its own line
            for child in condition.children() {
                if matches!(child.kind(), "(" | ")") {
                    continue;
                }
                let child_line = self.line_no(ctx, &child);
                if child_line > if_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(child_line);
                    if !ctx.is_indent_acceptable(actual, &line_wrapped) {
                        ctx.log_child_error(&child, "condition", actual, &line_wrapped);
                    }
                }
            }

            // For expression checking, use the actual position of the if statement if it's misaligned.
            // Checkstyle calculates continuation expected from where the statement actually is.
            let if_actual = ctx.get_line_start(if_line);
            let expr_base = if if_actual != indent.first_level() {
                IndentLevel::new(if_actual)
            } else {
                indent.clone()
            };
            self.check_expression(ctx, &condition, &expr_base);

            // Check closing paren if on its own line
            // Accept both indent (for `) {` on same line) and line-wrapped indent
            // Use exact matching — rparen must be at exactly one of the accepted positions
            if let Some(rparen) = condition.children().find(|c| c.kind() == ")") {
                let rparen_line = self.line_no(ctx, &rparen);
                if rparen_line > if_line && ctx.is_on_start_of_line(&rparen) {
                    let actual = ctx.column_from_node(&rparen);
                    let acceptable = indent.combine(&line_wrapped);
                    if !ctx.is_indent_exact(actual, &acceptable) {
                        ctx.log_error(&rparen, "rparen", actual, indent);
                    }
                }
            }
        }

        // Check consequence (then branch) - pass if_line for continuation brace detection
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if consequence.kind() == "block" {
                self.check_block_with_parent_line(ctx, &consequence, indent, Some(if_line));
            } else {
                // Single statement - use lenient checking
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &consequence, &stmt_indent);
            }
        }

        // Check alternative (else branch)
        if let Some(alternative) = node.child_by_field_name("alternative") {
            // Find the 'else' keyword to determine if this is a same-line else-if
            let else_line = node
                .children()
                .find(|c| c.kind() == "else")
                .map(|e| self.line_no(ctx, &e));

            // Check 'else' keyword if present
            for child in node.children() {
                if child.kind() == "else" && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.column_from_node(&child);
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&child, "else", actual, indent);
                    }
                }
            }

            if alternative.kind() == "block" {
                // Use else_line for continuation brace detection
                let else_ln = else_line.unwrap_or(if_line);
                self.check_block_with_parent_line(ctx, &alternative, indent, Some(else_ln));
            } else if alternative.kind() == "if_statement" {
                let alt_line = self.line_no(ctx, &alternative);
                // Check if the 'if' is on the same line as 'else' (else if pattern)
                // or on a new line (indented if after else)
                if else_line == Some(alt_line) {
                    // Same line: "else if" - check at same level as original if
                    self.check_if_statement(ctx, &alternative, indent);
                } else {
                    // Different line: if is a statement after else, should be indented
                    let stmt_indent = indent.with_offset(self.basic_offset);
                    self.check_if_statement(ctx, &alternative, &stmt_indent);
                }
            } else {
                // Single statement - use lenient checking
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &alternative, &stmt_indent);
            }
        }
    }

    /// Check indentation of for statement.
    fn check_for_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'for' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "for", actual, indent);
            }
        }

        let for_line = self.line_no(ctx, node);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);

        // Check init/condition/update on continuation lines (including : for enhanced for)
        for child in node.children() {
            match child.kind() {
                // Skip for keyword and body (block or single statement like ';')
                "for" | "block" => continue,
                // For enhanced_for, the ';' is the body (empty statement), not a separator
                ";" if node.kind() == "enhanced_for_statement" => continue,
                // Check parens - should be at for's base indent when on own line
                "(" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > for_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_exact(actual, indent) {
                            ctx.log_error(&child, "for lparen", actual, indent);
                        }
                    }
                }
                ")" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > for_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_exact(actual, indent) {
                            ctx.log_error(&child, "for rparen", actual, indent);
                        }
                    }
                }
                // Semicolons on their own line expected at basicOffset (first indent level)
                ";" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > for_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        let semi_expected = IndentLevel::new(self.basic_offset);
                        if !ctx.is_indent_exact(actual, &semi_expected) {
                            ctx.log_error(&child, ";", actual, &semi_expected);
                        }
                    }
                }
                // Check : and other parts (init/condition/update/value)
                _ => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > for_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                            ctx.log_error(&child, "for", actual, &line_wrapped_indent);
                        }
                    }
                    // Check for nested binary expression continuations
                    if child.kind() == "binary_expression" {
                        let expr_line = self.line_no(ctx, &child);
                        self.check_nested_binary_continuation(
                            ctx,
                            &child,
                            expr_line,
                            &line_wrapped_indent,
                            "for",
                        );
                    }
                    // Check for semicolon inside local_variable_declaration (the init part)
                    if child.kind() == "local_variable_declaration" {
                        for decl_child in child.children() {
                            if decl_child.kind() == ";" {
                                let semi_line = self.line_no(ctx, &decl_child);
                                if semi_line > for_line && ctx.is_on_start_of_line(&decl_child) {
                                    let actual = ctx.get_line_start(semi_line);
                                    let semi_expected = IndentLevel::new(self.basic_offset);
                                    if !ctx.is_indent_exact(actual, &semi_expected) {
                                        ctx.log_error(&decl_child, ";", actual, &semi_expected);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check body - pass for_line for continuation brace detection
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block_with_parent_line(ctx, &body, indent, Some(for_line));
            } else {
                // Single-statement body - use lenient checking
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &body, &stmt_indent);
            }
        }
    }

    /// Check nested binary expression continuations.
    /// When a binary expression spans multiple lines, nested parts should have additional indentation.
    fn check_nested_binary_continuation(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        base_line: usize,
        parent_indent: &IndentLevel,
        label: &str,
    ) {
        let nested_indent = parent_indent.with_offset(self.line_wrapping_indentation);

        for child in node.children() {
            match child.kind() {
                // Check nested binary expressions recursively
                "binary_expression" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > base_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &nested_indent) {
                            ctx.log_child_error(&child, label, actual, &nested_indent);
                        }
                    }
                    // Recurse for deeper nesting
                    self.check_nested_binary_continuation(
                        ctx,
                        &child,
                        base_line,
                        &nested_indent,
                        label,
                    );
                }
                // Check operators on new lines
                "&&" | "||" | "+" | "-" | "*" | "/" | "%" | "&" | "|" | "^" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > base_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &nested_indent) {
                            ctx.log_child_error(&child, label, actual, &nested_indent);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Check indentation of while statement.
    fn check_while_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'while' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "while", actual, indent);
            }
        }

        let while_line = self.line_no(ctx, node);

        // Check condition for expressions that may span lines
        if let Some(condition) = node.child_by_field_name("condition") {
            // For expression checking, use the actual position of the while statement if it's misaligned.
            // Checkstyle calculates continuation expected from where the statement actually is.
            let while_actual = ctx.get_line_start(while_line);
            let expr_base = if while_actual != indent.first_level() {
                IndentLevel::new(while_actual)
            } else {
                indent.clone()
            };
            self.check_expression(ctx, &condition, &expr_base);

            // Check for closing paren of condition on its own line
            // The closing paren should be at statement indent
            if let Some(rparen) = condition.children().find(|c| c.kind() == ")") {
                let rparen_line = self.line_no(ctx, &rparen);
                if rparen_line > while_line && ctx.is_on_start_of_line(&rparen) {
                    let actual = ctx.column_from_node(&rparen);
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&rparen, "rparen", actual, indent);
                    }
                }
            }
        }

        // Check body - pass while_line for continuation brace detection
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block_with_parent_line(ctx, &body, indent, Some(while_line));
            } else {
                // Single-statement body - use lenient checking
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &body, &stmt_indent);
            }
        }
    }

    /// Check indentation of do-while statement.
    fn check_do_while_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'do' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "do", actual, indent);
            }
        }

        let do_line = self.line_no(ctx, node);

        // Check body - pass do_line for continuation brace detection
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block_with_parent_line(ctx, &body, indent, Some(do_line));
            } else {
                // Single-statement body (no braces) - use lenient checking
                // as it can be line-wrapped at various indents
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &body, &stmt_indent);
            }
        }

        // Find the 'while' keyword to get its line
        let while_line = node
            .children()
            .find(|c| c.kind() == "while")
            .map(|w| self.line_no(ctx, &w))
            .unwrap_or(do_line);

        // Check 'while' at end
        for child in node.children() {
            if child.kind() == "while" && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !ctx.is_indent_exact(actual, indent) {
                    ctx.log_error(&child, "while", actual, indent);
                }
            }
        }

        // Check condition for expressions that may span lines
        if let Some(condition) = node.child_by_field_name("condition") {
            // Check opening paren if on its own line
            if let Some(lparen) = condition.children().find(|c| c.kind() == "(") {
                let lparen_line = self.line_no(ctx, &lparen);
                if lparen_line > while_line && ctx.is_on_start_of_line(&lparen) {
                    let actual = ctx.column_from_node(&lparen);
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&lparen, "lparen", actual, indent);
                    }
                }
            }

            // Check content inside the parenthesized condition if on its own line
            // This catches cases like:  do {} while\n(\ntest\n);
            // where each part is on its own line
            for child in condition.children() {
                // Skip parens (handled separately)
                if matches!(child.kind(), "(" | ")") {
                    continue;
                }
                let child_line = self.line_no(ctx, &child);
                if child_line > while_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(child_line);
                    // Content should be at statement indent in lenient mode
                    if !ctx.is_indent_acceptable(actual, indent) {
                        ctx.log_child_error(&child, "condition", actual, indent);
                    }
                }
            }

            self.check_expression(ctx, &condition, indent);

            // Check for closing paren of condition on its own line
            if let Some(rparen) = condition.children().find(|c| c.kind() == ")") {
                let rparen_line = self.line_no(ctx, &rparen);
                if rparen_line > while_line && ctx.is_on_start_of_line(&rparen) {
                    let actual = ctx.column_from_node(&rparen);
                    // Closing paren should be at statement indent, not line-wrapped
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&rparen, "rparen", actual, indent);
                    }
                }
            }
        }
    }

    /// Check indentation of try statement.
    fn check_try_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'try' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "try", actual, indent);
            }
        }

        let try_line = self.line_no(ctx, node);

        // Check try-with-resources: try (resource; resource) { ... }
        if node.kind() == "try_with_resources_statement"
            && let Some(resources) = node.child_by_field_name("resources")
        {
            // Resources should be indented by lineWrappingIndentation from try
            let resource_indent = indent.with_offset(self.line_wrapping_indentation);

            // Check lparen of resources on its own line from 'try'
            if let Some(lparen) = self.find_child(&resources, "(") {
                let lparen_line = self.line_no(ctx, &lparen);
                if lparen_line > try_line && ctx.is_on_start_of_line(&lparen) {
                    let actual = ctx.column_from_node(&lparen);
                    let lparen_acceptable = indent.combine(&resource_indent);
                    if !lparen_acceptable.is_acceptable(actual) {
                        ctx.log_error(&lparen, "lparen", actual, indent);
                    }
                }
            }

            for child in resources.children() {
                if child.kind() == "resource" {
                    let child_line = self.line_no(ctx, &child);
                    let resource_on_continuation = child_line > try_line;
                    if resource_on_continuation && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        // Use lenient check - respects forceStrictCondition setting
                        if !ctx.is_indent_acceptable(actual, &resource_indent) {
                            ctx.log_child_error(&child, "try", actual, &resource_indent);
                        }
                    }
                    // Check resource value continuation after '='
                    // If resource is on try line: value at resource_indent
                    // If resource is on continuation: value at resource_indent + lineWrap
                    let value_expected = if resource_on_continuation {
                        resource_indent.with_offset(self.line_wrapping_indentation)
                    } else {
                        resource_indent.clone()
                    };
                    let mut found_eq = false;
                    for res_child in child.children() {
                        if res_child.kind() == "=" {
                            found_eq = true;
                            continue;
                        }
                        if found_eq && !matches!(res_child.kind(), "line_comment" | "block_comment")
                        {
                            let val_line = self.line_no(ctx, &res_child);
                            if val_line > child_line && ctx.is_on_start_of_line(&res_child) {
                                let actual = ctx.get_line_start(val_line);
                                if !ctx.is_indent_acceptable(actual, &value_expected) {
                                    ctx.log_child_error(
                                        &res_child,
                                        "try resource",
                                        actual,
                                        &value_expected,
                                    );
                                }
                            }
                            // Check nested expressions in value
                            self.check_expression(ctx, &res_child, &resource_indent);
                            break;
                        }
                    }
                }
            }

            // Check closing paren of resources if on its own line
            if let Some(rparen) = self.find_child(&resources, ")")
                && ctx.is_on_start_of_line(&rparen)
            {
                let actual = ctx.column_from_node(&rparen);
                // Closing paren can be at try indent or resource indent - use strict check
                let rparen_acceptable = indent.combine(&resource_indent);
                if !rparen_acceptable.is_acceptable(actual) {
                    ctx.log_error(&rparen, "rparen", actual, indent);
                }
            }
        }

        // Check try body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }

        // Check catch clauses
        for child in node.children() {
            if child.kind() == "catch_clause" {
                self.check_catch_clause(ctx, &child, indent);
            } else if child.kind() == "finally_clause" {
                self.check_finally_clause(ctx, &child, indent);
            }
        }
    }

    /// Check indentation of catch clause.
    fn check_catch_clause(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'catch' keyword
        let catch_line = self.line_no(ctx, node);
        for child in node.children() {
            if child.kind() == "catch" && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !ctx.is_indent_exact(actual, indent) {
                    ctx.log_error(&child, "catch", actual, indent);
                }
            }
        }

        // Check catch_formal_parameter indentation (for multi-catch and annotations)
        // Expected indent for catch parameters on continuation lines
        let param_indent = indent.with_offset(self.line_wrapping_indentation);

        for child in node.children() {
            if child.kind() == "catch_formal_parameter" {
                // Check modifiers (annotations) on new lines
                for param_child in child.children() {
                    match param_child.kind() {
                        "modifiers" => {
                            // Check annotations in modifiers — use exact matching
                            // (annotations in catch params must be exactly at param_indent)
                            for mod_child in param_child.children() {
                                let mod_line = self.line_no(ctx, &mod_child);
                                if mod_line > catch_line && ctx.is_on_start_of_line(&mod_child) {
                                    let actual = ctx.get_line_start(mod_line);
                                    if !ctx.is_indent_exact(actual, &param_indent) {
                                        ctx.log_child_error(
                                            &mod_child,
                                            "catch parameter",
                                            actual,
                                            &param_indent,
                                        );
                                    }
                                }
                            }
                        }
                        "catch_type" => {
                            // Check multi-catch | separators and type identifiers on new lines.
                            // When the first type is on the catch line, | continuations at param_indent.
                            // When first type is on a new line (lparen last on catch line), | continuations
                            // need param_indent + lineWrap (because they're continuations of the first type).
                            let first_type = param_child.children().find(|c| {
                                matches!(c.kind(), "type_identifier" | "scoped_type_identifier")
                            });
                            let first_type_on_new_line = first_type
                                .as_ref()
                                .is_some_and(|ft| self.line_no(ctx, ft) > catch_line);
                            let pipe_indent = if first_type_on_new_line {
                                param_indent.with_offset(self.line_wrapping_indentation)
                            } else {
                                param_indent.clone()
                            };

                            for type_child in param_child.children() {
                                let type_line = self.line_no(ctx, &type_child);
                                if type_line > catch_line && ctx.is_on_start_of_line(&type_child) {
                                    let actual = ctx.get_line_start(type_line);
                                    // Use pipe_indent for | separators and subsequent types,
                                    // param_indent for the first type on its own line
                                    let expected = if type_child.kind() == "|"
                                        || (first_type_on_new_line
                                            && first_type.as_ref().is_some_and(|ft| {
                                                self.line_no(ctx, ft) < type_line
                                            })) {
                                        &pipe_indent
                                    } else {
                                        &param_indent
                                    };
                                    if !ctx.is_indent_acceptable(actual, expected) {
                                        ctx.log_child_error(
                                            &type_child,
                                            "catch parameter",
                                            actual,
                                            expected,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Check catch body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of finally clause.
    fn check_finally_clause(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'finally' keyword
        for child in node.children() {
            if child.kind() == "finally" && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !ctx.is_indent_exact(actual, indent) {
                    ctx.log_error(&child, "finally", actual, indent);
                }
            }
        }

        // Check finally body — finally_clause has no named 'body' field in tree-sitter,
        // so we find the block child directly
        for child in node.children() {
            if child.kind() == "block" {
                self.check_block(ctx, &child, indent);
                break;
            }
        }
    }

    /// Check indentation of switch statement.
    fn check_switch_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Switch can be at statement position OR line-wrapped position (in expressions)
        // Acceptable positions: indent (statement) or indent + lineWrapping (line-wrapped expression)
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);
        let acceptable_indent = indent.combine(&line_wrapped_indent);

        let switch_line = self.line_no(ctx, node);
        let switch_actual = ctx.get_line_start(switch_line);

        // Check 'switch' keyword indentation
        // Use strict checking here - switch must be exactly at statement or line-wrapped position
        let switch_is_valid = if ctx.is_on_start_of_line(node) {
            let is_valid = ctx.is_indent_exact(switch_actual, &acceptable_indent);
            if !is_valid {
                // Report with line-wrapped expected indent since that's what checkstyle expects
                // for switch expressions (the common case for line-wrapped switches)
                ctx.log_error(node, "switch", switch_actual, &line_wrapped_indent);
            }
            is_valid
        } else {
            true // Not on start of line, considered valid
        };

        // Check switch condition/expression on continuation line from 'switch'
        if let Some(condition) = node.child_by_field_name("condition") {
            // Check children of parenthesized_expression for wrapped condition
            for cond_child in condition.children() {
                if matches!(cond_child.kind(), "(" | ")") {
                    continue;
                }
                let cond_line = self.line_no(ctx, &cond_child);
                if cond_line > switch_line && ctx.is_on_start_of_line(&cond_child) {
                    let actual = ctx.get_line_start(cond_line);
                    let expected = indent.with_offset(self.line_wrapping_indentation);
                    if !ctx.is_indent_acceptable(actual, &expected) {
                        ctx.log_child_error(&cond_child, "switch", actual, &expected);
                    }
                }
            }
        }

        // Determine base indent for switch body:
        // - If switch is at a valid position, use its actual position
        // - If switch is at an invalid position, use the expected statement position
        let body_base_indent = if switch_is_valid {
            IndentLevel::new(switch_actual)
        } else {
            // Use expected statement position for consistency with checkstyle
            indent.clone()
        };

        // Check switch body/block
        // Try "body" field first (switch_statement), then look for switch_block child (switch_expression)
        let body = node
            .child_by_field_name("body")
            .or_else(|| self.find_child(node, "switch_block"));
        if let Some(body) = body {
            self.check_switch_body(ctx, &body, &body_base_indent);
        }
    }

    /// Check indentation of switch body.
    fn check_switch_body(&self, ctx: &HandlerContext, node: &CstNode, parent_indent: &IndentLevel) {
        // Check braces
        self.check_braces(ctx, node, parent_indent);

        // Case labels are indented by case_indent from switch
        let case_indent = parent_indent.with_offset(self.case_indent);
        // Case body is indented by basic_offset from case
        let body_indent = case_indent.with_offset(self.basic_offset);

        for child in node.children() {
            match child.kind() {
                "switch_block_statement_group" => {
                    self.check_switch_group(ctx, &child, &case_indent, &body_indent);
                }
                "switch_rule" => {
                    self.check_switch_rule(ctx, &child, &case_indent);
                }
                _ => {}
            }
        }
    }

    /// Check indentation of switch statement group (case: ... statements).
    fn check_switch_group(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        case_indent: &IndentLevel,
        body_indent: &IndentLevel,
    ) {
        for child in node.children() {
            match child.kind() {
                "switch_label" => {
                    let label_line = self.line_no(ctx, &child);
                    if ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(label_line);
                        if !ctx.is_indent_exact(actual, case_indent) {
                            ctx.log_error(&child, "case", actual, case_indent);
                        }
                    } else if let Some(parent) = node.parent() {
                        // Check for block comment preceding the case at switch_block level.
                        // In `/* hello */ case 5:`, the block_comment is a sibling of the
                        // switch_block_statement_group in the switch_block parent.
                        let has_preceding_comment = parent.children().any(|sibling| {
                            sibling.kind() == "block_comment"
                                && self.line_no(ctx, &sibling) == label_line
                        });
                        if has_preceding_comment {
                            let actual = ctx.get_line_start(label_line);
                            if !ctx.is_indent_exact(actual, case_indent) {
                                ctx.log_error(&child, "case", actual, case_indent);
                            }
                        }
                    }
                    // Check if label constant/value is on a continuation line from 'case'
                    let label_wrap = case_indent.with_offset(self.line_wrapping_indentation);
                    for label_child in child.children() {
                        if matches!(label_child.kind(), "case" | "default" | ":" | ",") {
                            continue;
                        }
                        let lc_line = self.line_no(ctx, &label_child);
                        if lc_line > label_line && ctx.is_on_start_of_line(&label_child) {
                            let actual = ctx.get_line_start(lc_line);
                            if !ctx.is_indent_acceptable(actual, &label_wrap) {
                                ctx.log_child_error(&label_child, "case", actual, &label_wrap);
                            }
                        }
                    }
                }
                "block" => {
                    // Case block: braces should be at case + braceAdjustment,
                    // body at case + braceAdjustment + basicOffset.
                    // Use strict brace checking since this is an explicit user block.
                    self.check_case_block(ctx, &child, case_indent);
                }
                _ => self.check_statement(ctx, &child, body_indent),
            }
        }
    }

    /// Check indentation of switch rule (case -> expr/block).
    fn check_switch_rule(&self, ctx: &HandlerContext, node: &CstNode, case_indent: &IndentLevel) {
        // Check case label
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, case_indent) {
                ctx.log_error(node, "case", actual, case_indent);
            }
        }

        let case_line = self.line_no(ctx, node);
        let continuation_indent = case_indent.with_offset(self.line_wrapping_indentation);

        // Track if arrow is on continuation line - body will need extra indent
        let arrow_on_continuation =
            node.children()
                .find(|c| c.kind() == "->")
                .is_some_and(|arrow| {
                    self.line_no(ctx, &arrow) > case_line && ctx.is_on_start_of_line(&arrow)
                });

        // Body indent depends on whether arrow is on continuation line
        // - Arrow on continuation: body uses lineWrappingIndentation from arrow position
        // - Arrow on same line: body uses basicOffset from case position (like block content)
        let body_continuation_indent = if arrow_on_continuation {
            continuation_indent.with_offset(self.line_wrapping_indentation)
        } else {
            case_indent.with_offset(self.basic_offset)
        };

        // Check the children (switch_label and body)
        for child in node.children() {
            match child.kind() {
                "switch_label" => {
                    // Check patterns in the switch label
                    self.check_switch_label_patterns(ctx, &child, case_indent, case_line);
                }
                // Arrow on continuation line
                "->" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > case_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &continuation_indent) {
                            ctx.log_error(&child, "lambda", actual, &continuation_indent);
                        }
                    }
                }
                "block" => {
                    self.check_block(ctx, &child, case_indent);
                }
                // Body expression on continuation line (not a block)
                // Should be indented by basicOffset from case (or lineWrap from arrow)
                "expression_statement" | "throw_statement" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > case_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &body_continuation_indent) {
                            ctx.log_child_error(&child, "case", actual, &body_continuation_indent);
                        }
                    }
                    // Check for nested switch_expression in the body
                    for body_child in child.children() {
                        if body_child.kind() == "switch_expression" {
                            self.check_nested_switch_in_case_arrow(
                                ctx,
                                &body_child,
                                &continuation_indent,
                            );
                        }
                    }
                }
                // Switch expression as direct case body (nested switch expressions)
                "switch_expression" => {
                    self.check_nested_switch_in_case_arrow(ctx, &child, &continuation_indent);
                }
                _ => {}
            }
        }
    }

    /// Check a nested switch_expression inside a case arrow body.
    /// Unlike top-level switches, these should be at exactly case_indent + lineWrappingIndentation.
    fn check_nested_switch_in_case_arrow(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        expected_indent: &IndentLevel,
    ) {
        let switch_line = self.line_no(ctx, node);

        // Check switch keyword position
        let switch_valid = if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(switch_line);
            let valid = ctx.is_indent_exact(actual, expected_indent);
            if !valid {
                ctx.log_error(node, "switch", actual, expected_indent);
            }
            valid
        } else {
            true
        };

        // Base indent for switch body: use actual position if valid, expected if invalid
        let body_base = if switch_valid {
            IndentLevel::new(ctx.get_line_start(switch_line))
        } else {
            expected_indent.clone()
        };

        // Check the switch body
        if let Some(body) = node
            .child_by_field_name("body")
            .or_else(|| self.find_child(node, "switch_block"))
        {
            self.check_switch_body(ctx, &body, &body_base);
        }
    }

    /// Check indentation of patterns in switch labels.
    fn check_switch_label_patterns(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        case_indent: &IndentLevel,
        case_line: usize,
    ) {
        // Pattern components should be indented by line_wrapping from case
        let pattern_indent = case_indent.with_offset(self.line_wrapping_indentation);
        let min_expected = pattern_indent.first_level();

        for child in node.children() {
            match child.kind() {
                // The grammar has: switch_label -> pattern -> record_pattern
                "pattern" => {
                    // Recurse into the pattern wrapper
                    self.check_switch_label_patterns(ctx, &child, case_indent, case_line);
                }
                "record_pattern" | "type_pattern" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line == case_line {
                        // Pattern on same line as case - check nested components
                        self.check_pattern(ctx, &child, &pattern_indent, case_line);
                    } else if ctx.is_on_start_of_line(&child) {
                        // Pattern on continuation line - checkstyle accepts >= expected
                        let actual = ctx.get_line_start(child_line);
                        if actual < min_expected {
                            ctx.log_child_error(&child, "case", actual, &pattern_indent);
                        }
                    }
                }
                // guard contains the 'when' clause
                "guard" => {
                    self.check_guard_continuation(
                        ctx,
                        &child,
                        case_line,
                        min_expected,
                        &pattern_indent,
                    );
                }
                _ => {}
            }
        }
    }

    /// Check indentation of guard (when clause) and its contents on continuation lines.
    fn check_guard_continuation(
        &self,
        ctx: &HandlerContext,
        guard_node: &CstNode,
        case_line: usize,
        min_expected: i32,
        expected_indent: &IndentLevel,
    ) {
        // Check guard itself if on continuation line
        let guard_line = self.line_no(ctx, guard_node);
        if guard_line > case_line && ctx.is_on_start_of_line(guard_node) {
            let actual = ctx.get_line_start(guard_line);
            if actual < min_expected {
                ctx.log_child_error(guard_node, "case", actual, expected_indent);
            }
        }

        // Check guard's children (when keyword, condition expression) on continuation lines
        for child in guard_node.children() {
            let child_line = self.line_no(ctx, &child);
            if child_line > guard_line && ctx.is_on_start_of_line(&child) {
                let actual = ctx.get_line_start(child_line);
                if actual < min_expected {
                    ctx.log_child_error(&child, "case", actual, expected_indent);
                }
            }
        }
    }

    /// Check indentation of synchronized statement.
    fn check_synchronized_statement(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        let sync_line = self.line_no(ctx, node);

        // Check 'synchronized' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(sync_line);
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "synchronized", actual, indent);
            }
        }

        // Check parenthesized expression for line wrapping
        for child in node.children() {
            if child.kind() == "parenthesized_expression" {
                let paren_line = self.line_no(ctx, &child);
                let line_wrapped = indent.with_offset(self.line_wrapping_indentation);

                // If the parenthesized expression starts on a different line from synchronized,
                // check the opening paren at the synchronized keyword's indent
                if paren_line > sync_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(paren_line);
                    if !ctx.is_indent_exact(actual, indent) {
                        ctx.log_error(&child, "synchronized lparen", actual, indent);
                    }
                }

                // When parens are on the same line as synchronized, check expression content
                // on continuation lines at indent + lineWrappingIndentation
                if paren_line == sync_line {
                    for inner in child.children() {
                        if matches!(inner.kind(), "(" | ")") {
                            continue;
                        }
                        let inner_line = self.line_no(ctx, &inner);
                        if inner_line > sync_line && ctx.is_on_start_of_line(&inner) {
                            let actual = ctx.get_line_start(inner_line);
                            if !ctx.is_indent_acceptable(actual, &line_wrapped) {
                                ctx.log_child_error(&inner, "synchronized", actual, &line_wrapped);
                            }
                        }
                        // Check expression chains (method chains, etc.) inside the condition
                        self.check_expression(ctx, &inner, &line_wrapped);
                    }
                }

                break;
            }
        }

        // Check body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of labeled statement.
    fn check_labeled_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Labels can be at the statement indent OR at the enclosing statement's indent
        // (i.e., one level back from the child indent)
        // Per checkstyle: Labels are allowed at enclosing statement level or at child level
        let parent_indent = indent.with_offset(-self.basic_offset);
        let acceptable = indent.combine(&parent_indent);

        // Check label
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, &acceptable) {
                ctx.log_error(node, "label", actual, &acceptable);
            }
        }

        // Check the labeled statement - use combined indent since label body
        // should also accept both levels
        for child in node.children() {
            if child.kind() != "identifier" && child.kind() != ":" {
                self.check_statement(ctx, &child, &acceptable);
            }
        }
    }

    /// Check indentation of static initializer.
    fn check_static_init(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'static' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "static init", actual, indent);
            }
        }

        // Check the block
        for child in node.children() {
            if child.kind() == "block" {
                self.check_block(ctx, &child, indent);
            }
        }
    }

    /// Check indentation of instance initializer block at class level.
    /// For instance initializers, braces should be at member indent (NOT adjusted),
    /// and body should be at member indent + basicOffset.
    fn check_instance_init_block(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        member_indent: &IndentLevel,
    ) {
        // Check the opening brace - should be exactly at member indent
        if let Some(lcurly) = self.find_child(node, "{")
            && ctx.is_on_start_of_line(&lcurly)
        {
            let actual = ctx.column_from_node(&lcurly);
            if !ctx.is_indent_exact(actual, member_indent) {
                ctx.log_error(&lcurly, "block lcurly", actual, member_indent);
            }
        }

        // Check the closing brace - should be exactly at member indent
        if let Some(rcurly) = self.find_child(node, "}")
            && ctx.is_on_start_of_line(&rcurly)
        {
            let actual = ctx.column_from_node(&rcurly);
            if !ctx.is_indent_exact(actual, member_indent) {
                ctx.log_error(&rcurly, "block rcurly", actual, member_indent);
            }
        }

        // Children should be at member indent + basicOffset
        let child_indent = member_indent.with_offset(self.basic_offset);

        for child in node.children() {
            match child.kind() {
                "{" | "}" => {} // Skip braces
                _ => self.check_statement(ctx, &child, &child_indent),
            }
        }
    }

    /// Check indentation of enum constant.
    fn check_enum_constant(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "enum constant", actual, indent);
            }
        }
    }

    /// Check indentation of annotation element declaration.
    fn check_annotation_element(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "annotation field def", actual, indent);
            }
        }

        // Check default value if it's an array initializer
        for child in node.children() {
            if child.kind() == "element_value_array_initializer" {
                self.check_annotation_array_initializer(ctx, &child, indent);
            }
        }
    }

    /// Check annotations in modifiers block.
    /// - Each annotation marker should be at the expected indent
    /// - Annotation argument lists on continuation lines should be at line-wrapped indent
    /// - `check_annotation_indent`: when true (type declarations), check each annotation
    ///   at `indent` strictly. When false (member/method declarations), checkstyle is lenient.
    fn check_modifiers_annotations(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
        check_annotation_indent: bool,
    ) {
        if node.kind() != "modifiers" {
            return;
        }

        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);
        let mods_line = self.line_no(ctx, node);

        for child in node.children() {
            let kind = child.kind();
            if kind == "annotation" || kind == "marker_annotation" {
                // Check if the annotation itself is at correct indent
                let child_line = self.line_no(ctx, &child);
                if child_line > mods_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(child_line);
                    if check_annotation_indent {
                        // Type declarations: check annotation indent strictly
                        if !ctx.is_indent_exact(actual, indent) {
                            ctx.log_error(&child, "annotation def", actual, indent);
                        }
                    }
                }

                // Get the line of the @ symbol for continuation checks
                let at_line = child
                    .children()
                    .find(|c| c.kind() == "@")
                    .map(|at| self.line_no(ctx, &at))
                    .unwrap_or(child_line);

                // Check if annotation_argument_list exists and starts on a new line
                for ann_child in child.children() {
                    if ann_child.kind() == "annotation_argument_list" {
                        // Get the opening paren
                        if let Some(lparen) = self.find_child(&ann_child, "(") {
                            let lparen_line = self.line_no(ctx, &lparen);
                            if lparen_line > at_line && ctx.is_on_start_of_line(&lparen) {
                                let actual = ctx.column_from_node(&lparen);
                                if !ctx.is_indent_acceptable(actual, &line_wrapped_indent) {
                                    ctx.log_error(&lparen, "(", actual, &line_wrapped_indent);
                                }
                            }
                        }

                        // Check closing paren if on its own line - should be at annotation's
                        // starting indent (where the @ symbol is)
                        if let Some(rparen) = self.find_child(&ann_child, ")") {
                            let rparen_line = self.line_no(ctx, &rparen);
                            if rparen_line > at_line && ctx.is_on_start_of_line(&rparen) {
                                let actual = ctx.column_from_node(&rparen);
                                let ann_start_col = ctx.column_from_node(&child);
                                let rparen_expected = IndentLevel::new(ann_start_col);
                                if !ctx.is_indent_exact(actual, &rparen_expected) {
                                    ctx.log_error(&rparen, ")", actual, &rparen_expected);
                                }
                            }
                        }

                        // Check element_value_array_initializer children
                        for arg_child in ann_child.children() {
                            if arg_child.kind() == "element_value_array_initializer" {
                                self.check_annotation_array_initializer(ctx, &arg_child, indent);
                            } else if arg_child.kind() == "element_value_pair" {
                                // Check value in element_value_pair
                                // Use the element_value_pair's line indent as base, not class indent
                                // This handles: @Ann(names = { "A", "B" }) where elements are
                                // indented from the attribute's position
                                let pair_line = self.line_no(ctx, &arg_child);
                                let pair_indent = IndentLevel::new(ctx.get_line_start(pair_line));
                                for pair_child in arg_child.children() {
                                    if pair_child.kind() == "element_value_array_initializer" {
                                        self.check_annotation_array_initializer(
                                            ctx,
                                            &pair_child,
                                            &pair_indent,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Check indentation of yield statement.
    fn check_yield_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'yield' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_child_error(node, "block", actual, indent);
            }
        }

        // Check if the expression value is on a separate line - uses lenient mode for continuation
        let yield_line = self.line_no(ctx, node);
        for child in node.children() {
            match child.kind() {
                "yield" | ";" => {}
                _ => {
                    // This is the expression value
                    let child_line = self.line_no(ctx, &child);
                    if child_line > yield_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, indent) {
                            ctx.log_child_error(&child, "yield value", actual, indent);
                        }
                    }
                    // Recursively check expressions
                    self.check_expression(ctx, &child, indent);
                }
            }
        }
    }

    /// Check indentation of explicit constructor invocation (super(...) or this(...)).
    fn check_explicit_constructor_invocation(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        // Check the statement itself (super/this keyword or object.super)
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !ctx.is_indent_exact(actual, indent) {
                ctx.log_error(node, "ctor call", actual, indent);
            }
        }

        let ctor_line = self.line_no(ctx, node);
        let ctor_start = ctx.get_line_start(ctor_line);
        let ctor_indent = IndentLevel::new(ctor_start);

        // For chained constructor calls (obj.super(...)), check object expression
        // and check continuation lines (the `.` or `super` on separate lines)
        if let Some(obj) = node.child_by_field_name("object") {
            self.check_expression(ctx, &obj, indent);

            let obj_line = self.line_no(ctx, &obj);
            let obj_start = ctx.get_line_start(obj_line);
            let continuation_indent =
                IndentLevel::new(obj_start).with_offset(self.line_wrapping_indentation);

            // Check for `.` on a continuation line
            if let Some(dot) = self.find_child(node, ".") {
                let dot_line = self.line_no(ctx, &dot);
                if dot_line > obj_line && ctx.is_on_start_of_line(&dot) {
                    let actual = ctx.get_line_start(dot_line);
                    if !ctx.is_indent_acceptable(actual, &continuation_indent) {
                        ctx.log_child_error(&dot, "ctor call", actual, &continuation_indent);
                    }
                }
            }

            // Check for `super` keyword on a continuation line after object or dot
            if let Some(super_kw) = self.find_child(node, "super") {
                let super_line = self.line_no(ctx, &super_kw);
                if super_line > obj_line && ctx.is_on_start_of_line(&super_kw) {
                    let actual = ctx.get_line_start(super_line);
                    if !ctx.is_indent_acceptable(actual, &continuation_indent) {
                        ctx.log_child_error(&super_kw, "ctor call", actual, &continuation_indent);
                    }
                }
            }
        }

        // Check arguments
        if let Some(args) = node.child_by_field_name("arguments") {
            let has_object = node.child_by_field_name("object").is_some();

            // Find the keyword (super or this) node and line
            let super_kw = self.find_child(node, "super");
            let this_kw = self.find_child(node, "this");
            let keyword_node = super_kw.as_ref().or(this_kw.as_ref());
            let keyword_line = keyword_node
                .map(|kw| self.line_no(ctx, kw))
                .unwrap_or(ctor_line);

            // Determine argument expected indent based on call type:
            // - Simple super()/this(): args at statement indent
            // - Qualified obj.super(): args at ctor_start + lineWrap,
            //   or keyword_col + basicOffset/lineWrap when keyword is on continuation line
            let arg_expected = if !has_object {
                indent.clone()
            } else if let Some(kw) = keyword_node {
                let kw_line = self.line_no(ctx, kw);
                if kw_line > ctor_line {
                    let kw_col = ctx.get_line_start(kw_line);
                    IndentLevel::new(kw_col + self.basic_offset)
                        .add_acceptable(&[kw_col + self.line_wrapping_indentation])
                } else {
                    IndentLevel::new(ctor_start + self.line_wrapping_indentation)
                }
            } else {
                IndentLevel::new(ctor_start + self.line_wrapping_indentation)
            };

            // Also accept parent-based indentation for lparen check
            let combined_arg_indent =
                arg_expected.combine(&indent.with_offset(self.line_wrapping_indentation));

            // Check if argument list (lparen) is on a different line from the keyword
            if let Some(lparen) = self.find_child(&args, "(") {
                let lparen_line = self.line_no(ctx, &lparen);
                if lparen_line > keyword_line && ctx.is_on_start_of_line(&lparen) {
                    let actual = ctx.get_line_start(lparen_line);
                    if !ctx.is_indent_acceptable(actual, &combined_arg_indent) {
                        ctx.log_child_error(&lparen, "ctor call", actual, &arg_expected);
                    }
                }
            }

            let lparen_line = self.line_no(ctx, &args);

            for child in args.children() {
                match child.kind() {
                    // Skip punctuation and comments
                    "(" | ")" | "," | "line_comment" | "block_comment" => {}
                    _ => {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > lparen_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            // For simple super()/this(), use combined indent to
                            // accept args at indent + lineWrap in strict mode.
                            // For qualified obj.super(), use arg_expected which
                            // already accounts for keyword position.
                            let check_indent = if !has_object {
                                &combined_arg_indent
                            } else {
                                &arg_expected
                            };
                            if !ctx.is_indent_acceptable(actual, check_indent) {
                                ctx.log_child_error(&child, "ctor call", actual, &arg_expected);
                            }
                        }
                        // Check nested expressions in arguments
                        match child.kind() {
                            "lambda_expression" => {
                                self.check_lambda_expression(ctx, &child, &arg_expected);
                            }
                            "binary_expression" | "ternary_expression" => {
                                // Check continuation lines within the expression
                                let expr_start_line = self.line_no(ctx, &child);
                                for subchild in child.children() {
                                    let sub_line = self.line_no(ctx, &subchild);
                                    if sub_line > expr_start_line
                                        && ctx.is_on_start_of_line(&subchild)
                                    {
                                        let actual = ctx.get_line_start(sub_line);
                                        if !ctx.is_indent_acceptable(actual, &arg_expected) {
                                            ctx.log_child_error(
                                                &subchild,
                                                "ctor call",
                                                actual,
                                                &arg_expected,
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Check closing paren if on its own line
            if let Some(rparen) = self.find_child(&args, ")")
                && ctx.is_on_start_of_line(&rparen)
            {
                let actual = ctx.column_from_node(&rparen);
                // Closing paren can align with call start or parent indent
                let rparen_acceptable = ctor_indent.combine(indent);
                if !ctx.is_indent_acceptable(actual, &rparen_acceptable) {
                    ctx.log_error(&rparen, "rparen", actual, &ctor_indent);
                }
            }
        }
    }

    // Expression handlers

    /// Check indentation within an expression tree.
    /// This recursively traverses expressions to find lambdas, method calls, etc.
    fn check_expression(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        match node.kind() {
            "lambda_expression" => self.check_lambda_expression(ctx, node, indent),
            "method_invocation" => self.check_method_invocation(ctx, node, indent),
            "object_creation_expression" => {
                self.check_object_creation_expression(ctx, node, indent)
            }
            "array_creation_expression" => self.check_array_creation_expression(ctx, node, indent),
            "array_initializer" => self.check_array_initializer(ctx, node, indent),
            "element_value_array_initializer" => {
                self.check_annotation_array_initializer(ctx, node, indent);
            }
            "instanceof_expression" => self.check_instanceof_expression(ctx, node, indent),
            "switch_expression" => {
                // When switch_expression is the RHS of an assignment in an expression_statement,
                // indent is the raw statement indent. The switch should be at indent + lineWrap.
                let switch_indent = if node.parent().is_some_and(|p| {
                    p.kind() == "assignment_expression"
                        && p.parent()
                            .is_some_and(|gp| gp.kind() == "expression_statement")
                }) {
                    indent.with_offset(self.line_wrapping_indentation)
                } else {
                    indent.clone()
                };
                self.check_switch_statement(ctx, node, &switch_indent);
            }
            "binary_expression" | "ternary_expression" => {
                self.check_binary_expression(ctx, node, indent)
            }
            "array_access" => {
                // Check subscript content on continuation lines in variable declarations
                // e.g., String line = mIndentCheck[\n  getLineNo()];
                let in_var_decl = node.parent().is_some_and(|p| {
                    p.kind() == "variable_declarator"
                        && p.parent()
                            .is_some_and(|gp| gp.kind() == "local_variable_declaration")
                });
                if in_var_decl {
                    let access_line = self.line_no(ctx, node);
                    let line_wrapped = indent.with_offset(self.line_wrapping_indentation);
                    for child in node.children() {
                        if matches!(child.kind(), "[" | "]") {
                            continue;
                        }
                        let child_line = self.line_no(ctx, &child);
                        if child_line > access_line && ctx.is_on_start_of_line(&child) {
                            let actual = ctx.get_line_start(child_line);
                            if !ctx.is_indent_acceptable(actual, &line_wrapped) {
                                ctx.log_child_error(&child, "array access", actual, &line_wrapped);
                            }
                        }
                    }
                }
                for child in node.children() {
                    self.check_expression(ctx, &child, indent);
                }
            }
            _ => {
                // Recursively check children for nested expressions
                for child in node.children() {
                    self.check_expression(ctx, &child, indent);
                }
            }
        }
    }

    /// Check indentation of binary/ternary expression continuations.
    fn check_binary_expression(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check if this binary expression is inside a context where checkstyle is lenient.
        // Continuation lines still get a floor check (actual must not be below the floor).
        //
        // Context types:
        // 0 = none (full checking)
        // 1 = argument_list (floor = expression start column)
        // 2 = variable_declarator on continuation line (floor = indent)
        // 3 = lambda (skip entirely)
        // 4 = return/throw statement directly (floor = indent + lineWrap)
        // 5 = variable_declarator on declaration line (floor = indent + lineWrap)
        let lenient_context = {
            let mut current = node.parent();
            let mut context = 0u8;
            let mut depth = 0;
            const MAX_DEPTH: usize = 50;
            while let Some(p) = current {
                if depth >= MAX_DEPTH {
                    break;
                }
                depth += 1;
                match p.kind() {
                    "lambda_expression" => {
                        context = 3;
                        break;
                    }
                    "return_statement" | "throw_statement" => {
                        context = 4;
                        break;
                    }
                    "argument_list" => {
                        context = 1;
                        break;
                    }
                    "variable_declarator" => {
                        if !ctx.force_strict_condition()
                            && p.parent()
                                .is_some_and(|gp| gp.kind() == "local_variable_declaration")
                        {
                            // Distinguish declaration-line expressions from
                            // continuation-line expressions.
                            if let Some(gp) = p.parent() {
                                let decl_line = self.line_no(ctx, &gp);
                                let node_line = self.line_no(ctx, node);
                                context = if node_line == decl_line { 5 } else { 2 };
                            } else {
                                context = 2;
                            }
                            break;
                        }
                        current = p.parent();
                    }
                    // Stop at statement boundaries
                    "expression_statement"
                    | "if_statement"
                    | "while_statement"
                    | "for_statement"
                    | "do_statement"
                    | "switch_statement"
                    | "try_statement"
                    | "synchronized_statement"
                    | "block" => break,
                    _ => current = p.parent(),
                }
            }
            context
        };

        // Lambda expressions are fully skipped — handled by check_lambda_expression.
        if lenient_context == 3 {
            return;
        }

        // For lenient contexts, perform a floor check: flag continuation lines below
        // the floor and accept any indentation at or above it.
        if lenient_context > 0 {
            let expr_start_col = ctx.get_line_start(self.line_no(ctx, node));
            let line_wrap_floor = indent.first_level() + self.line_wrapping_indentation;
            let throw_stmt_wrap_floor = {
                let mut current = node.parent();
                let mut floor = None;
                while let Some(p) = current {
                    if p.kind() == "throw_statement" {
                        floor = Some(
                            ctx.get_line_start(self.line_no(ctx, &p))
                                + self.line_wrapping_indentation,
                        );
                        break;
                    }
                    if matches!(
                        p.kind(),
                        "expression_statement"
                            | "return_statement"
                            | "local_variable_declaration"
                            | "field_declaration"
                            | "block"
                    ) {
                        break;
                    }
                    current = p.parent();
                }
                floor
            };
            let (floor, expected) = match lenient_context {
                1 => {
                    if let Some(throw_floor) = throw_stmt_wrap_floor {
                        (throw_floor, IndentLevel::new(throw_floor))
                    } else {
                        (expr_start_col, IndentLevel::new(expr_start_col))
                    }
                }
                4 => {
                    if ctx.force_strict_condition() || node.kind() == "ternary_expression" {
                        (expr_start_col, IndentLevel::new(expr_start_col))
                    } else {
                        (line_wrap_floor, IndentLevel::new(line_wrap_floor))
                    }
                }
                5 => (line_wrap_floor, IndentLevel::new(line_wrap_floor)),
                _ => (indent.first_level(), indent.clone()),
            };
            let expr_line = self.line_no(ctx, node);
            // Walk left-recursive binary/ternary tree iteratively to check all
            // continuation lines without recursive check_expression calls.
            let mut current = Some(*node);
            while let Some(cur_node) = current {
                current = None;
                for child in cur_node.children() {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > expr_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if actual < floor {
                            ctx.log_child_error(&child, "expr", actual, &expected);
                        }
                    }
                    // Check text block closing quotes on continuation lines
                    if child.kind() == "string_literal" {
                        let mut closing_quotes: Option<CstNode> = None;
                        for string_child in child.children() {
                            if string_child.kind() == "\"\"\"" {
                                closing_quotes = Some(string_child);
                            }
                        }
                        if let Some(close_quotes) = closing_quotes {
                            let close_line = self.line_no(ctx, &close_quotes);
                            if close_line > expr_line && ctx.is_on_start_of_line(&close_quotes) {
                                let actual = ctx.column_from_node(&close_quotes);
                                if actual < floor {
                                    ctx.log_child_error(
                                        &close_quotes,
                                        "text block",
                                        actual,
                                        &expected,
                                    );
                                }
                            }
                        }
                    }
                    // Follow left child for left-recursive binary expressions
                    if current.is_none()
                        && matches!(child.kind(), "binary_expression" | "ternary_expression")
                    {
                        current = Some(child);
                    }
                    // Recurse into nested method/constructor calls so their internal
                    // argument indentation is checked even in lenient context.
                    if matches!(
                        child.kind(),
                        "method_invocation" | "object_creation_expression"
                    ) {
                        self.check_expression(ctx, &child, indent);
                    }
                }
            }
            return;
        }

        let expr_line = self.line_no(ctx, node);
        // Use the actual column where the binary expression node starts, not the line's indentation.
        // This is important for expressions inside parentheses (e.g., `if (a || b)`) where the
        // expression starts at a different column than the line's indentation.
        let expr_start = ctx.column_from_node(node);

        // For continuation lines, determine the expected indent:
        // - If expr_start < indent + lineWrap, we're likely in a deeply nested context
        //   (e.g., method call argument) where indent is already adjusted - use indent
        // - Otherwise, we're in a statement context (e.g., if/while condition) - add lineWrap
        // Exception: expressions directly inside if/while/for/do conditions always use
        // statement context, even when expr_start < line_wrapped_level (which happens
        // with large lineWrappingIndentation values like Android style with 8).
        let line_wrapped_level = indent.first_level() + self.line_wrapping_indentation;
        let is_condition_expr = {
            // Walk up through binary/ternary parents to find if this expression
            // is inside a condition (if/while/for/do). Nested binary expressions
            // have binary_expression as parent, not parenthesized_expression.
            let mut cur = node.parent();
            let mut found = false;
            while let Some(p) = cur {
                match p.kind() {
                    "binary_expression" | "ternary_expression" => cur = p.parent(),
                    "parenthesized_expression" => {
                        found = p.parent().is_some_and(|gp| {
                            matches!(
                                gp.kind(),
                                "if_statement"
                                    | "while_statement"
                                    | "for_statement"
                                    | "do_statement"
                            )
                        });
                        break;
                    }
                    _ => break,
                }
            }
            found
        };
        let is_nested = !is_condition_expr && expr_start < line_wrapped_level;
        let expected_indent = if is_nested {
            // Nested context - indent is already adjusted
            IndentLevel::new(indent.first_level())
        } else {
            // Statement context - add line wrapping
            IndentLevel::new(line_wrapped_level)
        };

        for child in node.children() {
            let child_line = self.line_no(ctx, &child);
            if child_line > expr_line && ctx.is_on_start_of_line(&child) {
                let actual = ctx.get_line_start(child_line);
                // In lenient mode: accept >= min(expected_indent)
                // In strict mode: accept only expected_indent or indent + lineWrap
                if ctx.force_strict_condition() {
                    // Strict: must be at expected_indent, indent + lineWrap, or
                    // aligned with the expression's line start. Checkstyle accepts
                    // visual alignment for expression continuations (e.g., string
                    // concatenation, ternary branches) even in strict mode.
                    let expr_line_start = ctx.get_line_start(expr_line);
                    // Inside array dimensions, checkstyle expects wrapped indentation,
                    // not visual alignment with expression start.
                    let in_array_dimension = {
                        let mut current = node.parent();
                        let mut found = false;
                        while let Some(p) = current {
                            if p.kind() == "dimensions_expr" {
                                found = true;
                                break;
                            }
                            if !matches!(
                                p.kind(),
                                "binary_expression"
                                    | "ternary_expression"
                                    | "parenthesized_expression"
                            ) {
                                break;
                            }
                            current = p.parent();
                        }
                        found
                    };
                    let mut acceptable = expected_indent
                        .combine(&indent.with_offset(self.line_wrapping_indentation));
                    if !in_array_dimension {
                        acceptable = acceptable.add_acceptable(&[expr_start, expr_line_start]);
                    }
                    if !ctx.is_indent_exact(actual, &acceptable) {
                        ctx.log_child_error(&child, "expr", actual, &expected_indent);
                    }
                } else {
                    // Lenient mode: check against expected_indent and a reasonable floor
                    // Flag only if under-indented relative to BOTH:
                    // - actual < expected_indent
                    // - actual < floor (minimum acceptable)
                    //
                    // Accept continuations that:
                    // 1. Are at or above expected_indent (properly indented)
                    // 2. Are exactly at expr_start (aligned with expression start)
                    // 3. Are at or above indent (for deeply nested cases)
                    let base_line_wrapped = indent.first_level() + self.line_wrapping_indentation;

                    // Special case: accept continuation at expr_start (aligned with expression)
                    if actual == expr_start {
                        // Continuation aligns with where expression started - acceptable
                        continue;
                    }

                    // Use the same threshold as expected_indent for consistency
                    let effective_floor = if is_nested {
                        // Nested context - use indent as floor
                        indent.first_level()
                    } else {
                        // Statement context - use base_line_wrapped as floor
                        base_line_wrapped
                    };

                    if actual < expected_indent.first_level() && actual < effective_floor {
                        ctx.log_child_error(&child, "expr", actual, &expected_indent);
                    }
                }
            }

            // Check text block closing quotes on continuation lines
            if child.kind() == "string_literal" {
                // Find the closing """ in text blocks
                let mut closing_quotes: Option<CstNode> = None;
                for string_child in child.children() {
                    if string_child.kind() == "\"\"\"" {
                        closing_quotes = Some(string_child);
                    }
                }
                if let Some(close_quotes) = closing_quotes {
                    let close_line = self.line_no(ctx, &close_quotes);
                    if close_line > expr_line && ctx.is_on_start_of_line(&close_quotes) {
                        let actual = ctx.column_from_node(&close_quotes);
                        if !ctx.is_indent_acceptable(actual, &expected_indent) {
                            ctx.log_child_error(
                                &close_quotes,
                                "text block",
                                actual,
                                &expected_indent,
                            );
                        }
                    }
                }
            }

            // Recursively check nested expressions
            self.check_expression(ctx, &child, indent);
        }
    }

    /// Check indentation of instanceof expression with pattern matching.
    fn check_instanceof_expression(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        let instanceof_line = self.line_no(ctx, node);
        let instanceof_start = ctx.get_line_start(instanceof_line);

        // Expected indent for all pattern content is instanceof_start + lineWrapping
        let pattern_expected_indent =
            IndentLevel::new(instanceof_start).with_offset(self.line_wrapping_indentation);

        // The pattern/type appears after 'instanceof' keyword
        // Check if it's on a new line and properly indented
        for child in node.children() {
            match child.kind() {
                // Skip the left operand and 'instanceof' keyword
                "instanceof" => {}
                // Type patterns, record patterns should be indented with line wrapping
                "type_pattern"
                | "record_pattern"
                | "type_identifier"
                | "generic_type"
                | "scoped_type_identifier" => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > instanceof_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !ctx.is_indent_acceptable(actual, &pattern_expected_indent) {
                            ctx.log_child_error(
                                &child,
                                "instanceof",
                                actual,
                                &pattern_expected_indent,
                            );
                        }
                    }
                    // Check nested record pattern components with the expected indent
                    self.check_pattern(ctx, &child, &pattern_expected_indent, instanceof_line);
                }
                _ => {
                    // Recursively check other children (like the left operand)
                    self.check_expression(ctx, &child, indent);
                }
            }
        }
    }

    /// Check indentation of pattern (record pattern, type pattern).
    /// `expected_indent` is the indent all pattern content should have (from instanceof base).
    /// `base_line` is the line where the pattern check started (for detecting multiline).
    fn check_pattern(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        expected_indent: &IndentLevel,
        base_line: usize,
    ) {
        match node.kind() {
            "record_pattern" => {
                // Record pattern has components that can span multiple lines
                // e.g., Point(int x, int y) or Rectangle(ColoredPoint(...), ...)

                // Check if the pattern header (type identifier) is at correct indent
                // This determines whether closing paren should be at pattern or statement indent
                let pattern_at_correct_indent =
                    self.is_pattern_at_correct_indent(ctx, node, expected_indent, base_line);

                for child in node.children() {
                    match child.kind() {
                        "(" | "," => {}
                        ")" => {
                            // Closing paren expected depends on whether pattern is at correct indent
                            let child_line = self.line_no(ctx, &child);
                            if child_line > base_line && ctx.is_on_start_of_line(&child) {
                                let actual = ctx.get_line_start(child_line);
                                let paren_expected = if pattern_at_correct_indent {
                                    // Pattern correct: closing paren at statement indent
                                    expected_indent.first_level() - self.line_wrapping_indentation
                                } else {
                                    // Pattern wrong: closing paren at pattern expected indent
                                    expected_indent.first_level()
                                };
                                let paren_indent = IndentLevel::new(paren_expected);
                                if !ctx.is_indent_acceptable(actual, &paren_indent) {
                                    ctx.log_error(&child, "rparen", actual, &paren_indent);
                                }
                            }
                        }
                        "record_pattern_body" => {
                            // Check each component in the body and the closing paren
                            for component in child.children() {
                                match component.kind() {
                                    "(" | "," => {}
                                    ")" => {
                                        // Closing paren expected depends on whether pattern is at correct indent
                                        let comp_line = self.line_no(ctx, &component);
                                        if comp_line > base_line
                                            && ctx.is_on_start_of_line(&component)
                                        {
                                            let actual = ctx.get_line_start(comp_line);
                                            let paren_expected = if pattern_at_correct_indent {
                                                // Pattern correct: closing paren at statement indent
                                                expected_indent.first_level()
                                                    - self.line_wrapping_indentation
                                            } else {
                                                // Pattern wrong: closing paren at pattern expected indent
                                                expected_indent.first_level()
                                            };
                                            let paren_indent = IndentLevel::new(paren_expected);
                                            if !ctx.is_indent_acceptable(actual, &paren_indent) {
                                                ctx.log_error(
                                                    &component,
                                                    "rparen",
                                                    actual,
                                                    &paren_indent,
                                                );
                                            }
                                        }
                                    }
                                    _ => {
                                        let comp_line = self.line_no(ctx, &component);
                                        if comp_line > base_line
                                            && ctx.is_on_start_of_line(&component)
                                        {
                                            let actual = ctx.get_line_start(comp_line);
                                            if !ctx.is_indent_acceptable(actual, expected_indent) {
                                                ctx.log_child_error(
                                                    &component,
                                                    "record pattern",
                                                    actual,
                                                    expected_indent,
                                                );
                                            }
                                        }
                                        // Recursively check nested patterns
                                        self.check_pattern(
                                            ctx,
                                            &component,
                                            expected_indent,
                                            base_line,
                                        );
                                    }
                                }
                            }
                        }
                        _ => {
                            // Nested type identifier, record pattern, etc.
                            let child_line = self.line_no(ctx, &child);
                            if child_line > base_line && ctx.is_on_start_of_line(&child) {
                                let actual = ctx.get_line_start(child_line);
                                if !ctx.is_indent_acceptable(actual, expected_indent) {
                                    ctx.log_child_error(
                                        &child,
                                        "record pattern",
                                        actual,
                                        expected_indent,
                                    );
                                }
                            }
                            self.check_pattern(ctx, &child, expected_indent, base_line);
                        }
                    }
                }
            }
            "type_pattern" | "record_pattern_component" => {
                // Check nested patterns in components
                for child in node.children() {
                    self.check_pattern(ctx, &child, expected_indent, base_line);
                }
            }
            _ => {
                // Other patterns - recursively check
                for child in node.children() {
                    if matches!(
                        child.kind(),
                        "record_pattern" | "type_pattern" | "record_pattern_component"
                    ) {
                        self.check_pattern(ctx, &child, expected_indent, base_line);
                    }
                }
            }
        }
    }

    /// Check if a record pattern's header (type identifier) is at correct indent.
    /// Used to determine whether closing paren should be at pattern or statement indent.
    fn is_pattern_at_correct_indent(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        expected_indent: &IndentLevel,
        base_line: usize,
    ) -> bool {
        // Find the type identifier (first child that's a type name)
        for child in node.children() {
            match child.kind() {
                "identifier" | "type_identifier" | "scoped_type_identifier" | "generic_type" => {
                    let child_line = self.line_no(ctx, &child);
                    // If on base line, consider it correct (not a multiline pattern)
                    if child_line == base_line {
                        return true;
                    }
                    // Check if on a new line and at correct indent
                    if ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        return ctx.is_indent_acceptable(actual, expected_indent);
                    }
                    return true; // Not at start of line, consider correct
                }
                _ => {}
            }
        }
        // No identifier found, assume correct
        true
    }

    /// Check indentation of lambda expression.
    fn check_lambda_expression(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Get the lambda's starting position for line wrapping calculations
        let lambda_line = self.line_no(ctx, node);
        let lambda_start = ctx.get_line_start(lambda_line);

        // Lambda indent based on actual position (for line wrapping)
        let lambda_indent = IndentLevel::new(lambda_start);

        // Check if lambda itself is at wrong position
        let lambda_at_wrong_pos = !ctx.is_indent_acceptable(lambda_start, indent);
        let lambda_starts_line = ctx.is_on_start_of_line(node);

        // The arrow should be at the same level as the lambda parameters
        // With forceStrictCondition=true, only accept expected position
        let arrow_expected_indent = if self.force_strict_condition {
            indent.clone()
        } else {
            lambda_indent.combine(indent)
        };

        // Check the arrow (->) if it's on a continuation line
        // In argument lists (method/constructor args), checkstyle is lenient (>= min).
        // In variable assignments and other contexts, use exact positional matching.
        let arrow_expected_with_wrap = if self.force_strict_condition {
            arrow_expected_indent.clone()
        } else {
            arrow_expected_indent.add_acceptable(&[
                indent.first_level() + self.line_wrapping_indentation,
                lambda_start + self.line_wrapping_indentation,
            ])
        };
        let arrow_in_arg_list = node.parent().is_some_and(|p| p.kind() == "argument_list");
        for child in node.children() {
            if child.kind() == "->" {
                let arrow_line = self.line_no(ctx, &child);
                if arrow_line > lambda_line && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.get_line_start(arrow_line);
                    if arrow_in_arg_list {
                        if !ctx.is_indent_acceptable(actual, &arrow_expected_indent) {
                            ctx.log_error(&child, "->", actual, indent);
                        }
                    } else if !ctx.is_indent_exact(actual, &arrow_expected_with_wrap) {
                        ctx.log_error(&child, "->", actual, indent);
                    }
                }
            }
        }

        // Lambda body indentation
        if let Some(body) = node.child_by_field_name("body") {
            let body_line = self.line_no(ctx, &body);

            if body.kind() == "block" {
                // Block body - determine the base indent for the block
                // Key insight: when the lambda block brace appears on a NEW LINE and the lambda
                // is at the statement level (not on a continuation line), checkstyle accepts
                // the brace at the statement level. This is the common pattern:
                //   executor.submit(() ->
                //   {              <- at same column as executor.submit, not +4
                //       doWork();
                //   });
                //
                // But for lambdas on continuation lines like:
                //   Function<String, String> f =
                //           (string) -> {   <- lambda at column 16 (continuation)
                //               work();     <- content at 20 is a VIOLATION (expected 16)
                //           };
                // We should NOT accept the lambda's position as the base.
                //
                // EXCEPTION: For lambdas inside method chain arguments (like .mapToObj(i -> { ... })),
                // checkstyle accepts the lambda's actual position even with forceStrictCondition=true.
                //
                // Heuristic: lambda is at "statement level" if lambda_start <= indent.first_level()
                let lambda_at_statement_level = lambda_start <= indent.first_level();

                // Check if lambda is inside a method chain argument - be lenient about block indent
                let in_method_chain_arg = node.parent().is_some_and(|p| {
                    p.kind() == "argument_list"
                        && p.parent()
                            .is_some_and(|gp| gp.kind() == "method_invocation")
                });

                let block_indent = if in_method_chain_arg {
                    // Lambda in method chain argument - accept actual position
                    let mut combined = lambda_indent
                        .combine(indent)
                        .add_acceptable(&[lambda_start + self.line_wrapping_indentation]);

                    // Handle doubly-nested calls like r2r(r2r(() -> { ... })) by counting
                    // how many method_invocation(argument_list()) levels surround the lambda.
                    // Each additional nesting level adds another acceptable indent level.
                    let mut nesting = 0i32;
                    let mut current = node.parent();
                    while let Some(p) = current {
                        if p.kind() == "argument_list"
                            && let Some(gp) = p.parent()
                            && gp.kind() == "method_invocation"
                        {
                            nesting += 1;
                            current = gp.parent();
                            continue;
                        }
                        break;
                    }
                    if nesting > 1 {
                        for i in 2..=nesting {
                            let extra = lambda_start + i * self.line_wrapping_indentation;
                            combined = combined.add_acceptable(&[extra]);
                        }
                    }

                    combined
                } else if body_line > lambda_line && lambda_at_statement_level {
                    // Block starts on a new line AND lambda is at statement level
                    // Accept BOTH lambda position (statement level) and expected indent
                    lambda_indent.combine(indent)
                } else if self.force_strict_condition && lambda_at_wrong_pos {
                    // Lambda at wrong position (or on continuation) - use expected
                    indent.clone()
                } else {
                    // Block brace `{` is on same line as lambda (`-> {`)
                    // Check if the opening brace is at start of line
                    if let Some(lcurly) = self.find_child(&body, "{") {
                        if ctx.is_on_start_of_line(&lcurly) {
                            // Brace at start of line - check if at expected position
                            let brace_col = ctx.column_from_node(&lcurly);
                            if self.force_strict_condition {
                                indent.clone()
                            } else if brace_col >= indent.first_level() {
                                // Brace at or above expected position - use it as block base
                                IndentLevel::new(brace_col)
                            } else {
                                // Brace below expected position - use expected so it gets flagged
                                indent.clone()
                            }
                        } else {
                            // Brace at end of line (e.g., `return () -> {`)
                            let lambda_at_start = ctx.is_on_start_of_line(node);
                            let line_over_indented = lambda_start > indent.first_level();

                            if !lambda_at_start && line_over_indented {
                                // Lambda NOT at start of line, but line is over-indented
                                // This indicates the containing statement is misaligned
                                // Use expected position as block base
                                indent.with_offset(self.basic_offset)
                            } else if lambda_at_start {
                                // Lambda at start of line
                                // Check if lambda is at expected continuation positions
                                let expected_one_step = indent.first_level() + self.basic_offset;
                                let expected_line_wrap =
                                    indent.first_level() + self.line_wrapping_indentation;
                                if lambda_start == expected_one_step
                                    || lambda_start == expected_line_wrap
                                {
                                    // Lambda at expected continuation - combine for brace flexibility
                                    indent.combine(&lambda_indent)
                                } else {
                                    lambda_indent.clone()
                                }
                            } else {
                                // Lambda in middle of line at correct indent - use combined
                                indent
                                    .with_offset(self.line_wrapping_indentation)
                                    .combine(&lambda_indent)
                                    .combine(indent)
                            }
                        }
                    } else {
                        indent.clone()
                    }
                };
                self.check_block(ctx, &body, &block_indent);
            } else if ctx.is_on_start_of_line(&body) {
                // Expression body on a new line - should be indented with line wrapping
                // Checkstyle is lenient about lambda expression body indentation even with
                // forceStrictCondition=true. Accept any indent >= base statement level.
                //
                // For nested lambdas inside method arguments, the indent can accumulate
                // to very high levels. Use the lambda's actual position as the base check.
                let actual = ctx.get_line_start(body_line);
                // Use the lambda's actual position as the floor (not accumulated indent)
                // This handles nested lambdas where indent has accumulated
                let min_indent = lambda_start.min(indent.first_level());
                if actual < min_indent {
                    let body_indent = indent.with_offset(self.line_wrapping_indentation);
                    ctx.log_child_error(&body, "lambda", actual, &body_indent);
                }
                // Check nested expressions in the body
                let expr_indent =
                    if self.force_strict_condition || (lambda_at_wrong_pos && lambda_starts_line) {
                        indent.clone()
                    } else {
                        lambda_indent.clone()
                    };
                self.check_expression(ctx, &body, &expr_indent);
            } else {
                // Same line - check nested expressions
                let expr_indent = if lambda_at_wrong_pos && lambda_starts_line {
                    indent.clone()
                } else {
                    lambda_indent.clone()
                };
                self.check_expression(ctx, &body, &expr_indent);
            }
        }
    }

    /// Check indentation of method invocation.
    fn check_method_invocation(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // For chained calls, we need the position of this specific method in the chain,
        // not the start of the entire chain. Use the '.' token position if available.
        let (_method_line, method_start) = if node.child_by_field_name("object").is_some() {
            // Chained call - find the '.' to get the actual position of this method
            if let Some(dot) = node.children().find(|c| c.kind() == ".") {
                let dot_line = self.line_no(ctx, &dot);
                (dot_line, ctx.get_line_start(dot_line))
            } else {
                let line = self.line_no(ctx, node);
                (line, ctx.get_line_start(line))
            }
        } else {
            // Standalone call - use the node's position
            let line = self.line_no(ctx, node);
            (line, ctx.get_line_start(line))
        };

        // For chained method calls, calculate the base indent from the actual line start
        // This allows proper line wrapping for chains like:
        //   new String()
        //       .substring(0, 100)  <- indented by lineWrappingIndentation from 'new String()'
        let chain_indent = IndentLevel::new(method_start);

        // Track the expected chain indent for argument checking. When a chained method
        // is misaligned, checkstyle computes child indent from the expected position.
        let mut chain_expected_start: Option<i32> = None;

        // Check if this is a chained call (has an object/receiver)
        if let Some(obj) = node.child_by_field_name("object") {
            let obj_line = self.line_no(ctx, &obj);

            // Find the "." operator
            let dot_node = node.children().find(|c| c.kind() == ".");

            // Check if "." is on a continuation line
            // When the chain is at "statement level" (not nested inside an argument list
            // or other expression), checkstyle requires chain continuations at
            // >= indent + lineWrappingIndentation. For nested chains, plain indent is the floor.
            let method_has_multiline_args = node
                .child_by_field_name("arguments")
                .and_then(|args| {
                    self.find_child(&args, ")")
                        .map(|rparen| self.line_no(ctx, &rparen) > self.line_no(ctx, &args))
                })
                .unwrap_or(false);

            let (is_top_level_chain, top_level_expression_stmt) = {
                let mut current = node.parent();
                let mut top_level = false;
                let mut expression_stmt = false;
                while let Some(p) = current {
                    match p.kind() {
                        "method_invocation" | "parenthesized_expression" | "field_access" => {
                            current = p.parent();
                        }
                        "expression_statement" | "return_statement" | "throw_statement" => {
                            top_level = true;
                            expression_stmt = p.kind() == "expression_statement";
                            break;
                        }
                        "variable_declarator" => {
                            // For variable initializers: only apply chain lineWrap when
                            // the chain root starts on the same line as the declaration
                            // (value NOT on a continuation line). When the value is
                            // line-wrapped, indent already includes lineWrap.
                            if let Some(gp) = p.parent()
                                && matches!(
                                    gp.kind(),
                                    "local_variable_declaration" | "field_declaration"
                                )
                            {
                                let decl_line = self.line_no(ctx, &gp);
                                let mut root = obj;
                                while root.kind() == "method_invocation" {
                                    if let Some(inner) = root.child_by_field_name("object") {
                                        root = inner;
                                    } else {
                                        break;
                                    }
                                }
                                let root_line = self.line_no(ctx, &root);
                                top_level = root_line == decl_line;
                            }
                            break;
                        }
                        _ => break,
                    }
                }
                (top_level, expression_stmt)
            };
            let top_level_requires_wrap =
                is_top_level_chain && (!top_level_expression_stmt || method_has_multiline_args);

            if let Some(ref dot) = dot_node {
                let dot_line = self.line_no(ctx, dot);
                if dot_line > obj_line && ctx.is_on_start_of_line(dot) {
                    let actual = ctx.get_line_start(dot_line);
                    let expected_chain_indent = if !is_top_level_chain
                        && ctx.force_strict_condition()
                    {
                        let mut nested_expected =
                            indent.combine(&indent.with_offset(self.line_wrapping_indentation));
                        if self.line_wrapping_indentation > 0 {
                            let additions: Vec<i32> = (2..=6)
                                .map(|step| {
                                    indent.first_level() + (step * self.line_wrapping_indentation)
                                })
                                .collect();
                            nested_expected = nested_expected.add_acceptable(&additions);
                        }
                        nested_expected
                    } else if top_level_requires_wrap && !ctx.force_strict_condition() {
                        indent.with_offset(self.line_wrapping_indentation)
                    } else {
                        indent.clone()
                    };
                    // Also accept column 0 (see checkstyle issue #7675).
                    if actual != 0 {
                        if is_top_level_chain && ctx.force_strict_condition() {
                            if actual < expected_chain_indent.first_level() {
                                ctx.log_error(dot, "method call", actual, &expected_chain_indent);
                            }
                        } else if !ctx.is_indent_acceptable(actual, &expected_chain_indent) {
                            ctx.log_error(dot, "method call", actual, &expected_chain_indent);
                        }
                    }

                    // Track expected chain position for argument indent computation.
                    // Only applies when dot is on a continuation line.
                    if top_level_requires_wrap {
                        chain_expected_start = Some(expected_chain_indent.first_level());
                    }
                }
            }

            // Check if method NAME is on a continuation line (e.g., `Files.\nnewBufferedWriter(...)`)
            // This catches the pattern where "." is at end of line and method name is on next line
            if let Some(name) = node.child_by_field_name("name") {
                let name_line = self.line_no(ctx, &name);
                // Check if name is on a different line than both object and dot
                let dot_line = dot_node
                    .as_ref()
                    .map(|d| self.line_no(ctx, d))
                    .unwrap_or(obj_line);
                if name_line > dot_line && ctx.is_on_start_of_line(&name) {
                    let actual = ctx.get_line_start(name_line);
                    let expected_chain_indent = if !is_top_level_chain
                        && ctx.force_strict_condition()
                    {
                        let mut nested_expected =
                            indent.combine(&indent.with_offset(self.line_wrapping_indentation));
                        if self.line_wrapping_indentation > 0 {
                            let additions: Vec<i32> = (2..=6)
                                .map(|step| {
                                    indent.first_level() + (step * self.line_wrapping_indentation)
                                })
                                .collect();
                            nested_expected = nested_expected.add_acceptable(&additions);
                        }
                        nested_expected
                    } else if top_level_requires_wrap && !ctx.force_strict_condition() {
                        indent.with_offset(self.line_wrapping_indentation)
                    } else {
                        indent.clone()
                    };
                    // For patterns like `Files.\n    newBufferedWriter(...)`, checkstyle
                    // accepts name continuation at the previous line start + lineWrap.
                    let wrapped_from_dot = IndentLevel::new(
                        ctx.get_line_start(dot_line) + self.line_wrapping_indentation,
                    );
                    let expected_name_indent = expected_chain_indent.combine(&wrapped_from_dot);
                    if is_top_level_chain && ctx.force_strict_condition() {
                        if actual < expected_chain_indent.first_level() {
                            ctx.log_child_error(
                                &name,
                                "method call",
                                actual,
                                &expected_name_indent,
                            );
                        }
                    } else if !ctx.is_indent_acceptable(actual, &expected_name_indent) {
                        ctx.log_child_error(&name, "method call", actual, &expected_name_indent);
                    }
                    if top_level_requires_wrap {
                        chain_expected_start = Some(expected_chain_indent.first_level());
                    }
                }
            }

            // Recursively check the object expression
            self.check_expression(ctx, &obj, indent);
        }

        // Check arguments
        if let Some(args) = node.child_by_field_name("arguments") {
            // For argument indentation, use the method name position (identifier) not the dot.
            // This handles cases like:
            //   this.
            //       methodName(    <- method name at indent 8
            //           arg        <- argument at indent 12 = 8 + 4
            let method_name_start = node
                .child_by_field_name("name")
                .map(|name| {
                    let name_line = self.line_no(ctx, &name);
                    ctx.get_line_start(name_line)
                })
                .unwrap_or(method_start);
            // Checkstyle computes child indent from the method's expected position,
            // not its actual position. For chain calls, use the expected chain indent
            // when the method is misaligned (actual < expected).
            let effective_method_start = if let Some(exp) = chain_expected_start {
                std::cmp::max(method_name_start, exp)
            } else {
                method_name_start
            };
            let arg_base_indent = IndentLevel::new(effective_method_start);
            // Checkstyle accepts method call arguments at either basicOffset or
            // lineWrappingIndentation from the method's expected position.
            let arg_indent = arg_base_indent
                .with_offset(self.basic_offset)
                .combine(&arg_base_indent.with_offset(self.line_wrapping_indentation));

            let lparen_line = self.line_no(ctx, &args);
            let mut _in_multiline_args = false;

            // Check if this is a multiline argument list (closing ) on a different line)
            // For multiline argument lists, nested expressions need accumulated indentation
            let is_multiline_arglist = self
                .find_child(&args, ")")
                .is_some_and(|rparen| self.line_no(ctx, &rparen) > lparen_line);

            // For multiline argument lists, accumulate line wrapping for nested expressions
            let nested_indent = if is_multiline_arglist {
                indent.with_offset(self.line_wrapping_indentation)
            } else {
                chain_indent.clone()
            };

            // Check if this method call is inside a return statement or field declaration.
            // Checkstyle doesn't check argument indentation for these contexts - it accepts
            // any indentation for method call args in return statements and >= member indent
            // for field declarations.
            let in_return_context = node
                .parent()
                .is_some_and(|p| p.kind() == "return_statement");
            let in_field_context = node.parent().is_some_and(|p| {
                matches!(p.kind(), "variable_declarator")
                    && p.parent()
                        .is_some_and(|gp| gp.kind() == "field_declaration")
            });
            let skip_arg_indent_check = in_return_context || in_field_context;
            let in_var_initializer_context = node
                .parent()
                .is_some_and(|p| p.kind() == "variable_declarator");

            // Detect if the first argument is on a new line (not on the paren line).
            // When all args start on new lines, checkstyle accepts args aligned with
            // the method name position itself, not requiring +lineWrap.
            let first_arg_on_new_line = args
                .children()
                .find(|c| !matches!(c.kind(), "(" | ")" | "," | "line_comment" | "block_comment"))
                .is_some_and(|first_arg| self.line_no(ctx, &first_arg) > lparen_line);

            for child in args.children() {
                match child.kind() {
                    // Skip punctuation and comments
                    "(" | ")" | "," | "line_comment" | "block_comment" => {}
                    _ => {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > lparen_line {
                            _in_multiline_args = true;
                            // Arguments on new lines should be indented
                            // Skip this check for return statements and field declarations
                            // where checkstyle is more lenient.
                            let skip_child_indent =
                                skip_arg_indent_check && child.kind() != "lambda_expression";
                            if !skip_child_indent && ctx.is_on_start_of_line(&child) {
                                let actual = ctx.get_line_start(child_line);
                                if ctx.force_strict_condition() {
                                    let below_floor = actual < effective_method_start;
                                    let above_strict_max = in_var_initializer_context
                                        && actual > arg_indent.last_level();
                                    if below_floor || above_strict_max {
                                        ctx.log_child_error(
                                            &child,
                                            "method call",
                                            actual,
                                            &arg_indent,
                                        );
                                    }
                                } else if first_arg_on_new_line {
                                    let allow_method_alignment = child.kind()
                                        != "lambda_expression"
                                        && actual == method_name_start;
                                    if !allow_method_alignment
                                        && !ctx.is_indent_acceptable(actual, &arg_indent)
                                    {
                                        ctx.log_child_error(
                                            &child,
                                            "method call",
                                            actual,
                                            &arg_indent,
                                        );
                                    }
                                } else if actual < effective_method_start {
                                    ctx.log_child_error(&child, "method call", actual, &arg_indent);
                                }
                            }
                        }
                        // Check nested expressions in arguments.
                        // For object creation expressions, method invocations, and binary expressions,
                        // pass the base indent level to avoid accumulating lineWrappingIndentation.
                        // Checkstyle treats continuations within an expression context as being
                        // relative to the context's start, not accumulated per nesting level.
                        let expr_indent = match child.kind() {
                            "object_creation_expression" | "method_invocation" => &arg_base_indent,
                            "binary_expression" | "ternary_expression" => indent,
                            "lambda_expression" => &arg_base_indent,
                            _ => &nested_indent,
                        };
                        self.check_expression(ctx, &child, expr_indent);
                    }
                }
            }

            // Check closing paren if on its own line
            if is_multiline_arglist
                && let Some(rparen) = self.find_child(&args, ")")
                && ctx.is_on_start_of_line(&rparen)
            {
                let actual = ctx.column_from_node(&rparen);
                // Closing paren should align with method call expected position or indent level,
                // also accept lineWrap offset for nested method calls.
                // Use effective_method_start (which uses chain expected position when available)
                // so misaligned chain methods don't lower the rparen threshold.
                let base = IndentLevel::new(effective_method_start).combine(indent);
                let rparen_expected =
                    base.combine(&indent.with_offset(self.line_wrapping_indentation));
                if !ctx.is_indent_exact(actual, &rparen_expected) {
                    ctx.log_error(&rparen, "rparen", actual, indent);
                }
            }
        }
    }

    /// Check indentation of object creation expression (new ...).
    fn check_object_creation_expression(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        // Get the new expression's starting position for line wrapping
        let new_line = self.line_no(ctx, node);
        let new_start = ctx.get_line_start(new_line);
        let new_indent = IndentLevel::new(new_start);
        let expected_new_pos = indent.combine(&indent.with_offset(self.line_wrapping_indentation));
        let new_at_wrong_pos = !ctx.is_indent_acceptable(new_start, &expected_new_pos);

        // Check type name and lparen on continuation lines.
        // These should be indented relative to the `new` keyword position, not the passed indent.
        // When `new` is at the end of a line and the type starts on the next line,
        // checkstyle expects the type to be at new_start + lineWrap.
        let continuation_indent = new_indent.with_offset(self.line_wrapping_indentation);

        // Check type name on continuation line (e.g., new\nObject())
        if let Some(type_node) = node.child_by_field_name("type") {
            let type_line = self.line_no(ctx, &type_node);
            if type_line > new_line && ctx.is_on_start_of_line(&type_node) {
                let actual = ctx.get_line_start(type_line);
                // In lenient mode, accept >= new_start; in strict mode, need exact new_start + lineWrap
                if !ctx.is_indent_acceptable(actual, &new_indent) {
                    ctx.log_child_error(&type_node, "new", actual, &continuation_indent);
                }
            }
        }

        // Check argument list opening paren on continuation line (e.g., new Object\n())
        if let Some(args) = node.child_by_field_name("arguments")
            && let Some(lparen) = self.find_child(&args, "(")
        {
            let lparen_line = self.line_no(ctx, &lparen);
            if lparen_line > new_line && ctx.is_on_start_of_line(&lparen) {
                let actual = ctx.get_line_start(lparen_line);
                // In lenient mode, accept >= new_start; in strict mode, need exact
                if !ctx.is_indent_acceptable(actual, &new_indent) {
                    ctx.log_child_error(&lparen, "new", actual, &continuation_indent);
                }
            }
        }

        // Determine if the first argument is on a continuation line
        // This affects how we calculate the anonymous class body indent
        let first_arg_continuation_indent = node
            .child_by_field_name("arguments")
            .or_else(|| self.find_child(node, "argument_list"))
            .and_then(|arg_list| {
                // Find the first actual argument (skip punctuation and comments)
                for arg in arg_list.children() {
                    match arg.kind() {
                        "(" | ")" | "," | "line_comment" | "block_comment" => continue,
                        _ => {
                            let arg_line = self.line_no(ctx, &arg);
                            if arg_line > new_line {
                                // First argument is on a different line than `new`
                                // Use that line's indent as the base for anonymous class body
                                return Some(ctx.get_line_start(arg_line));
                            }
                            // First argument is on same line as `new`
                            return None;
                        }
                    }
                }
                None
            });

        // Check anonymous class body if present
        // Expected brace positions:
        // - Based on expected new position (passed indent): indent, indent+basic, indent+lineWrap
        // - Based on actual new position when new is at a "clean" offset from indent
        //   (divisible by basic_offset or line_wrapping)
        // - Based on containing lambda expression position (for anonymous classes in lambdas)
        let mut expected_brace = indent.add_acceptable(&[
            indent.first_level() + self.basic_offset,
            indent.first_level() + self.line_wrapping_indentation,
        ]);
        // Add actual-based positions when new is at a clean offset from indent base
        // This handles nested continuation patterns (e.g., method arg at double line-wrap)
        let offset_from_base = new_start - indent.first_level();
        if offset_from_base > 0 {
            // Check if new is at a clean multiple of the indent offsets
            let is_clean_offset = (self.basic_offset > 0
                && offset_from_base % self.basic_offset == 0)
                || (self.line_wrapping_indentation > 0
                    && offset_from_base % self.line_wrapping_indentation == 0);
            if is_clean_offset {
                expected_brace = expected_brace.add_acceptable(&[
                    new_start,
                    new_start + self.basic_offset,
                    new_start + self.line_wrapping_indentation,
                ]);
            }
        }

        // If we're inside a lambda expression, also accept alignment with the lambda's position.
        // This handles patterns like:
        //   supplier((i) -> new Service[]{ new Service()
        //   {  // <- aligned with lambda start
        //       ...
        //   }.index(i) }
        // );
        let lambda_start = {
            let mut current = node.parent();
            let mut lambda_pos = None;
            while let Some(p) = current {
                if p.kind() == "lambda_expression" {
                    lambda_pos = Some(ctx.get_line_start(self.line_no(ctx, &p)));
                    break;
                }
                // Stop at statement boundaries
                if matches!(
                    p.kind(),
                    "expression_statement"
                        | "local_variable_declaration"
                        | "return_statement"
                        | "block"
                        | "method_declaration"
                ) {
                    break;
                }
                current = p.parent();
            }
            lambda_pos
        };
        if let Some(lambda_indent) = lambda_start {
            expected_brace =
                expected_brace.add_acceptable(&[lambda_indent, lambda_indent + self.basic_offset]);
        }

        for child in node.children() {
            match child.kind() {
                "class_body" => {
                    // Anonymous class body - determine the base indent
                    // When forceStrictCondition=true, use expected indent only
                    // When false, accept both actual and expected indents
                    let body_indent = if let Some(lcurly) = self.find_child(&child, "{") {
                        // Always check closing brace against expected positions
                        // For anonymous class braces, use strict checking (exact match)
                        if let Some(rcurly) = self.find_child(&child, "}")
                            && ctx.is_on_start_of_line(&rcurly)
                        {
                            let actual_rcurly = ctx.column_from_node(&rcurly);
                            if !ctx.is_indent_exact(actual_rcurly, &expected_brace) {
                                ctx.log_child_error(
                                    &rcurly,
                                    "block rcurly",
                                    actual_rcurly,
                                    &expected_brace,
                                );
                            }
                        }

                        if ctx.is_on_start_of_line(&lcurly) {
                            // Opening brace starts the line - check if it's at correct position
                            let actual_brace = ctx.column_from_node(&lcurly);
                            if !ctx.is_indent_exact(actual_brace, &expected_brace) {
                                ctx.log_child_error(
                                    &lcurly,
                                    "block lcurly",
                                    actual_brace,
                                    &expected_brace,
                                );
                            }
                            // Use brace position for body indent, combined with expected
                            let brace_indent = IndentLevel::new(actual_brace);
                            brace_indent.combine(&new_indent).combine(indent)
                        } else if let Some(cont_indent) = first_arg_continuation_indent {
                            // First argument was on a continuation line
                            // Use that line's indent as the base for the anonymous class body
                            IndentLevel::new(cont_indent)
                        } else if ctx.force_strict_condition() {
                            // In strict mode, use expected indent when `new` is misaligned.
                            if new_at_wrong_pos {
                                indent.clone()
                            } else {
                                new_indent.clone()
                            }
                        } else {
                            // Opening brace at end of line (e.g., new Runnable() {)
                            // In lenient mode, avoid propagating under-indented `new`
                            // positions into anonymous class members.
                            if new_at_wrong_pos {
                                indent.combine(&indent.with_offset(self.basic_offset))
                            } else {
                                new_indent
                                    .combine(indent)
                                    .combine(&new_indent.with_offset(self.basic_offset))
                                    .combine(&indent.with_offset(self.basic_offset))
                            }
                        }
                    } else {
                        new_indent.clone()
                    };
                    self.check_class_body(ctx, &child, &body_indent);
                }
                "argument_list" => {
                    // Use the new expression's line start as base for argument indentation
                    // Arguments on continuation lines should be at new_indent + lineWrap
                    let arg_indent = new_indent.with_offset(self.line_wrapping_indentation);

                    // Check if this object creation is in a context where checkstyle is lenient
                    // about argument indentation (return statement, field declaration)
                    let in_return_context = node
                        .parent()
                        .is_some_and(|p| p.kind() == "return_statement");
                    let in_field_context = node.parent().is_some_and(|p| {
                        matches!(p.kind(), "variable_declarator")
                            && p.parent()
                                .is_some_and(|gp| gp.kind() == "field_declaration")
                    });
                    let skip_arg_indent_check = in_return_context || in_field_context;
                    let in_try_resource = {
                        let mut current = node.parent();
                        let mut found = false;
                        while let Some(p) = current {
                            if p.kind() == "resource" {
                                found = true;
                                break;
                            }
                            if matches!(
                                p.kind(),
                                "expression_statement"
                                    | "return_statement"
                                    | "throw_statement"
                                    | "local_variable_declaration"
                                    | "field_declaration"
                                    | "block"
                            ) {
                                break;
                            }
                            current = p.parent();
                        }
                        found
                    };

                    let lparen_line = self.line_no(ctx, &child);

                    // Detect if first argument is on a new line.
                    // When all args start on new lines, checkstyle accepts args
                    // aligned with the `new` keyword position itself.
                    let first_new_arg_on_new_line = child
                        .children()
                        .find(|c| {
                            !matches!(c.kind(), "(" | ")" | "," | "line_comment" | "block_comment")
                        })
                        .is_some_and(|first_arg| self.line_no(ctx, &first_arg) > lparen_line);

                    for arg in child.children() {
                        match arg.kind() {
                            // Skip punctuation and comments
                            "(" | ")" | "," | "line_comment" | "block_comment" => {}
                            _ => {
                                let arg_line = self.line_no(ctx, &arg);
                                if arg_line > lparen_line && ctx.is_on_start_of_line(&arg) {
                                    let actual = ctx.get_line_start(arg_line);
                                    if !skip_arg_indent_check {
                                        if ctx.force_strict_condition() {
                                            if in_try_resource {
                                                if !ctx.is_indent_acceptable(actual, &arg_indent) {
                                                    ctx.log_child_error(
                                                        &arg,
                                                        "new",
                                                        actual,
                                                        &arg_indent,
                                                    );
                                                }
                                            } else if actual < new_start {
                                                ctx.log_child_error(
                                                    &arg,
                                                    "new",
                                                    actual,
                                                    &arg_indent,
                                                );
                                            }
                                        } else if first_new_arg_on_new_line {
                                            let allow_new_alignment =
                                                self.line_wrapping_indentation <= self.basic_offset
                                                    && arg.kind() != "lambda_expression"
                                                    && actual == new_start;
                                            if !allow_new_alignment
                                                && !ctx.is_indent_acceptable(actual, &arg_indent)
                                            {
                                                ctx.log_child_error(
                                                    &arg,
                                                    "new",
                                                    actual,
                                                    &arg_indent,
                                                );
                                            }
                                        } else if actual < new_start {
                                            ctx.log_child_error(&arg, "new", actual, &arg_indent);
                                        }
                                    }
                                }
                                // Recursively check nested expressions. The
                                // check_binary_expression handler's floor-based
                                // logic handles continuation lines correctly for
                                // binary/ternary args without a separate check.
                                let nested_indent = match arg.kind() {
                                    "object_creation_expression"
                                    | "method_invocation"
                                    | "lambda_expression" => &arg_indent,
                                    _ => {
                                        if ctx.force_strict_condition() {
                                            &arg_indent
                                        } else {
                                            &new_indent
                                        }
                                    }
                                };
                                self.check_expression(ctx, &arg, nested_indent);
                            }
                        }
                    }
                }
                _ => {
                    self.check_expression(ctx, &child, &new_indent);
                }
            }
        }
    }

    /// Check indentation of array creation expression (new int[] {...}, new int[5]).
    fn check_array_creation_expression(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        self.check_array_creation_expression_with_context(ctx, node, indent, false);
    }

    /// Check indentation of array creation expression with variable init context flag.
    /// When in_variable_init is true, uses arrayInitIndent for elements.
    /// Otherwise, uses lineWrappingIndentation.
    fn check_array_creation_expression_with_context(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
        in_variable_init: bool,
    ) {
        // Get the array creation expression's starting position for line wrapping
        let new_line = self.line_no(ctx, node);
        let new_start = ctx.get_line_start(new_line);
        let new_indent = IndentLevel::new(new_start);
        let line_wrapped = new_indent.with_offset(self.line_wrapping_indentation);

        // Check dimensions_expr children (e.g., new int[42 + x])
        for child in node.children() {
            match child.kind() {
                "dimensions_expr" => {
                    // Check expressions and brackets inside dimensions
                    self.check_array_dimensions_expr(ctx, &child, &new_indent, &line_wrapped);
                }
                "dimensions" => {
                    // Check empty dimensions (e.g., new int[])
                    self.check_array_dimensions(ctx, &child, indent, &line_wrapped);
                }
                "array_initializer" => {
                    // For variable initializers, elements use arrayInitIndent from statement
                    // For expression contexts (return, method args), elements use lineWrappingIndentation
                    let base_indent = if in_variable_init {
                        new_indent.clone()
                    } else {
                        // Adjust so that arrayInitIndent offset gives lineWrappingIndentation
                        if self.line_wrapping_indentation >= self.array_init_indent {
                            new_indent.with_offset(
                                self.line_wrapping_indentation - self.array_init_indent,
                            )
                        } else {
                            new_indent.clone()
                        }
                    };
                    self.check_array_initializer(ctx, &child, &base_indent);
                }
                _ => {}
            }
        }
    }

    /// Check indentation of array dimensions expression (e.g., [42 + x]).
    fn check_array_dimensions_expr(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        base_indent: &IndentLevel,
        line_wrapped: &IndentLevel,
    ) {
        let dim_line = self.line_no(ctx, node);

        for child in node.children() {
            let child_line = self.line_no(ctx, &child);
            if child_line > dim_line && ctx.is_on_start_of_line(&child) {
                let actual = ctx.get_line_start(child_line);
                match child.kind() {
                    "]" => {
                        // Closing bracket should be at line-wrapped indent
                        if !ctx.is_indent_acceptable(actual, line_wrapped) {
                            ctx.log_error(&child, "array dimension rbracket", actual, line_wrapped);
                        }
                    }
                    _ => {
                        // Expressions inside should be at line-wrapped indent
                        if !ctx.is_indent_acceptable(actual, line_wrapped) {
                            ctx.log_child_error(&child, "array dimension", actual, line_wrapped);
                        }
                        // Recursively check the expression
                        self.check_expression(ctx, &child, base_indent);
                    }
                }
            } else if child.kind() != "[" && child.kind() != "]" {
                // Check nested expressions on same line
                self.check_expression(ctx, &child, base_indent);
            }
        }
    }

    /// Check indentation of empty array dimensions (e.g., []).
    fn check_array_dimensions(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        base_indent: &IndentLevel,
        line_wrapped: &IndentLevel,
    ) {
        let dim_line = self.line_no(ctx, node);

        for child in node.children() {
            let child_line = self.line_no(ctx, &child);
            if child_line > dim_line && ctx.is_on_start_of_line(&child) {
                let actual = ctx.get_line_start(child_line);
                // Brackets on continuation lines should be at line-wrapped indent
                let acceptable = line_wrapped.combine(base_indent);
                if !ctx.is_indent_acceptable(actual, &acceptable) {
                    ctx.log_error(&child, "array dimension", actual, line_wrapped);
                }
            }
        }
    }

    /// Check indentation of array initializer.
    fn check_array_initializer(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check braces
        // For array initializers, allow base indent, brace-adjusted, or line-wrapped
        let brace_indent = indent.with_offset(self.brace_adjustment);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);
        let acceptable = brace_indent.combine(indent).combine(&line_wrapped_indent);

        // Check opening brace if on its own line (structural - use strict)
        let lcurly = self.find_child(node, "{");
        let lcurly_on_own_line = lcurly
            .as_ref()
            .is_some_and(|lc| ctx.is_on_start_of_line(lc));
        if let Some(lcurly) = &lcurly
            && lcurly_on_own_line
        {
            let actual = ctx.column_from_node(lcurly);
            if !ctx.is_indent_exact(actual, &acceptable) {
                ctx.log_error(lcurly, "lcurly", actual, &brace_indent);
            }
        }

        // Check if the opening brace is at an acceptable position (using exact check)
        // If not, use lenient mode for child elements
        let brace_misaligned = if let Some(lcurly) = &lcurly {
            let actual = ctx.column_from_node(lcurly);
            !ctx.is_indent_exact(actual, &acceptable)
        } else {
            false
        };

        // Elements should be indented by array_init_indent from the base indent
        // When brace is on its own line, also accept brace_position + array_init_indent
        // When brace is inline with content, also accept alignment with first element
        let lcurly_line = self.line_no(ctx, node);
        let element_indent = if lcurly_on_own_line {
            if let Some(lcurly) = &lcurly {
                let brace_col = ctx.column_from_node(lcurly);
                indent
                    .with_offset(self.array_init_indent)
                    .add_acceptable(&[brace_col + self.array_init_indent])
            } else {
                indent.with_offset(self.array_init_indent)
            }
        } else {
            // Inline brace - find first content element on the opening line for alignment
            let mut first_element_col: Option<i32> = None;
            for child in node.children() {
                match child.kind() {
                    "{" | "," | "line_comment" | "block_comment" => {}
                    _ => {
                        let child_line = self.line_no(ctx, &child);
                        if child_line == lcurly_line {
                            first_element_col = Some(ctx.column_from_node(&child));
                            break;
                        }
                    }
                }
            }
            let base = indent.with_offset(self.array_init_indent);
            if let Some(col) = first_element_col {
                base.add_acceptable(&[col])
            } else {
                base
            }
        };

        for child in node.children() {
            match child.kind() {
                "{" | "}" | "," | "line_comment" | "block_comment" => {}
                "array_initializer" => {
                    // Nested array initializer
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        // Use strict checking - nested array initializers must match exactly
                        if !ctx.is_indent_exact(actual, &element_indent) {
                            ctx.log_child_error(
                                &child,
                                "array initialization",
                                actual,
                                &element_indent,
                            );
                        }
                    }
                    self.check_array_initializer(ctx, &child, &element_indent);
                }
                "array_creation_expression" => {
                    // Nested array creation (e.g., new int[] { 1, 2, 3} inside int[][])
                    // When parent brace is misaligned, use lenient mode
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        let is_acceptable = if brace_misaligned {
                            ctx.is_indent_acceptable(actual, &element_indent)
                        } else {
                            ctx.is_indent_exact(actual, &element_indent)
                        };
                        if !is_acceptable {
                            ctx.log_child_error(
                                &child,
                                "array initialization",
                                actual,
                                &element_indent,
                            );
                        }
                    }
                    self.check_array_creation_expression(ctx, &child, &element_indent);
                }
                _ => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        // Use strict checking for regular elements
                        if !ctx.is_indent_exact(actual, &element_indent) {
                            ctx.log_child_error(
                                &child,
                                "array initialization",
                                actual,
                                &element_indent,
                            );
                        }
                    }
                    self.check_expression(ctx, &child, &element_indent);
                }
            }
        }

        // Check closing brace if on its own line (structural - use strict)
        // Use same acceptable levels as opening brace
        if let Some(rcurly) = self.find_child(node, "}")
            && ctx.is_on_start_of_line(&rcurly)
        {
            let actual = ctx.column_from_node(&rcurly);
            if !ctx.is_indent_exact(actual, &acceptable) {
                ctx.log_error(&rcurly, "rcurly", actual, &brace_indent);
            }
        }
    }

    /// Check indentation of annotation array initializer (e.g., @SuppressWarnings({"a", "b"})).
    fn check_annotation_array_initializer(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        // Similar to regular array initializer but uses parent line start as base indent
        // For annotation arrays on continuation lines, also accept lineWrappingIndentation
        let brace_indent = indent.with_offset(self.brace_adjustment);
        let line_wrapped_indent = indent.with_offset(self.line_wrapping_indentation);
        let acceptable = brace_indent.combine(indent).combine(&line_wrapped_indent);

        let lcurly = self.find_child(node, "{");
        let lcurly_on_own_line = lcurly
            .as_ref()
            .is_some_and(|lc| ctx.is_on_start_of_line(lc));

        // Check opening brace if on its own line (structural - use strict)
        if let Some(lcurly) = &lcurly
            && lcurly_on_own_line
        {
            let actual = ctx.column_from_node(lcurly);
            if !ctx.is_indent_exact(actual, &acceptable) {
                ctx.log_error(lcurly, "lcurly", actual, &brace_indent);
            }
        }

        // Elements should be indented by basicOffset from brace position
        // When brace is on its own line at correct position, use brace position + basicOffset
        // Otherwise use indent + array_init_indent
        let element_indent = if lcurly_on_own_line {
            if let Some(lcurly) = &lcurly {
                let brace_col = ctx.column_from_node(lcurly);
                // Check if brace is at an acceptable position
                if ctx.is_indent_exact(brace_col, &acceptable) {
                    // Brace at correct position - use actual position + basicOffset
                    IndentLevel::new(brace_col + self.basic_offset)
                } else {
                    // Brace at wrong position - use expected + basicOffset
                    indent.with_offset(self.array_init_indent)
                }
            } else {
                indent.with_offset(self.array_init_indent)
            }
        } else {
            indent.with_offset(self.array_init_indent)
        };
        // Also accept line wrapping indentation for flexibility
        let combined_indent =
            element_indent.combine(&indent.with_offset(self.line_wrapping_indentation));
        let lcurly_line = self.line_no(ctx, node);

        for child in node.children() {
            match child.kind() {
                "{" | "}" | "," | "line_comment" | "block_comment" => {}
                _ => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        // Use exact checking for array elements - they must be at specific indents
                        if !ctx.is_indent_exact(actual, &combined_indent) {
                            ctx.log_child_error(
                                &child,
                                "annotation array initialization",
                                actual,
                                &element_indent,
                            );
                        }
                    }
                    self.check_expression(ctx, &child, &element_indent);
                }
            }
        }

        // Check closing brace if on its own line (structural - use strict)
        // Use same acceptable levels as opening brace
        if let Some(rcurly) = self.find_child(node, "}")
            && ctx.is_on_start_of_line(&rcurly)
        {
            let actual = ctx.column_from_node(&rcurly);
            if !ctx.is_indent_exact(actual, &acceptable) {
                ctx.log_error(&rcurly, "rcurly", actual, &brace_indent);
            }
        }
    }

    /// Check single-statement body (no braces) with lenient checking.
    /// Used for bodies of if/while/for/do-while without braces.
    /// Allows >= expected indent per forceStrictCondition=false behavior.
    fn check_single_statement_body(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            // Use lenient checking for single-statement bodies
            if !ctx.is_indent_acceptable(actual, indent) {
                ctx.log_child_error(node, "block", actual, indent);
            }
        }
        // Check nested expressions/statements
        self.check_expression(ctx, node, indent);

        // For nested statement-without-braces (e.g., for (...) for (...) expr),
        // also check the inner body recursively so deeply nested bodies get checked.
        let inner_body = match node.kind() {
            "for_statement" | "enhanced_for_statement" | "while_statement" | "do_statement" => {
                node.child_by_field_name("body")
            }
            "if_statement" => node.child_by_field_name("consequence"),
            _ => None,
        };
        if let Some(body) = inner_body {
            if body.kind() == "block" {
                self.check_block(ctx, &body, indent);
            } else {
                let child_indent = indent.with_offset(self.basic_offset);
                self.check_single_statement_body(ctx, &body, &child_indent);
            }
        }
    }

    // Helper methods

    /// Find a child node by kind.
    fn find_child<'a>(&self, node: &CstNode<'a>, kind: &str) -> Option<CstNode<'a>> {
        node.children().find(|c| c.kind() == kind)
    }

    /// Get line number from node (0-based).
    /// Uses binary search on precomputed line offsets for O(log n) performance.
    fn line_no(&self, ctx: &HandlerContext, node: &CstNode) -> usize {
        ctx.line_no_from_offset(node.range().start())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, Indentation::default())
    }

    fn check_source_with_config(source: &str, rule: Indentation) -> Vec<Diagnostic> {
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
    fn test_correct_class_indentation() {
        let source = r#"
class Foo {
    int x;
    void bar() {
        int y = 1;
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Expected no violations, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_incorrect_class_member_indentation() {
        let source = r#"
class Foo {
  int x;
}
"#;
        let diagnostics = check_source(source);
        assert!(
            !diagnostics.is_empty(),
            "Expected violations for incorrect indentation"
        );
    }

    #[test]
    fn test_if_statement_indentation() {
        let source = r#"
class Foo {
    void bar() {
        if (true) {
            int x = 1;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Expected no violations, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_switch_statement_indentation() {
        let source = r#"
class Foo {
    void bar() {
        switch (x) {
            case 1:
                break;
            default:
                break;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Expected no violations, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_lambda_block_brace_at_continuation_indent() {
        // Lambda with block body where { is on its own line at continuation indent.
        // The block children should be indented relative to the brace, not statement level.
        let source = r#"
class Foo {
    void bar() {
        return
            (subscription) ->
            {
                final int a = 1;
            };
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Lambda block at continuation indent should not flag children: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_ctor_call_args_strict_mode() {
        // super() arguments on continuation lines in strict mode should be accepted.
        let source = r#"
class Foo {
    Foo() {
        super(
            arg1,
            arg2);
    }
}
"#;
        let rule = Indentation {
            force_strict_condition: true,
            ..Indentation::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics.is_empty(),
            "Ctor call args at continuation indent in strict mode should pass: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_method_call_arg_lenient_no_false_positive() {
        // Method call arguments in try-with-resources context where indent already
        // includes line-wrap offset should not produce false positives.
        let source = r#"
class Foo {
    void bar() {
        try (TestMediaDriver md = TestMediaDriver.launch(
            new Context(), watcher))
        {
            int x = 1;
        }
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "Method call args in try-with-resources should not flag: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_chain_strict_check_in_argument_list() {
        // Chain dots inside argument lists should not be flagged in strict mode
        // when they're not top-level chains.
        let source = r#"
class Foo {
    void bar() {
        library.initiate(ILink3ConnectionConfiguration.builder()
            .host("127.0.0.1")
            .port(123));
    }
}
"#;
        let rule = Indentation {
            force_strict_condition: true,
            ..Indentation::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics.is_empty(),
            "Chain dots in arg lists should not be flagged in strict mode: {:?}",
            diagnostics
        );
    }
}
