//! FallThrough rule implementation.
//!
//! Checks for fall-through in switch statements. A case that has statements
//! but does not terminate (break, return, throw, continue) falls through
//! to the next case.
//!
//! Checkstyle equivalent: FallThroughCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: fall through from previous branch.
#[derive(Debug, Clone)]
pub struct FallThroughViolation;

impl Violation for FallThroughViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Fall through from previous branch of the switch statement.".to_string()
    }
}

/// Configuration for FallThrough rule.
#[derive(Debug, Clone)]
pub struct FallThrough {
    pub check_last_case_group: bool,
    pub relief_pattern: Regex,
}

impl Default for FallThrough {
    fn default() -> Self {
        Self {
            check_last_case_group: false,
            relief_pattern: Regex::new(r"(?i)falls?\s*thr(u|ough)").unwrap(),
        }
    }
}

const RELEVANT_KINDS: &[&str] = &["switch_block"];

impl FromConfig for FallThrough {
    const MODULE_NAME: &'static str = "FallThrough";

    fn from_config(properties: &Properties) -> Self {
        let check_last_case_group = properties
            .get("checkLastCaseGroup")
            .is_some_and(|v| *v == "true");
        let relief_pattern = properties
            .get("reliefPattern")
            .and_then(|v| Regex::new(v).ok())
            .unwrap_or_else(|| Regex::new(r"(?i)falls?\s*thr(u|ough)").unwrap());

        Self {
            check_last_case_group,
            relief_pattern,
        }
    }
}

impl Rule for FallThrough {
    fn name(&self) -> &'static str {
        "FallThrough"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "switch_block" {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();
        let mut cursor = ts_node.walk();

        // Collect switch_block_statement_group children
        let groups: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|c| c.kind() == "switch_block_statement_group")
            .collect();

        if groups.is_empty() {
            return vec![];
        }

        let mut diagnostics = vec![];
        let group_count = groups.len();

        for (i, group) in groups.iter().enumerate() {
            let is_last = i == group_count - 1;

            // Skip the last group unless checkLastCaseGroup is set
            if is_last && !self.check_last_case_group {
                continue;
            }

            // Check if this group has statements (not just labels)
            if !self.group_has_statements(group) {
                // Empty case group (just labels) — fall-through is fine (it's a grouped case)
                continue;
            }

            // Check if the group terminates
            if self.group_terminates(group, source) {
                continue;
            }

            // Check for relief comment
            if !is_last {
                // Check between this group and the next
                if self.has_relief_comment(source, group, &groups[i + 1]) {
                    continue;
                }
            } else {
                // Last group: check between group end and switch block closing brace
                if self.has_relief_comment_last_group(source, group, &ts_node) {
                    continue;
                }
            }

            // Fall-through detected — report on the next case label (if exists)
            if !is_last {
                // Report on the next group's first switch_label
                let next_group = &groups[i + 1];
                let mut next_cursor = next_group.walk();
                for child in next_group.children(&mut next_cursor) {
                    if child.kind() == "switch_label" {
                        let range = CstNode::new(child, source).range();
                        diagnostics.push(Diagnostic::new(FallThroughViolation, range));
                        break;
                    }
                }
            } else if self.check_last_case_group {
                // Last group falls through (checkLastCaseGroup is true)
                // Report on this group's label
                let mut grp_cursor = group.walk();
                for child in group.children(&mut grp_cursor) {
                    if child.kind() == "switch_label" {
                        let range = CstNode::new(child, source).range();
                        diagnostics.push(Diagnostic::new(FallThroughViolation, range));
                        break;
                    }
                }
            }
        }

        diagnostics
    }
}

impl FallThrough {
    /// Check if a switch group has any statements (not just labels and colons).
    fn group_has_statements(&self, group: &tree_sitter::Node) -> bool {
        let mut cursor = group.walk();
        for child in group.children(&mut cursor) {
            match child.kind() {
                "switch_label" | ":" => {}
                _ => return true,
            }
        }
        false
    }

    /// Check if a switch group terminates (break/return/throw/continue at the end).
    fn group_terminates(&self, group: &tree_sitter::Node, source: &str) -> bool {
        // Get the last statement in the group (skip labels and colons)
        let mut last_statement = None;
        let mut cursor = group.walk();
        for child in group.children(&mut cursor) {
            match child.kind() {
                "switch_label" | ":" => {}
                _ => last_statement = Some(child),
            }
        }

        let Some(stmt) = last_statement else {
            return false;
        };

        self.statement_terminates(&stmt, source)
    }

    /// Recursively check if a statement terminates execution of the case.
    fn statement_terminates(&self, node: &tree_sitter::Node, source: &str) -> bool {
        match node.kind() {
            "break_statement" | "return_statement" | "throw_statement" | "continue_statement"
            | "yield_statement" => true,

            "block" => {
                // Check last statement in block
                let mut cursor = node.walk();
                let mut last = None;
                for child in node.children(&mut cursor) {
                    if child.kind() != "{" && child.kind() != "}" {
                        last = Some(child);
                    }
                }
                last.is_some_and(|s| self.statement_terminates(&s, source))
            }

            "if_statement" => {
                // Both branches must terminate
                let consequence = node.child_by_field_name("consequence");
                let alternative = node.child_by_field_name("alternative");

                let cons_terminates = consequence
                    .as_ref()
                    .is_some_and(|c| self.statement_terminates(c, source));

                let alt_terminates = alternative
                    .as_ref()
                    .is_some_and(|a| self.statement_terminates(a, source));

                // Must have both branches and both must terminate
                cons_terminates && alt_terminates
            }

            "while_statement" | "do_statement" | "for_statement" => {
                // A loop terminates the case if its body unconditionally terminates
                // via return/throw (NOT break, which exits the loop, not the case).
                let body = node.child_by_field_name("body");
                body.as_ref()
                    .is_some_and(|b| self.statement_terminates_in_loop(b, source))
            }

            "try_statement" | "try_with_resources_statement" => {
                // If finally block terminates, the whole try terminates regardless
                let mut cursor_finally = node.walk();
                for child in node.children(&mut cursor_finally) {
                    if child.kind() == "finally_clause" {
                        // finally_clause's block is a direct child, not a field
                        let mut fc = child.walk();
                        for fc_child in child.children(&mut fc) {
                            if fc_child.kind() == "block"
                                && self.statement_terminates(&fc_child, source)
                            {
                                return true;
                            }
                        }
                    }
                }

                // Body must terminate, and all catch clauses must terminate
                let body = node.child_by_field_name("body");
                let body_terminates = body
                    .as_ref()
                    .is_some_and(|b| self.statement_terminates(b, source));

                if !body_terminates {
                    return false;
                }

                // Check catch clauses
                let mut cursor = node.walk();
                let mut has_catch = false;
                for child in node.children(&mut cursor) {
                    if child.kind() == "catch_clause" {
                        has_catch = true;
                        let catch_body = child.child_by_field_name("body");
                        if !catch_body
                            .as_ref()
                            .is_some_and(|b| self.statement_terminates(b, source))
                        {
                            return false;
                        }
                    }
                }

                // Body terminates + all catches terminate (or no catches is fine if body terminates)
                body_terminates && has_catch
            }

            "synchronized_statement" => {
                let body = node.child_by_field_name("body");
                body.as_ref()
                    .is_some_and(|b| self.statement_terminates(b, source))
            }

            "expression_statement" => {
                // Check for System.exit() call
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "method_invocation" {
                        let method_text = child.utf8_text(source.as_bytes()).unwrap_or_default();
                        if method_text.starts_with("System.exit") {
                            return true;
                        }
                    }
                }
                false
            }

            "switch_expression" => {
                // A nested switch terminates the outer case if it's exhaustive
                // (has default) and all non-empty groups terminate via
                // return/throw/continue (NOT break, which only exits the inner switch)
                self.switch_exhaustively_terminates(node)
            }

            _ => false,
        }
    }

    /// Like `statement_terminates` but for loop bodies: `break` exits the loop,
    /// not the switch case, so it does NOT count as terminating.
    fn statement_terminates_in_loop(&self, node: &tree_sitter::Node, source: &str) -> bool {
        match node.kind() {
            // break exits the loop, not the switch — does NOT terminate
            "break_statement" => false,
            // These still terminate (return/throw exit the method/case entirely)
            "return_statement" | "throw_statement" | "yield_statement" => true,
            // continue goes back to loop top — the loop never finishes, so it terminates
            "continue_statement" => true,

            "block" => {
                let mut cursor = node.walk();
                let mut last = None;
                for child in node.children(&mut cursor) {
                    if child.kind() != "{" && child.kind() != "}" {
                        last = Some(child);
                    }
                }
                last.is_some_and(|s| self.statement_terminates_in_loop(&s, source))
            }

            "if_statement" => {
                let consequence = node.child_by_field_name("consequence");
                let alternative = node.child_by_field_name("alternative");
                let cons = consequence
                    .as_ref()
                    .is_some_and(|c| self.statement_terminates_in_loop(c, source));
                let alt = alternative
                    .as_ref()
                    .is_some_and(|a| self.statement_terminates_in_loop(a, source));
                cons && alt
            }

            // Nested loops: break in nested loop exits that inner loop, not ours
            // so use normal terminates for the nested loop body
            "while_statement" | "do_statement" | "for_statement" => {
                let body = node.child_by_field_name("body");
                body.as_ref()
                    .is_some_and(|b| self.statement_terminates_in_loop(b, source))
            }

            _ => self.statement_terminates(node, source),
        }
    }

    /// Check if a nested switch exhaustively terminates (has default and all groups terminate).
    /// In this context, `break` only exits the inner switch — it does NOT terminate the outer case.
    fn switch_exhaustively_terminates(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        let switch_block = node
            .children(&mut cursor)
            .find(|c| c.kind() == "switch_block");
        let Some(switch_block) = switch_block else {
            return false;
        };

        let mut sb_cursor = switch_block.walk();
        let groups: Vec<_> = switch_block
            .children(&mut sb_cursor)
            .filter(|c| c.kind() == "switch_block_statement_group")
            .collect();

        if groups.is_empty() {
            return false;
        }

        // Must have default case (exhaustive)
        let has_default = groups.iter().any(|g| {
            let mut gc = g.walk();
            g.children(&mut gc).any(|c| {
                c.kind() == "switch_label" && {
                    let mut lc = c.walk();
                    c.children(&mut lc).any(|l| l.kind() == "default")
                }
            })
        });

        if !has_default {
            return false;
        }

        // All non-empty groups must terminate the outer case
        for group in &groups {
            if !self.group_has_statements(group) {
                continue;
            }
            let mut last = None;
            let mut gc = group.walk();
            for child in group.children(&mut gc) {
                match child.kind() {
                    "switch_label" | ":" => {}
                    _ => last = Some(child),
                }
            }
            let Some(stmt) = last else {
                return false;
            };
            if !self.terminates_outer_case(&stmt) {
                return false;
            }
        }

        true
    }

    /// Check if a statement terminates the outer case when inside an inner switch.
    /// `break` only exits the inner switch, so it does NOT count.
    fn terminates_outer_case(&self, node: &tree_sitter::Node) -> bool {
        match node.kind() {
            "break_statement" => false, // only exits inner switch
            "return_statement" | "throw_statement" | "continue_statement" => true,
            "block" => {
                let mut cursor = node.walk();
                let mut last = None;
                for child in node.children(&mut cursor) {
                    if child.kind() != "{" && child.kind() != "}" {
                        last = Some(child);
                    }
                }
                last.is_some_and(|s| self.terminates_outer_case(&s))
            }
            "if_statement" => {
                let consequence = node.child_by_field_name("consequence");
                let alternative = node.child_by_field_name("alternative");
                consequence
                    .as_ref()
                    .is_some_and(|c| self.terminates_outer_case(c))
                    && alternative
                        .as_ref()
                        .is_some_and(|a| self.terminates_outer_case(a))
            }
            _ => false,
        }
    }

    /// Check for a relief comment in the last case group (between group end and switch block closing brace).
    fn has_relief_comment_last_group(
        &self,
        source: &str,
        group: &tree_sitter::Node,
        switch_block: &tree_sitter::Node,
    ) -> bool {
        let group_start = group.start_byte();
        let group_end = group.end_byte();
        let block_end = switch_block.end_byte();

        // Check text from end of group's last statement to end of switch block
        if group_end < block_end && block_end <= source.len() {
            let after_group = &source[group_end..block_end];
            if self.relief_pattern.is_match(after_group) {
                return true;
            }
        }

        // Check the last portion of the group text (inline comments like `i++; // fallthru`)
        if group_start < group_end && group_end <= source.len() {
            let group_text = &source[group_start..group_end];
            if let Some(last_nl) = group_text.rfind('\n') {
                let last_line = &group_text[last_nl..];
                if self.relief_pattern.is_match(last_line) {
                    return true;
                }
            }
        }

        false
    }

    /// Check for a relief comment (fall-through comment) between two groups.
    /// Searches the entire current group text AND the text between groups,
    /// since relief comments can appear anywhere in the falling-through case body.
    fn has_relief_comment(
        &self,
        source: &str,
        current_group: &tree_sitter::Node,
        next_group: &tree_sitter::Node,
    ) -> bool {
        let group_start = current_group.start_byte();
        let group_end = current_group.end_byte();
        let next_start = next_group.start_byte();

        // Search the full span from start of current group to start of next group
        if group_start < next_start && next_start <= source.len() {
            let full_text = &source[group_start..next_start];
            if self.relief_pattern.is_match(full_text) {
                return true;
            }
        }

        // Also check text between groups (in case the comment is after the group's AST end)
        if group_end < next_start && next_start <= source.len() {
            let between = &source[group_end..next_start];
            if self.relief_pattern.is_match(between) {
                return true;
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
        let rule = FallThrough::default();
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
    fn test_no_fallthrough() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                doSomething();
                break;
            case 2:
                doOther();
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_fallthrough() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                doSomething();
            case 2:
                doOther();
                break;
        }
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0], 7); // "case 2:" line
    }

    #[test]
    fn test_empty_case_no_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
            case 2:
                doSomething();
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_return_terminates() {
        let source = r#"
class Test {
    int method(int i) {
        switch (i) {
            case 1:
                return 1;
            case 2:
                return 2;
        }
        return 0;
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_throw_terminates() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                throw new RuntimeException();
            case 2:
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_relief_comment() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                doSomething();
                // fall through
            case 2:
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_if_else_both_terminate() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                if (true) {
                    break;
                } else {
                    break;
                }
            case 2:
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_if_without_else_does_not_terminate() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                if (true) {
                    break;
                }
            case 2:
                break;
        }
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    // Regression: relief comment inside if-branch should suppress violation (artio pattern)
    #[test]
    fn test_relief_comment_inside_if_branch() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
            {
                if (i > 0) {
                    // fall through
                } else {
                    break;
                }
            }
            case 2:
                break;
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    // Regression: relief comment in last case group should suppress violation
    #[test]
    fn test_relief_comment_last_group() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                i++;
                // fallthru
        }
    }
}
"#;
        // Default config has checkLastCaseGroup=false, so last group not checked
        assert!(check_source(source).is_empty());
    }

    // Regression: nested exhaustive switch should terminate outer case
    #[test]
    fn test_exhaustive_nested_switch_terminates() {
        let source = r#"
class Test {
    void method(int i, int j) {
        while (true) {
            switch (i) {
                case 1:
                    switch (j) {
                        case 1: continue;
                        case 2: return;
                        default: return;
                    }
                case 2:
                    break;
            }
        }
    }
}
"#;
        assert!(check_source(source).is_empty());
    }

    // Regression: nested switch without default should NOT terminate outer case
    #[test]
    fn test_non_exhaustive_nested_switch_does_not_terminate() {
        let source = r#"
class Test {
    void method(int i, int j) {
        while (true) {
            switch (i) {
                case 1:
                    switch (j) {
                        case 1: continue;
                        case 2: return;
                    }
                case 2:
                    break;
            }
        }
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    // Regression: empty nested switch should NOT terminate outer case
    #[test]
    fn test_empty_nested_switch_falls_through() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
                switch (i) {}
            case 2:
                break;
        }
    }
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }
}
