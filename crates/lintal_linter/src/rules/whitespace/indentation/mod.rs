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

impl Rule for Indentation {
    fn name(&self) -> &'static str {
        "Indentation"
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
                "class_declaration" | "interface_declaration" | "enum_declaration"
                | "annotation_type_declaration" | "record_declaration" => {
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
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "package def", actual, indent);
            }
        }
    }

    /// Check indentation of import declaration.
    fn check_import_declaration(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "import", actual, indent);
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
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, type_name, actual, indent);
            }
        }

        // Check class body with increased indentation
        if let Some(body) = self.find_child(node, "class_body")
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
                "field_declaration" => self.check_member_def(ctx, &child, &child_indent),
                "method_declaration" | "constructor_declaration" => {
                    self.check_method_def(ctx, &child, &child_indent);
                }
                "class_declaration" | "interface_declaration" | "enum_declaration"
                | "annotation_type_declaration" | "record_declaration" => {
                    self.check_class_declaration(ctx, &child, &child_indent);
                }
                "static_initializer" => self.check_static_init(ctx, &child, &child_indent),
                "block" => self.check_block(ctx, &child, &child_indent), // instance initializer
                "enum_constant" => self.check_enum_constant(ctx, &child, &child_indent),
                "annotation_type_element_declaration" => {
                    self.check_annotation_element(ctx, &child, &child_indent);
                }
                _ => {}
            }
        }
    }

    /// Check indentation of braces.
    fn check_braces(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        let brace_indent = indent.with_offset(self.brace_adjustment);

        for child in node.children() {
            if matches!(child.kind(), "{" | "}") && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !brace_indent.is_acceptable(actual) && !indent.is_acceptable(actual) {
                    let brace_type = if child.kind() == "{" { "lcurly" } else { "rcurly" };
                    ctx.log_error(&child, brace_type, actual, &brace_indent);
                }
            }
        }
    }

    /// Check indentation of a field declaration.
    fn check_member_def(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "member def", actual, indent);
            }
        }
        // Check expressions in field initializers
        self.check_expression(ctx, node, indent);
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
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, type_name, actual, indent);
            }
        }

        // Check method body
        if let Some(body) = self.find_child(node, "block") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of a block.
    fn check_block(&self, ctx: &HandlerContext, node: &CstNode, parent_indent: &IndentLevel) {
        // Check braces
        self.check_braces(ctx, node, parent_indent);

        // Children should be indented by basic_offset from parent
        let child_indent = parent_indent.with_offset(self.basic_offset);

        for child in node.children() {
            match child.kind() {
                "{" | "}" => {} // Skip braces
                _ => self.check_statement(ctx, &child, &child_indent),
            }
        }
    }

    /// Check indentation of a statement.
    fn check_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        match node.kind() {
            "local_variable_declaration" | "expression_statement" | "return_statement"
            | "throw_statement" | "break_statement" | "continue_statement" | "assert_statement"
            | "yield_statement" => {
                if ctx.is_on_start_of_line(node) {
                    let actual = ctx.get_line_start(self.line_no(ctx, node));
                    if !indent.is_acceptable(actual) {
                        ctx.log_child_error(node, "block", actual, indent);
                    }
                }
                // Check expressions within the statement
                self.check_expression(ctx, node, indent);
            }
            "if_statement" => self.check_if_statement(ctx, node, indent),
            "for_statement" | "enhanced_for_statement" => self.check_for_statement(ctx, node, indent),
            "while_statement" => self.check_while_statement(ctx, node, indent),
            "do_statement" => self.check_do_while_statement(ctx, node, indent),
            "try_statement" | "try_with_resources_statement" => {
                self.check_try_statement(ctx, node, indent);
            }
            "switch_expression" | "switch_statement" => self.check_switch_statement(ctx, node, indent),
            "synchronized_statement" => self.check_synchronized_statement(ctx, node, indent),
            "labeled_statement" => self.check_labeled_statement(ctx, node, indent),
            "block" => self.check_block(ctx, node, indent),
            _ => {}
        }
    }

    /// Check indentation of if statement.
    fn check_if_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'if' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "if", actual, indent);
            }
        }

        // Check consequence (then branch)
        if let Some(consequence) = node.child_by_field_name("consequence") {
            if consequence.kind() == "block" {
                self.check_block(ctx, &consequence, indent);
            } else {
                // Single statement - should be indented
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_statement(ctx, &consequence, &stmt_indent);
            }
        }

        // Check alternative (else branch)
        if let Some(alternative) = node.child_by_field_name("alternative") {
            // Check 'else' keyword if present
            for child in node.children() {
                if child.kind() == "else" && ctx.is_on_start_of_line(&child) {
                    let actual = ctx.column_from_node(&child);
                    if !indent.is_acceptable(actual) {
                        ctx.log_error(&child, "else", actual, indent);
                    }
                }
            }

            if alternative.kind() == "block" {
                self.check_block(ctx, &alternative, indent);
            } else if alternative.kind() == "if_statement" {
                // else if - check at same level
                self.check_if_statement(ctx, &alternative, indent);
            } else {
                // Single statement - should be indented
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_statement(ctx, &alternative, &stmt_indent);
            }
        }
    }

    /// Check indentation of for statement.
    fn check_for_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'for' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "for", actual, indent);
            }
        }

        // Check body
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block(ctx, &body, indent);
            } else {
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_statement(ctx, &body, &stmt_indent);
            }
        }
    }

    /// Check indentation of while statement.
    fn check_while_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'while' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "while", actual, indent);
            }
        }

        // Check body
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block(ctx, &body, indent);
            } else {
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_statement(ctx, &body, &stmt_indent);
            }
        }
    }

    /// Check indentation of do-while statement.
    fn check_do_while_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'do' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "do", actual, indent);
            }
        }

        // Check body
        if let Some(body) = node.child_by_field_name("body") {
            if body.kind() == "block" {
                self.check_block(ctx, &body, indent);
            } else {
                let stmt_indent = indent.with_offset(self.basic_offset);
                self.check_statement(ctx, &body, &stmt_indent);
            }
        }

        // Check 'while' at end
        for child in node.children() {
            if child.kind() == "while" && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !indent.is_acceptable(actual) {
                    ctx.log_error(&child, "while", actual, indent);
                }
            }
        }
    }

    /// Check indentation of try statement.
    fn check_try_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'try' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "try", actual, indent);
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
        for child in node.children() {
            if child.kind() == "catch" && ctx.is_on_start_of_line(&child) {
                let actual = ctx.column_from_node(&child);
                if !indent.is_acceptable(actual) {
                    ctx.log_error(&child, "catch", actual, indent);
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
                if !indent.is_acceptable(actual) {
                    ctx.log_error(&child, "finally", actual, indent);
                }
            }
        }

        // Check finally body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of switch statement.
    fn check_switch_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'switch' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "switch", actual, indent);
            }
        }

        // Check switch body/block
        if let Some(body) = node.child_by_field_name("body") {
            self.check_switch_body(ctx, &body, indent);
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
                    if ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(self.line_no(ctx, &child));
                        if !case_indent.is_acceptable(actual) {
                            ctx.log_error(&child, "case", actual, case_indent);
                        }
                    }
                }
                "block" => {
                    // Case block: braces at case indent, body at case + basicOffset
                    // This is different from nested blocks which add another level
                    self.check_block(ctx, &child, case_indent);
                }
                _ => self.check_statement(ctx, &child, body_indent),
            }
        }
    }

    /// Check indentation of switch rule (case -> expr/block).
    fn check_switch_rule(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        case_indent: &IndentLevel,
    ) {
        // Check case label
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !case_indent.is_acceptable(actual) {
                ctx.log_error(node, "case", actual, case_indent);
            }
        }

        // Check the body (expression or block after ->)
        for child in node.children() {
            if child.kind() == "block" {
                self.check_block(ctx, &child, case_indent);
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
        // Check 'synchronized' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "synchronized", actual, indent);
            }
        }

        // Check body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_block(ctx, &body, indent);
        }
    }

    /// Check indentation of labeled statement.
    fn check_labeled_statement(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check label
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "label", actual, indent);
            }
        }

        // Check the labeled statement
        for child in node.children() {
            if child.kind() != "identifier" && child.kind() != ":" {
                self.check_statement(ctx, &child, indent);
            }
        }
    }

    /// Check indentation of static initializer.
    fn check_static_init(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check 'static' keyword
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
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

    /// Check indentation of enum constant.
    fn check_enum_constant(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "enum constant", actual, indent);
            }
        }
    }

    /// Check indentation of annotation element declaration.
    fn check_annotation_element(
        &self,
        ctx: &HandlerContext,
        node: &CstNode,
        indent: &IndentLevel,
    ) {
        if ctx.is_on_start_of_line(node) {
            let actual = ctx.get_line_start(self.line_no(ctx, node));
            if !indent.is_acceptable(actual) {
                ctx.log_error(node, "annotation field def", actual, indent);
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
            "object_creation_expression" => self.check_object_creation_expression(ctx, node, indent),
            "array_creation_expression" => self.check_array_creation_expression(ctx, node, indent),
            "array_initializer" => self.check_array_initializer(ctx, node, indent),
            "element_value_array_initializer" => {
                self.check_annotation_array_initializer(ctx, node, indent);
            }
            _ => {
                // Recursively check children for nested expressions
                for child in node.children() {
                    self.check_expression(ctx, &child, indent);
                }
            }
        }
    }

    /// Check indentation of lambda expression.
    fn check_lambda_expression(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Get the lambda's starting position for line wrapping calculations
        let lambda_line = self.line_no(ctx, node);
        let lambda_start = ctx.get_line_start(lambda_line);

        // Lambda indent based on actual position (for line wrapping)
        let lambda_indent = IndentLevel::new(lambda_start);

        // Lambda body indentation
        if let Some(body) = node.child_by_field_name("body") {
            let body_line = self.line_no(ctx, &body);

            if body.kind() == "block" {
                // Block body - the block should be at lambda's indent level
                // Use the actual lambda position for proper alignment
                let block_indent = if body_line > lambda_line {
                    // Block starts on a new line
                    lambda_indent.combine(indent)
                } else {
                    // Block on same line as lambda
                    indent.clone()
                };
                self.check_block(ctx, &body, &block_indent);
            } else if ctx.is_on_start_of_line(&body) {
                // Expression body on a new line - should be indented with line wrapping
                let body_indent = lambda_indent.with_offset(self.line_wrapping_indentation);
                // Also accept parent-based indentation
                let combined = body_indent
                    .combine(&indent.with_offset(self.line_wrapping_indentation))
                    .combine(indent);

                let actual = ctx.get_line_start(body_line);
                if !combined.is_acceptable(actual) {
                    ctx.log_child_error(&body, "lambda", actual, &body_indent);
                }
                // Check nested expressions in the body
                self.check_expression(ctx, &body, &lambda_indent);
            } else {
                // Same line - check nested expressions
                self.check_expression(ctx, &body, &lambda_indent);
            }
        }
    }

    /// Check indentation of method invocation.
    fn check_method_invocation(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Get the line where this method call starts
        let method_line = self.line_no(ctx, node);
        let method_start = ctx.get_line_start(method_line);

        // For chained method calls, calculate the base indent from the actual line start
        // This allows proper line wrapping for chains like:
        //   new String()
        //       .substring(0, 100)  <- indented by lineWrappingIndentation from 'new String()'
        let chain_indent = IndentLevel::new(method_start);

        // Check if this is a chained call (has an object/receiver)
        if let Some(obj) = node.child_by_field_name("object") {
            let obj_line = self.line_no(ctx, &obj);

            // If the method call starts on a new line from its object, check indentation
            if method_line > obj_line && ctx.is_on_start_of_line(node) {
                // Find the "." operator
                for child in node.children() {
                    if child.kind() == "." && ctx.is_on_start_of_line(&child) {
                        let dot_line = self.line_no(ctx, &child);
                        let actual = ctx.get_line_start(dot_line);

                        // The chain should be indented by lineWrappingIndentation from obj start
                        let obj_start = ctx.get_line_start(obj_line);
                        let expected = IndentLevel::new(obj_start)
                            .with_offset(self.line_wrapping_indentation);

                        // Also accept indent level passed in and double wrap
                        let combined = expected
                            .combine(indent)
                            .combine(&indent.with_offset(self.line_wrapping_indentation));

                        if !combined.is_acceptable(actual) {
                            ctx.log_error(&child, "method call", actual, &expected);
                        }
                        break;
                    }
                }
            }

            // Recursively check the object expression
            self.check_expression(ctx, &obj, indent);
        }

        // Check arguments
        if let Some(args) = node.child_by_field_name("arguments") {
            // Use the method call's line start as base for argument indentation
            let arg_base_indent = IndentLevel::new(method_start);
            let arg_indent = arg_base_indent.with_offset(self.line_wrapping_indentation);
            // Also accept parent-based indentation
            let combined_arg_indent = arg_indent.combine(&indent.with_offset(self.line_wrapping_indentation));

            let lparen_line = self.line_no(ctx, &args);
            let mut in_multiline_args = false;

            for child in args.children() {
                match child.kind() {
                    "(" | ")" | "," => {}
                    _ => {
                        let child_line = self.line_no(ctx, &child);
                        if child_line > lparen_line {
                            in_multiline_args = true;
                            // Arguments on new lines should be indented
                            if ctx.is_on_start_of_line(&child) {
                                let actual = ctx.get_line_start(child_line);
                                if !combined_arg_indent.is_acceptable(actual) {
                                    ctx.log_child_error(&child, "method call", actual, &arg_indent);
                                }
                            }
                        }
                        // Check nested expressions in arguments with the argument indent
                        self.check_expression(ctx, &child, &chain_indent);
                    }
                }
            }

            // Check closing paren if on its own line
            if in_multiline_args
                && let Some(rparen) = self.find_child(&args, ")")
                && ctx.is_on_start_of_line(&rparen)
            {
                let actual = ctx.column_from_node(&rparen);
                // Closing paren can align with opening, parent indent, or method start
                let rparen_acceptable = chain_indent.combine(indent);
                if !rparen_acceptable.is_acceptable(actual) {
                    ctx.log_error(&rparen, "rparen", actual, &chain_indent);
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

        // Check anonymous class body if present
        for child in node.children() {
            match child.kind() {
                "class_body" => {
                    // Anonymous class body
                    self.check_class_body(ctx, &child, indent);
                }
                "argument_list" => {
                    // Use the new expression's line start as base for argument indentation
                    let arg_indent = new_indent.with_offset(self.line_wrapping_indentation);
                    // Also accept parent-based and double-wrap indentation
                    let combined_arg_indent = arg_indent
                        .combine(&indent.with_offset(self.line_wrapping_indentation))
                        .combine(&indent.with_offset(self.line_wrapping_indentation * 2));

                    let lparen_line = self.line_no(ctx, &child);
                    for arg in child.children() {
                        match arg.kind() {
                            "(" | ")" | "," => {}
                            _ => {
                                let arg_line = self.line_no(ctx, &arg);
                                if arg_line > lparen_line && ctx.is_on_start_of_line(&arg) {
                                    let actual = ctx.get_line_start(arg_line);
                                    if !combined_arg_indent.is_acceptable(actual) {
                                        ctx.log_child_error(&arg, "new", actual, &arg_indent);
                                    }
                                }
                                // Pass the new expression's indent for nested expressions
                                self.check_expression(ctx, &arg, &new_indent);
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
        _indent: &IndentLevel,
    ) {
        // Get the array creation expression's starting position for line wrapping
        let new_line = self.line_no(ctx, node);
        let new_start = ctx.get_line_start(new_line);
        let new_indent = IndentLevel::new(new_start);

        // Check array initializer if present
        for child in node.children() {
            if child.kind() == "array_initializer" {
                // Pass the new expression's start as the base indent
                self.check_array_initializer(ctx, &child, &new_indent);
            }
        }
    }

    /// Check indentation of array initializer.
    fn check_array_initializer(&self, ctx: &HandlerContext, node: &CstNode, indent: &IndentLevel) {
        // Check braces
        let brace_indent = indent.with_offset(self.brace_adjustment);

        // Check opening brace if on its own line
        if let Some(lcurly) = self.find_child(node, "{")
            && ctx.is_on_start_of_line(&lcurly)
        {
            let actual = ctx.column_from_node(&lcurly);
            if !brace_indent.is_acceptable(actual) && !indent.is_acceptable(actual) {
                ctx.log_error(&lcurly, "lcurly", actual, &brace_indent);
            }
        }

        // Elements should be indented by array_init_indent
        let element_indent = indent.with_offset(self.array_init_indent);
        let lcurly_line = self.line_no(ctx, node);

        for child in node.children() {
            match child.kind() {
                "{" | "}" | "," => {}
                "array_initializer" => {
                    // Nested array initializer
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !element_indent.is_acceptable(actual) {
                            ctx.log_child_error(&child, "array initialization", actual, &element_indent);
                        }
                    }
                    self.check_array_initializer(ctx, &child, &element_indent);
                }
                _ => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !element_indent.is_acceptable(actual) {
                            ctx.log_child_error(&child, "array initialization", actual, &element_indent);
                        }
                    }
                    self.check_expression(ctx, &child, &element_indent);
                }
            }
        }

        // Check closing brace if on its own line
        if let Some(rcurly) = self.find_child(node, "}")
            && ctx.is_on_start_of_line(&rcurly)
        {
            let actual = ctx.column_from_node(&rcurly);
            if !brace_indent.is_acceptable(actual) && !indent.is_acceptable(actual) {
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
        let brace_indent = indent.with_offset(self.brace_adjustment);

        // Check opening brace if on its own line
        if let Some(lcurly) = self.find_child(node, "{")
            && ctx.is_on_start_of_line(&lcurly)
        {
            let actual = ctx.column_from_node(&lcurly);
            if !brace_indent.is_acceptable(actual) && !indent.is_acceptable(actual) {
                ctx.log_error(&lcurly, "lcurly", actual, &brace_indent);
            }
        }

        // Elements should be indented by array_init_indent
        let element_indent = indent.with_offset(self.array_init_indent);
        // Also accept line wrapping indentation for flexibility
        let combined_indent = element_indent.combine(&indent.with_offset(self.line_wrapping_indentation));
        let lcurly_line = self.line_no(ctx, node);

        for child in node.children() {
            match child.kind() {
                "{" | "}" | "," => {}
                _ => {
                    let child_line = self.line_no(ctx, &child);
                    if child_line > lcurly_line && ctx.is_on_start_of_line(&child) {
                        let actual = ctx.get_line_start(child_line);
                        if !combined_indent.is_acceptable(actual) {
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

        // Check closing brace if on its own line
        if let Some(rcurly) = self.find_child(node, "}")
            && ctx.is_on_start_of_line(&rcurly)
        {
            let actual = ctx.column_from_node(&rcurly);
            if !brace_indent.is_acceptable(actual) && !indent.is_acceptable(actual) {
                ctx.log_error(&rcurly, "rcurly", actual, &brace_indent);
            }
        }
    }

    // Helper methods

    /// Find a child node by kind.
    fn find_child<'a>(&self, node: &CstNode<'a>, kind: &str) -> Option<CstNode<'a>> {
        node.children().find(|c| c.kind() == kind)
    }

    /// Get line number from node (0-based).
    fn line_no(&self, ctx: &HandlerContext, node: &CstNode) -> usize {
        let offset = node.range().start();
        // Count newlines before offset
        let source = ctx.source();
        let offset_usize = usize::from(offset);
        source[..offset_usize.min(source.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count()
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
        let rule = Indentation::default();
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
        assert!(diagnostics.is_empty(), "Expected no violations, got: {:?}", diagnostics);
    }

    #[test]
    fn test_incorrect_class_member_indentation() {
        let source = r#"
class Foo {
  int x;
}
"#;
        let diagnostics = check_source(source);
        assert!(!diagnostics.is_empty(), "Expected violations for incorrect indentation");
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
        assert!(diagnostics.is_empty(), "Expected no violations, got: {:?}", diagnostics);
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
        assert!(diagnostics.is_empty(), "Expected no violations, got: {:?}", diagnostics);
    }
}
