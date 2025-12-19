//! FinalLocalVariable rule - checks that local variables that are never reassigned should be final.
//!
//! This is a complex stateful rule that tracks variable declarations and assignments.

use crate::{CheckContext, FromConfig, Rule};
use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::TextRange;
use std::collections::{HashMap, HashSet};

/// Checks that local variables that are never reassigned are declared final.
pub struct FinalLocalVariable {
    #[allow(dead_code)] // Will be used in later tasks for enhanced for loop support
    validate_enhanced_for_loop_variable: bool,
    validate_unnamed_variables: bool,
}

/// Violation for a variable that should be final.
#[derive(Debug, Clone)]
pub struct VariableShouldBeFinal {
    pub var_name: String,
}

impl Violation for VariableShouldBeFinal {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

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
    fn add_variable(&mut self, name: String, ident_range: TextRange, has_initializer: bool) {
        self.variables.insert(
            name.clone(),
            VariableCandidate {
                ident_range,
                name,
                has_initializer,
                assigned: false,
                already_assigned: false,
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
        self.variables
            .values()
            .filter(|v| {
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
                self.report_violation(candidate.ident_range, &candidate.name);
            }
        }
    }

    /// Get the current scope.
    fn current_scope(&mut self) -> Option<&mut ScopeData> {
        self.scopes.last_mut()
    }

    /// Report a violation for a variable that should be final.
    fn report_violation(&mut self, ident_range: TextRange, var_name: &str) {
        let diagnostic = Diagnostic::new(
            VariableShouldBeFinal {
                var_name: var_name.to_string(),
            },
            ident_range,
        );
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

    /// Process a variable declaration.
    fn process_variable_declaration(&mut self, node: &CstNode) {
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
                    scope.add_variable(var_name.to_string(), name_node.range(), has_initializer);
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
                    if let Some(&(before_assigned, before_already_assigned)) =
                        before_if.get(name)
                        && var.assigned && !before_assigned && !before_already_assigned
                    {
                        consequence_assignments.insert(name.clone());
                    }
                }
            }
        }

        // Process the alternative (else branch) if it exists
        let mut alternative_assignments = HashSet::new();
        let has_alternative = if let Some(alternative) = node.child_by_field_name("alternative") {
            self.visit(&alternative);

            // Detect what was assigned in the alternative
            if let Some(scope) = self.scopes.last() {
                for (name, var) in &scope.variables {
                    if let Some(&(before_assigned, before_already_assigned)) =
                        before_if.get(name)
                        && var.assigned && !before_assigned && !before_already_assigned
                    {
                        alternative_assignments.insert(name.clone());
                    }
                }
            }
            true
        } else {
            false
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
                        if let Some(var) = scope.variables.get_mut(var_name) {
                            // If the variable was marked as already_assigned due to being
                            // assigned in both branches, we need to reset that since for
                            // uninitialized variables, this is effectively a single initialization
                            if var.already_assigned && !var.has_initializer {
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
        if let Some(condition) = node.child_by_field_name("condition")
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
}

impl Rule for FinalLocalVariable {
    fn name(&self) -> &'static str {
        "FinalLocalVariable"
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
