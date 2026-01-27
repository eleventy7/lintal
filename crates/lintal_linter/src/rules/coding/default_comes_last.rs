//! DefaultComesLast rule implementation.
//!
//! Checks that the `default` label is the last label in a switch block.
//! This improves readability by making the default case easy to find.
//!
//! Checkstyle equivalent: DefaultComesLastCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: default should be last label in the switch.
#[derive(Debug, Clone)]
pub struct DefaultComesLastViolation;

impl Violation for DefaultComesLastViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Default should be last label in the switch.".to_string()
    }
}

/// Violation: default should be last label in the case group (when skipIfLastAndSharedWithCase).
#[derive(Debug, Clone)]
pub struct DefaultComesLastInGroupViolation;

impl Violation for DefaultComesLastInGroupViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "Default should be last label in the case group.".to_string()
    }
}

/// Configuration for DefaultComesLast rule.
#[derive(Debug, Clone, Default)]
pub struct DefaultComesLast {
    /// When true, allow default to not be last if it's the last label
    /// in a case group shared with other case labels.
    pub skip_if_last_and_shared_with_case: bool,
}

const RELEVANT_KINDS: &[&str] = &["switch_block"];

impl FromConfig for DefaultComesLast {
    const MODULE_NAME: &'static str = "DefaultComesLast";

    fn from_config(properties: &Properties) -> Self {
        let skip_if_last_and_shared_with_case = properties
            .get("skipIfLastAndSharedWithCase")
            .is_some_and(|v| *v == "true");

        Self {
            skip_if_last_and_shared_with_case,
        }
    }
}

impl Rule for DefaultComesLast {
    fn name(&self) -> &'static str {
        "DefaultComesLast"
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

        // Collect all switch_block_statement_group and switch_rule children
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|c| c.kind() == "switch_block_statement_group" || c.kind() == "switch_rule")
            .collect();

        if children.is_empty() {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Find default labels and check their positions
        for (i, child) in children.iter().enumerate() {
            let has_default = self.has_default_label(child);
            if !has_default {
                continue;
            }

            // Check if this is the last group in the switch
            let is_last_group = i == children.len() - 1;

            if is_last_group {
                // Default is in the last group - no violation for basic check
                continue;
            }

            // Check if default falls through to the last group
            // Pattern: `default:` followed by `case X:` with shared code at the end
            if self.default_falls_through_to_last(&children, i) {
                continue;
            }

            // Default is not in the last group
            if self.skip_if_last_and_shared_with_case {
                // Check if default is the last label in its logical case group
                // A logical case group includes fall-through groups before this one
                let (default_is_last_in_group, is_shared_with_case, has_statements) =
                    self.default_position_in_group(child);

                // Also check if previous groups fall through (no statements) and have case labels
                let has_case_in_fallthrough = self.has_case_in_fallthrough_groups(&children, i);

                let effectively_shared = is_shared_with_case || has_case_in_fallthrough;

                if default_is_last_in_group && effectively_shared && has_statements {
                    // Allowed: default is last label in its logical case group
                    continue;
                }

                // Violation - default is not the last in the case group, or not shared
                if let Some(default_node) = self.find_default_label(child) {
                    let range = CstNode::new(default_node, source).range();
                    diagnostics.push(Diagnostic::new(DefaultComesLastInGroupViolation, range));
                }
            } else {
                // Standard violation
                if let Some(default_node) = self.find_default_label(child) {
                    let range = CstNode::new(default_node, source).range();
                    diagnostics.push(Diagnostic::new(DefaultComesLastViolation, range));
                }
            }
        }

        diagnostics
    }
}

impl DefaultComesLast {
    /// Check if a switch_block_statement_group or switch_rule has a default label.
    fn has_default_label(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_label" {
                let mut label_cursor = child.walk();
                for label_child in child.children(&mut label_cursor) {
                    if label_child.kind() == "default" {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Find the default label node within a group.
    fn find_default_label<'a>(
        &self,
        node: &'a tree_sitter::Node<'a>,
    ) -> Option<tree_sitter::Node<'a>> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_label" {
                let mut label_cursor = child.walk();
                for label_child in child.children(&mut label_cursor) {
                    if label_child.kind() == "default" {
                        return Some(label_child);
                    }
                }
            }
        }
        None
    }

    /// Check if the default at the given index falls through to the last group.
    ///
    /// This handles patterns like:
    /// ```java
    /// default:
    /// case "backoff":
    ///     return foo();
    /// ```
    /// Where `default:` has no statements and falls through to `case "backoff":`,
    /// which IS the last group. In this case, default is effectively last.
    fn default_falls_through_to_last(
        &self,
        children: &[tree_sitter::Node],
        default_idx: usize,
    ) -> bool {
        // First, check if the default's own group has no statements (is a fall-through)
        if self.group_has_statements(&children[default_idx]) {
            // Default group has statements, so it doesn't fall through
            return false;
        }

        // Default falls through - check if all subsequent groups until the last
        // are either fall-throughs or we reach the last group
        for i in (default_idx + 1)..children.len() {
            let is_last = i == children.len() - 1;

            if is_last {
                // We reached the last group via fall-through - default is effectively last
                return true;
            }

            // Not the last group - check if it falls through
            if self.group_has_statements(&children[i]) {
                // This group has statements but isn't the last - default doesn't reach last
                return false;
            }
        }

        false
    }

    /// Check if any fall-through groups before the given index have case labels.
    /// A fall-through group is one with no statements (only labels).
    fn has_case_in_fallthrough_groups(
        &self,
        children: &[tree_sitter::Node],
        current_idx: usize,
    ) -> bool {
        // Look backwards from current_idx
        for i in (0..current_idx).rev() {
            let group = &children[i];

            // Check if this group has statements
            let has_statements = self.group_has_statements(group);

            if has_statements {
                // Stop looking - this group doesn't fall through
                break;
            }

            // This is a fall-through group - check if it has a case label
            if self.has_case_label(group) {
                return true;
            }
        }
        false
    }

    /// Check if a group has any statements (not just labels).
    fn group_has_statements(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "switch_label" | ":" | "{" | "}" | "->" => {}
                _ => return true,
            }
        }
        false
    }

    /// Check if a group has a case label (not default).
    fn has_case_label(&self, node: &tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "switch_label" {
                let mut label_cursor = child.walk();
                if child
                    .children(&mut label_cursor)
                    .any(|c| c.kind() == "case")
                {
                    return true;
                }
            }
        }
        false
    }

    /// Check the position of default within a case group.
    /// Returns (is_default_last_label, is_shared_with_case, has_statements).
    ///
    /// For `case 1: default: break;` - default is last label, shared with case, has statements
    /// For `default: break;` - default is last label, NOT shared with case, has statements
    /// For `case 1: default: case 2: break;` - default is NOT last label
    fn default_position_in_group(&self, node: &tree_sitter::Node) -> (bool, bool, bool) {
        let mut cursor = node.walk();
        let mut labels = vec![];
        let mut has_statements = false;
        let mut has_case_label = false;

        for child in node.children(&mut cursor) {
            match child.kind() {
                "switch_label" => {
                    // Check if this is a case label (not default)
                    let mut label_cursor = child.walk();
                    let is_case = child
                        .children(&mut label_cursor)
                        .any(|c| c.kind() == "case");
                    if is_case {
                        has_case_label = true;
                    }
                    labels.push(child);
                }
                ":" | "{" | "}" | "->" => {}
                _ => {
                    // Any other kind is a statement
                    has_statements = true;
                }
            }
        }

        // Check if default is the last label
        let default_is_last = labels.last().is_some_and(|last| {
            let mut label_cursor = last.walk();
            last.children(&mut label_cursor)
                .any(|c| c.kind() == "default")
        });

        (default_is_last, has_case_label, has_statements)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str) -> Vec<usize> {
        check_source_with_config(source, false)
    }

    fn check_source_with_config(source: &str, skip_if_last_and_shared: bool) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = DefaultComesLast {
            skip_if_last_and_shared_with_case: skip_if_last_and_shared,
        };
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
    fn test_default_last_no_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            case 2: break;
            default: break;
        }
    }
}
"#;
        let violations = check_source(source);
        assert!(violations.is_empty(), "Default is last - no violation");
    }

    #[test]
    fn test_default_not_last_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            default: break;
            case 2: break;
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0], 6); // default is on line 6
    }

    #[test]
    fn test_no_default_no_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1: break;
            case 2: break;
        }
    }
}
"#;
        let violations = check_source(source);
        assert!(violations.is_empty(), "No default - no violation");
    }

    #[test]
    fn test_default_first_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            default: break;
            case 1: break;
            case 2: break;
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_skip_option_allows_shared_case() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
            default:
                break;
            case 2:
                break;
        }
    }
}
"#;
        // Without option: violation
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);

        // With option: no violation (default is last in its case group)
        let violations = check_source_with_config(source, true);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_skip_option_still_catches_middle_default() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1:
            default:
            case 2:
                break;
            case 3:
                break;
        }
    }
}
"#;
        // With option: still a violation (default is followed by case 2 before break)
        let violations = check_source_with_config(source, true);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_default_falls_through_to_last_no_violation() {
        // Pattern from artio: default: followed by case X: at the end
        let source = r#"
class Test {
    void method(String s) {
        switch (s) {
            case "a":
                return;
            case "b":
                return;
            default:
            case "c":
                return;
        }
    }
}
"#;
        let violations = check_source(source);
        assert!(
            violations.is_empty(),
            "Default falls through to last group - no violation, got {:?}",
            violations
        );
    }

    #[test]
    fn test_default_falls_through_but_not_to_last_violation() {
        // default: falls through but not to the last group
        let source = r#"
class Test {
    void method(String s) {
        switch (s) {
            case "a":
                return;
            default:
            case "b":
                return;
            case "c":
                return;
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1, "Default doesn't fall through to last");
    }

    #[test]
    fn test_arrow_switch_violation() {
        let source = r#"
class Test {
    void method(int i) {
        switch (i) {
            case 1 -> System.out.println(1);
            default -> System.out.println(0);
            case 2 -> System.out.println(2);
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);
    }
}
