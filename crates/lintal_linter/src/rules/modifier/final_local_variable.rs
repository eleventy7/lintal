//! FinalLocalVariable rule - checks that local variables that are never reassigned should be final.
//!
//! This is a complex stateful rule that tracks variable declarations and assignments.

use crate::{CheckContext, FromConfig, Rule};
use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;
use std::collections::{HashMap, HashSet};

/// Checks that local variables that are never reassigned are declared final.
pub struct FinalLocalVariable {
    validate_enhanced_for_loop_variable: bool,
    validate_unnamed_variables: bool,
}

const RELEVANT_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "static_initializer",
    "block",
    "lambda_expression",
];

/// Violation for a variable that should be final.
#[derive(Debug, Clone)]
pub struct VariableShouldBeFinal {
    pub var_name: String,
}

impl Violation for VariableShouldBeFinal {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Variable '{}' should be declared final.", self.var_name)
    }
}

impl FromConfig for FinalLocalVariable {
    const MODULE_NAME: &'static str = "FinalLocalVariable";

    fn from_config(properties: &HashMap<&str, &str>) -> Self {
        let validate_enhanced_for_loop_variable = properties
            .get("validateEnhancedForLoopVariable")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let validate_unnamed_variables = properties
            .get("validateUnnamedVariables")
            .map(|v| *v == "true")
            .unwrap_or(false);

        Self {
            validate_enhanced_for_loop_variable,
            validate_unnamed_variables,
        }
    }
}

/// Candidate variable that might need to be final.
#[derive(Debug, Clone)]
struct VariableCandidate {
    /// The range of the identifier in the source
    ident_range: TextRange,
    /// The name of the variable
    name: String,
    /// Whether this variable was declared with an initializer
    has_initializer: bool,
    /// Whether this variable has been assigned (not including initialization)
    assigned: bool,
    /// Whether this variable has been assigned more than once
    already_assigned: bool,
    /// The position to insert "final " (before the type in the declaration)
    insert_position: lintal_text_size::TextSize,
    /// Range of the declaration statement (used to group multi-variable declarations)
    declaration_range: TextRange,
}

/// Data for a single scope (method, constructor, block, etc.)
#[derive(Debug)]
struct ScopeData {
    /// Map of variable name to candidate
    variables: HashMap<String, VariableCandidate>,
}

impl ScopeData {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Add a variable declaration to this scope.
    fn add_variable(
        &mut self,
        name: String,
        ident_range: TextRange,
        has_initializer: bool,
        insert_position: lintal_text_size::TextSize,
        declaration_range: TextRange,
    ) {
        self.variables.insert(
            name.clone(),
            VariableCandidate {
                ident_range,
                name,
                has_initializer,
                assigned: false,
                already_assigned: false,
                insert_position,
                declaration_range,
            },
        );
    }

    /// Mark a variable as assigned.
    /// If it was already assigned, mark it as already_assigned (not a candidate for final).
    fn mark_assigned(&mut self, name: &str) {
        if let Some(candidate) = self.variables.get_mut(name) {
            if candidate.assigned {
                candidate.already_assigned = true;
            } else {
                candidate.assigned = true;
            }
        }
    }

    /// Get all variables that should be final (never reassigned).
    fn get_should_be_final(&self) -> Vec<&VariableCandidate> {
        // Collect all variables grouped by declaration range
        let mut declaration_groups: HashMap<TextRange, Vec<&VariableCandidate>> = HashMap::new();
        for v in self.variables.values() {
            declaration_groups
                .entry(v.declaration_range)
                .or_default()
                .push(v);
        }

        // Find declarations that have multiple variables AND at least one is modified.
        // We can't make individual variables final if they share a declaration with a
        // modified variable (e.g., "for (int i = 0, size = n; i < size; i++)" - can't
        // make just 'size' final because 'i' is modified).
        let tainted_declarations: HashSet<TextRange> = declaration_groups
            .iter()
            .filter(|(_, vars)| {
                // Only taint if there are multiple variables and at least one is modified
                vars.len() > 1 && vars.iter().any(|v| v.already_assigned || v.assigned)
            })
            .map(|(range, _)| *range)
            .collect();

        self.variables
            .values()
            .filter(|v| {
                // Skip if this variable's declaration is tainted (multi-var with modified)
                if tainted_declarations.contains(&v.declaration_range) {
                    return false;
                }

                if v.already_assigned {
                    // Assigned more than once, never a candidate
                    false
                } else if v.has_initializer {
                    // Has initializer, should be final if never reassigned
                    !v.assigned
                } else {
                    // No initializer, should be final (we don't check assigned flag because
                    // the first assignment is effectively the initialization)
                    true
                }
            })
            .collect()
    }
}

/// Visitor that processes a method/constructor/block body.
struct FinalLocalVariableVisitor<'a> {
    rule: &'a FinalLocalVariable,
    ctx: &'a CheckContext<'a>,
    /// Stack of scopes
    scopes: Vec<ScopeData>,
    /// Diagnostics collected
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FinalLocalVariableVisitor<'a> {
    fn new(rule: &'a FinalLocalVariable, ctx: &'a CheckContext<'a>) -> Self {
        Self {
            rule,
            ctx,
            scopes: vec![],
            diagnostics: vec![],
        }
    }

    /// Push a new scope.
    fn push_scope(&mut self) {
        self.scopes.push(ScopeData::new());
    }

    /// Pop a scope and report violations for variables that should be final.
    fn pop_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for candidate in scope.get_should_be_final() {
                self.report_violation(
                    candidate.ident_range,
                    &candidate.name,
                    candidate.insert_position,
                );
            }
        }
    }

    /// Get the current scope.
    fn current_scope(&mut self) -> Option<&mut ScopeData> {
        self.scopes.last_mut()
    }

    /// Report a violation for a variable that should be final.
    fn report_violation(
        &mut self,
        ident_range: TextRange,
        var_name: &str,
        insert_position: lintal_text_size::TextSize,
    ) {
        let diagnostic = Diagnostic::new(
            VariableShouldBeFinal {
                var_name: var_name.to_string(),
            },
            ident_range,
        )
        .with_fix(Fix::safe_edit(Edit::insertion(
            "final ".to_string(),
            insert_position,
        )));
        self.diagnostics.push(diagnostic);
    }

    /// Visit a node and process it.
    fn visit(&mut self, node: &CstNode) {
        match node.kind() {
            "local_variable_declaration" => {
                self.process_variable_declaration(node);
                self.visit_children(node);
            }
            "assignment_expression" => {
                self.process_assignment(node);
                self.visit_children(node);
            }
            "update_expression" => {
                self.process_update_expression(node);
                self.visit_children(node);
            }
            "if_statement" => {
                self.process_if_statement(node);
            }
            "switch_expression" | "switch_statement" => {
                self.process_switch(node);
            }
            "for_statement" | "while_statement" | "do_statement" => {
                self.process_loop(node);
            }
            "enhanced_for_statement" => {
                self.process_enhanced_for_loop(node);
            }
            "lambda_expression" => {
                self.process_lambda_expression(node);
            }
            _ => {
                self.visit_children(node);
            }
        }
    }

    /// Visit all children of a node.
    fn visit_children(&mut self, node: &CstNode) {
        for child in node.children() {
            self.visit(&child);
        }
    }

    /// Check if a node or its descendants contain an assignment to a specific variable.
    fn contains_assignment_to(&self, node: &CstNode, var_name: &str) -> bool {
        match node.kind() {
            "assignment_expression" => {
                if let Some(left) = node.child_by_field_name("left")
                    && left.kind() == "identifier"
                {
                    let name = &self.ctx.source()[left.range()];
                    if name == var_name {
                        return true;
                    }
                }
            }
            "update_expression" => {
                // Check for x++, ++x, x--, --x
                if let Some(expr) = node.child_by_field_name("argument") {
                    if expr.kind() == "identifier" {
                        let name = &self.ctx.source()[expr.range()];
                        if name == var_name {
                            return true;
                        }
                    }
                } else {
                    // Fallback: check all children
                    for child in node.children() {
                        if child.kind() == "identifier" {
                            let name = &self.ctx.source()[child.range()];
                            if name == var_name {
                                return true;
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively check children
        for child in node.children() {
            if self.contains_assignment_to(&child, var_name) {
                return true;
            }
        }

        false
    }

    /// Compute the maximum number of assignments to a variable on any single execution path
    /// through the given node. This properly handles control flow:
    /// - For if/switch: takes the MAX of branches (since only one executes)
    /// - For sequential code in blocks: SUMS assignments (all execute)
    fn max_assignments_on_path(&self, node: &CstNode, var_name: &str) -> usize {
        match node.kind() {
            "if_statement" => {
                // For if, take the max of consequence and alternative paths
                let cons_max = node
                    .child_by_field_name("consequence")
                    .map(|c| self.max_assignments_on_path(&c, var_name))
                    .unwrap_or(0);
                let alt_max = node
                    .child_by_field_name("alternative")
                    .map(|a| self.max_assignments_on_path(&a, var_name))
                    .unwrap_or(0);
                cons_max.max(alt_max)
            }
            "switch_expression" | "switch_statement" => {
                // For switch, take the max of all branches
                let mut max_count = 0;
                if let Some(body) = node.child_by_field_name("body") {
                    for child in body.children() {
                        let branch_count = self.max_assignments_on_path(&child, var_name);
                        max_count = max_count.max(branch_count);
                    }
                }
                max_count
            }
            "assignment_expression" => {
                if let Some(left) = node.child_by_field_name("left")
                    && left.kind() == "identifier"
                {
                    let name = &self.ctx.source()[left.range()];
                    if name == var_name {
                        return 1;
                    }
                }
                0
            }
            "update_expression" => {
                // Check for x++, ++x, x--, --x
                if let Some(expr) = node.child_by_field_name("argument") {
                    if expr.kind() == "identifier" {
                        let name = &self.ctx.source()[expr.range()];
                        if name == var_name {
                            return 1;
                        }
                    }
                } else {
                    // Fallback: check all children
                    for child in node.children() {
                        if child.kind() == "identifier" {
                            let name = &self.ctx.source()[child.range()];
                            if name == var_name {
                                return 1;
                            }
                        }
                    }
                }
                0
            }
            _ => {
                // For blocks and other sequential constructs, sum the assignments
                let mut total = 0;
                for child in node.children() {
                    total += self.max_assignments_on_path(&child, var_name);
                }
                total
            }
        }
    }

    /// Process a variable declaration.
    fn process_variable_declaration(&mut self, node: &CstNode) {
        // Skip variables declared in for-loop initializers (checkstyle does the same)
        // These are scoped to the loop and making them final is not typically useful
        if let Some(parent) = node.parent()
            && parent.kind() == "for_statement"
        {
            return;
        }

        // Check if already has final modifier
        // Note: modifiers might not be a field, check children
        for child in node.children() {
            if child.kind() == "modifiers" {
                if super::common::has_modifier(&child, "final") {
                    return; // Already final, skip
                }
            } else if child.kind() == "final" {
                // Sometimes final appears directly as a child
                return;
            }
        }

        // Calculate insert position for "final "
        // If there's a modifiers node, insert after it; otherwise before the type
        let insert_position = node
            .children()
            .find(|child| child.kind() == "modifiers")
            .map(|modifiers| modifiers.range().end())
            .or_else(|| {
                // Find the type node (various type kinds)
                node.children()
                    .find(|child| {
                        matches!(
                            child.kind(),
                            "type_identifier"
                                | "generic_type"
                                | "array_type"
                                | "integral_type"
                                | "floating_point_type"
                                | "boolean_type"
                                | "void_type"
                        )
                    })
                    .map(|type_node| type_node.range().start())
            })
            .unwrap_or_else(|| node.range().start());

        // Find all variable declarators
        for child in node.children() {
            if child.kind() == "variable_declarator"
                && let Some(name_node) = child.child_by_field_name("name")
            {
                let var_name = &self.ctx.source()[name_node.range()];

                // Skip unnamed variables if configured
                if !self.rule.validate_unnamed_variables && var_name == "_" {
                    continue;
                }

                // Check if this declarator has an initializer
                let has_initializer = child.child_by_field_name("value").is_some();

                // Add to current scope
                if let Some(scope) = self.current_scope() {
                    scope.add_variable(
                        var_name.to_string(),
                        name_node.range(),
                        has_initializer,
                        insert_position,
                        node.range(),
                    );
                }
            }
        }
    }

    /// Process an assignment expression.
    fn process_assignment(&mut self, node: &CstNode) {
        if let Some(left) = node.child_by_field_name("left")
            && left.kind() == "identifier"
        {
            let var_name = &self.ctx.source()[left.range()];
            // Mark as assigned in all scopes (check from innermost to outermost)
            for scope in self.scopes.iter_mut().rev() {
                if scope.variables.contains_key(var_name) {
                    scope.mark_assigned(var_name);
                    break;
                }
            }
        }
    }

    /// Process an update expression (++, --).
    fn process_update_expression(&mut self, node: &CstNode) {
        // The update_expression has the form: expression ++ or ++ expression
        // We need to find the identifier being updated
        if let Some(expr) = node.child_by_field_name("argument") {
            if expr.kind() == "identifier" {
                let var_name = &self.ctx.source()[expr.range()];
                // Mark as assigned in all scopes
                for scope in self.scopes.iter_mut().rev() {
                    if scope.variables.contains_key(var_name) {
                        scope.mark_assigned(var_name);
                        break;
                    }
                }
            }
        }
        // Fallback: check all children
        else {
            for child in node.children() {
                if child.kind() == "identifier" {
                    let var_name = &self.ctx.source()[child.range()];
                    for scope in self.scopes.iter_mut().rev() {
                        if scope.variables.contains_key(var_name) {
                            scope.mark_assigned(var_name);
                            break;
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Process a loop (for, while, do-while).
    /// Variables declared outside the loop but assigned inside cannot be final
    /// because the loop body may execute multiple times.
    fn process_loop(&mut self, node: &CstNode) {
        // Check if we have a scope
        if self.scopes.is_empty() {
            // No scope, just visit children normally
            self.visit_children(node);
            return;
        }

        // Take a snapshot of variables before the loop
        let variables_before_loop: HashSet<String> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope.variables.keys().cloned().collect()
        };

        // For for-statements, we need to create a new scope for variables declared in init
        // (they're scoped to the for loop, not the enclosing method)
        let is_for_statement = node.kind() == "for_statement";

        if is_for_statement {
            // Push a new scope for the for loop
            self.push_scope();
        }

        // Visit all children (including initialization, condition, update, body)
        self.visit_children(node);

        // Pop the for loop scope if we created one
        if is_for_statement {
            self.pop_scope();
        }

        // After visiting the loop, mark any variable declared before the loop
        // but assigned inside the loop as already_assigned (cannot be final)
        let mut assigned_in_loop = HashSet::new();

        // Find the loop body
        let body = match node.kind() {
            "for_statement" | "while_statement" => node.child_by_field_name("body"),
            "do_statement" => {
                // do-while has the body first
                node.children().find(|child| child.kind() == "block")
            }
            _ => None,
        };

        if let Some(body_node) = body {
            // Check which variables are assigned in the loop body
            for var_name in &variables_before_loop {
                if self.contains_assignment_to(&body_node, var_name) {
                    assigned_in_loop.insert(var_name.clone());
                }
            }
        }

        // For for-statements, also check the update part
        if node.kind() == "for_statement"
            && let Some(update) = node.child_by_field_name("update")
        {
            for var_name in &variables_before_loop {
                if self.contains_assignment_to(&update, var_name) {
                    assigned_in_loop.insert(var_name.clone());
                }
            }
        }

        // Also check the condition - it's executed on every iteration
        // e.g., while ((x = getValue()) > 0) - x is assigned each iteration
        if let Some(condition) = node.child_by_field_name("condition") {
            for var_name in &variables_before_loop {
                if self.contains_assignment_to(&condition, var_name) {
                    assigned_in_loop.insert(var_name.clone());
                }
            }
        }

        // Mark all variables assigned in the loop as already_assigned
        if let Some(scope) = self.scopes.last_mut() {
            for var_name in &assigned_in_loop {
                if let Some(var) = scope.variables.get_mut(var_name) {
                    var.already_assigned = true;
                }
            }
        }
    }

    /// Process an enhanced for loop (for-each).
    /// The loop variable can optionally be checked based on validateEnhancedForLoopVariable.
    fn process_enhanced_for_loop(&mut self, node: &CstNode) {
        // Check if we have a scope
        if self.scopes.is_empty() {
            // No scope, just visit children normally
            self.visit_children(node);
            return;
        }

        // Take a snapshot of variables before the loop
        let variables_before_loop: HashSet<String> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope.variables.keys().cloned().collect()
        };

        // Handle the loop variable declaration if validateEnhancedForLoopVariable is enabled
        if self.rule.validate_enhanced_for_loop_variable {
            // Find the loop variable declaration
            // enhanced_for_statement has: modifiers? type name ':' value body
            // Check if the loop variable has 'final' modifier
            let mut has_final = false;

            // Check for modifiers as direct children of enhanced_for_statement
            for child in node.children() {
                if child.kind() == "modifiers" {
                    has_final = super::common::has_modifier(&child, "final");
                    break;
                } else if child.kind() == "final" {
                    has_final = true;
                    break;
                }
            }

            // If no final modifier, check if the loop variable is reassigned in the body
            if !has_final && let Some(name_node) = node.child_by_field_name("name") {
                let var_name = &self.ctx.source()[name_node.range()];

                // Skip unnamed variables if configured
                if !self.rule.validate_unnamed_variables && var_name == "_" {
                    // Don't check this variable
                } else {
                    // Check if the variable is assigned in the loop body
                    let mut assigned_in_body = false;
                    if let Some(body) = node.child_by_field_name("body") {
                        assigned_in_body = self.contains_assignment_to(&body, var_name);
                    }

                    // If not assigned in body, it should be final
                    if !assigned_in_body {
                        // Calculate insert position for "final "
                        // If there's a modifiers node, insert after it; otherwise before the type
                        let insert_position = node
                            .children()
                            .find(|child| child.kind() == "modifiers")
                            .map(|modifiers| modifiers.range().end())
                            .or_else(|| {
                                // Find the type node
                                node.children()
                                    .find(|child| {
                                        matches!(
                                            child.kind(),
                                            "type_identifier"
                                                | "generic_type"
                                                | "array_type"
                                                | "integral_type"
                                                | "floating_point_type"
                                                | "boolean_type"
                                        )
                                    })
                                    .map(|type_node| type_node.range().start())
                            })
                            .unwrap_or_else(|| node.range().start());

                        self.report_violation(name_node.range(), var_name, insert_position);
                    }
                }
            }
        }

        // Push a new scope for variables declared inside the loop body
        // (enhanced for loop creates a new scope for the loop variable and body variables)
        self.push_scope();

        // Visit all children
        self.visit_children(node);

        // Pop the scope (this will report violations for variables declared in the loop body)
        self.pop_scope();

        // After visiting the loop, mark any variable declared before the loop
        // but assigned inside the loop as already_assigned (cannot be final)
        if let Some(body) = node.child_by_field_name("body") {
            let mut assigned_in_loop = HashSet::new();

            // Check which variables are assigned in the loop body
            for var_name in &variables_before_loop {
                if self.contains_assignment_to(&body, var_name) {
                    assigned_in_loop.insert(var_name.clone());
                }
            }

            // Mark all variables assigned in the loop as already_assigned
            if let Some(scope) = self.scopes.last_mut() {
                for var_name in &assigned_in_loop {
                    if let Some(var) = scope.variables.get_mut(var_name) {
                        var.already_assigned = true;
                    }
                }
            }
        }
    }

    /// Process an if statement with control flow analysis.
    fn process_if_statement(&mut self, node: &CstNode) {
        // Check if we have a scope
        if self.scopes.is_empty() {
            // No scope, just visit children normally
            self.visit_children(node);
            return;
        }

        // Track which variables were uninitialized before the if statement
        let uninitialized_before: HashSet<String> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope
                .variables
                .iter()
                .filter(|(_, v)| !v.assigned && !v.already_assigned)
                .map(|(name, _)| name.clone())
                .collect()
        };

        // Take a snapshot of assignments before processing branches
        let before_if: HashMap<String, (bool, bool)> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope
                .variables
                .iter()
                .map(|(name, v)| (name.clone(), (v.assigned, v.already_assigned)))
                .collect()
        };

        // Process the condition
        if let Some(condition) = node.child_by_field_name("condition") {
            self.visit(&condition);
        }

        // Process the consequence (then branch)
        let mut consequence_assignments = HashSet::new();
        if let Some(consequence) = node.child_by_field_name("consequence") {
            self.visit(&consequence);

            // Detect what was assigned in the consequence
            if let Some(scope) = self.scopes.last() {
                for (name, var) in &scope.variables {
                    if let Some(&(before_assigned, before_already_assigned)) = before_if.get(name)
                        && var.assigned
                        && !before_assigned
                        && !before_already_assigned
                    {
                        consequence_assignments.insert(name.clone());
                    }
                }
            }
        }

        // Snapshot state after consequence but before alternative
        // If already_assigned is true here, it was set during consequence processing
        // (meaning multiple assignments within the consequence branch)
        let after_consequence: HashMap<String, (bool, bool)> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope
                .variables
                .iter()
                .map(|(name, v)| (name.clone(), (v.assigned, v.already_assigned)))
                .collect()
        };

        // Process the alternative (else branch) if it exists
        let mut alternative_assignments = HashSet::new();
        let has_alternative = if let Some(alternative) = node.child_by_field_name("alternative") {
            self.visit(&alternative);

            // Detect what was assigned in the alternative
            if let Some(scope) = self.scopes.last() {
                for (name, var) in &scope.variables {
                    if let Some(&(before_assigned, before_already_assigned)) = before_if.get(name)
                        && var.assigned
                        && !before_assigned
                        && !before_already_assigned
                    {
                        alternative_assignments.insert(name.clone());
                    }
                }
            }
            true
        } else {
            false
        };

        // Pre-compute max assignments on path for each variable in the alternative.
        // This is done before mutably borrowing the scope to avoid borrow conflicts.
        let alternative_max_assignments: HashMap<String, usize> = if has_alternative {
            let mut result = HashMap::new();
            if let Some(alt) = node.child_by_field_name("alternative") {
                for var_name in &uninitialized_before {
                    result.insert(
                        var_name.clone(),
                        self.max_assignments_on_path(&alt, var_name),
                    );
                }
            }
            result
        } else {
            HashMap::new()
        };

        // Merge the results based on control flow rules
        if let Some(scope) = self.scopes.last_mut() {
            if has_alternative {
                // Both branches exist
                for var_name in &uninitialized_before {
                    let in_consequence = consequence_assignments.contains(var_name);
                    let in_alternative = alternative_assignments.contains(var_name);

                    if in_consequence && in_alternative {
                        // Assigned in both branches - this counts as single initialization
                        // The normal mark_assigned logic already set assigned=true, we don't
                        // want to mark as already_assigned
                        // However, we need to "undo" the double-assignment marking
                        // BUT only if the variable wasn't assigned multiple times within a single branch
                        if let Some(var) = scope.variables.get_mut(var_name) {
                            // If the variable was marked as already_assigned due to being
                            // assigned in both branches, we need to reset that since for
                            // uninitialized variables, this is effectively a single initialization.
                            // BUT don't reset if it was assigned multiple times within a single branch.
                            // Check if already_assigned was true after consequence (= multiple in consequence)
                            let assigned_multiple_in_consequence = after_consequence
                                .get(var_name)
                                .map(|(_, aa)| *aa)
                                .unwrap_or(false)
                                && !before_if.get(var_name).map(|(_, aa)| *aa).unwrap_or(false);

                            // Also check if any path in the alternative has multiple assignments.
                            // This catches cases like: else { x = 1; if (cond) { x = 2; } }
                            // where x can be assigned twice on a single path.
                            let assigned_multiple_in_alternative = alternative_max_assignments
                                .get(var_name)
                                .copied()
                                .unwrap_or(0)
                                > 1;

                            if var.already_assigned
                                && !var.has_initializer
                                && !assigned_multiple_in_consequence
                                && !assigned_multiple_in_alternative
                            {
                                var.already_assigned = false;
                                var.assigned = true;
                            }
                        }
                    } else if in_consequence || in_alternative {
                        // Assigned in only one branch - still OK as single initialization
                        // Don't mark as already_assigned
                    }
                }

                // For variables that were already initialized before the if
                for (var_name, var) in scope.variables.iter_mut() {
                    if !uninitialized_before.contains(var_name) && var.has_initializer {
                        // Variable was initialized before
                        let in_consequence = consequence_assignments.contains(var_name);
                        let in_alternative = alternative_assignments.contains(var_name);

                        if in_consequence || in_alternative {
                            // Assigned in at least one branch after being initialized
                            var.already_assigned = true;
                        }
                    }
                }
            } else {
                // Only consequence branch exists (no else)
                // Variables assigned in the consequence that were already initialized should be marked
                for (var_name, var) in scope.variables.iter_mut() {
                    if !uninitialized_before.contains(var_name)
                        && var.has_initializer
                        && consequence_assignments.contains(var_name)
                    {
                        var.already_assigned = true;
                    }
                }
            }
        }
    }

    /// Process a switch statement or switch expression with control flow analysis.
    fn process_switch(&mut self, node: &CstNode) {
        // Check if we have a scope
        if self.scopes.is_empty() {
            // No scope, just visit children normally
            self.visit_children(node);
            return;
        }

        // Track which variables were uninitialized before the switch
        let uninitialized_before: HashSet<String> = {
            let current_scope = self.scopes.last().unwrap();
            current_scope
                .variables
                .iter()
                .filter(|(_, v)| !v.has_initializer && !v.assigned && !v.already_assigned)
                .map(|(name, _)| name.clone())
                .collect()
        };

        // Process the condition/value
        if let Some(condition) = node
            .child_by_field_name("condition")
            .or_else(|| node.child_by_field_name("value"))
        {
            self.visit(&condition);
        }

        // Get the switch body
        let switch_body = node.child_by_field_name("body");
        if switch_body.is_none() {
            self.visit_children(node);
            return;
        }
        let switch_body = switch_body.unwrap();

        // Collect all branches (switch_block_statement_group for traditional switches,
        // switch_rule for arrow-style cases, and switch_label for expressions)
        let mut branches: Vec<CstNode> = Vec::new();

        for child in switch_body.children() {
            match child.kind() {
                "switch_block_statement_group" => {
                    // Traditional switch case with statements
                    branches.push(child);
                }
                "switch_rule" => {
                    // Arrow-style case (Java 14+)
                    branches.push(child);
                }
                _ => {}
            }
        }

        // First, visit all branches to track assignments globally
        for branch in &branches {
            self.visit(branch);
        }

        // Now analyze which variables were assigned in which branches
        // We do this by checking the AST of each branch, not the visitor state
        let mut branch_assignments: Vec<HashSet<String>> = Vec::new();

        if let Some(scope) = self.scopes.last() {
            for branch in &branches {
                let mut assignments = HashSet::new();
                // Check each variable to see if it's assigned in this branch
                for var_name in scope.variables.keys() {
                    if self.contains_assignment_to(branch, var_name) {
                        assignments.insert(var_name.clone());
                    }
                }
                branch_assignments.push(assignments);
            }
        }

        // Merge the results based on control flow rules
        // Key insight: A variable should be final if it's assigned at most once in each execution path.
        // For switches, each branch is a different execution path, so:
        // - If a variable is assigned in any branch(es) and nowhere else, it's a final candidate
        // - If a variable was initialized before the switch and is assigned in any branch, it's NOT a candidate (assigned twice)
        // - If a variable is assigned in multiple branches, that's OK (different execution paths)

        if let Some(scope) = self.scopes.last_mut() {
            // Find all variables assigned in at least one branch
            let mut assigned_in_switch: HashSet<String> = HashSet::new();
            for assignments in &branch_assignments {
                assigned_in_switch.extend(assignments.iter().cloned());
            }

            // For uninitialized variables (no initializer, not assigned before)
            for var_name in &uninitialized_before {
                if assigned_in_switch.contains(var_name) {
                    // Variable is assigned somewhere in the switch
                    // This counts as the first (and possibly only) assignment
                    // Reset the already_assigned flag if it was set during visitor traversal
                    if let Some(var) = scope.variables.get_mut(var_name)
                        && var.already_assigned
                    {
                        // The variable was marked as already_assigned because the visitor
                        // saw multiple assignments (one per branch), but these are in different
                        // execution paths, so it's actually just one assignment per path
                        var.already_assigned = false;
                        var.assigned = true;
                    }
                }
            }

            // For variables that were already initialized before the switch
            for (var_name, var) in scope.variables.iter_mut() {
                if !uninitialized_before.contains(var_name) && var.has_initializer {
                    // Variable was initialized before
                    if assigned_in_switch.contains(var_name) {
                        // Assigned in switch after being initialized - this is a second assignment
                        var.already_assigned = true;
                    }
                }
            }
        }
    }

    /// Process a lambda expression - creates a new scope.
    /// Lambda parameters should NOT be checked (they're not local variables).
    fn process_lambda_expression(&mut self, node: &CstNode) {
        // Lambda expressions have their own scope
        // Parameters are not checked (they're parameters, not local variables)

        // Find the lambda body
        // Lambda can have: parameters and body
        // Body can be an expression or a block
        if let Some(body) = node.child_by_field_name("body") {
            // Only process if it's a block (contains local variables)
            if body.kind() == "block" {
                // Create a new scope for the lambda
                self.push_scope();
                self.visit(&body);
                self.pop_scope();
            } else {
                // Expression body - just visit it (might contain nested lambdas)
                self.visit(&body);
            }
        } else {
            // Fallback: visit all children
            self.visit_children(node);
        }
    }
}

impl Rule for FinalLocalVariable {
    fn name(&self) -> &'static str {
        "FinalLocalVariable"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only process at the top-level nodes that establish scopes
        match node.kind() {
            "method_declaration" | "constructor_declaration" => {
                if let Some(body) = node.child_by_field_name("body") {
                    let mut visitor = FinalLocalVariableVisitor::new(self, ctx);
                    visitor.push_scope();
                    visitor.visit(&body);
                    visitor.pop_scope();
                    return visitor.diagnostics;
                }
            }
            "static_initializer" => {
                // Static initializer block - find the block child
                for child in node.children() {
                    if child.kind() == "block" {
                        let mut visitor = FinalLocalVariableVisitor::new(self, ctx);
                        visitor.push_scope();
                        visitor.visit(&child);
                        visitor.pop_scope();
                        return visitor.diagnostics;
                    }
                }
            }
            "block" => {
                // Only process instance initializer blocks (parent is class_body)
                if let Some(parent) = node.parent()
                    && parent.kind() == "class_body"
                {
                    let mut visitor = FinalLocalVariableVisitor::new(self, ctx);
                    visitor.push_scope();
                    visitor.visit(node);
                    visitor.pop_scope();
                    return visitor.diagnostics;
                }
            }
            "lambda_expression" => {
                // Process lambda expressions (they can appear in field initializers, etc.)
                if let Some(body) = node.child_by_field_name("body")
                    && body.kind() == "block"
                {
                    let mut visitor = FinalLocalVariableVisitor::new(self, ctx);
                    visitor.push_scope();
                    visitor.visit(&body);
                    visitor.pop_scope();
                    return visitor.diagnostics;
                }
            }
            _ => {}
        }
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_config_defaults() {
        let properties = HashMap::new();
        let rule = FinalLocalVariable::from_config(&properties);
        assert!(!rule.validate_enhanced_for_loop_variable);
        assert!(!rule.validate_unnamed_variables);
    }

    #[test]
    fn test_from_config_custom() {
        let mut properties = HashMap::new();
        properties.insert("validateEnhancedForLoopVariable", "true");
        properties.insert("validateUnnamedVariables", "true");
        let rule = FinalLocalVariable::from_config(&properties);
        assert!(rule.validate_enhanced_for_loop_variable);
        assert!(rule.validate_unnamed_variables);
    }
}
