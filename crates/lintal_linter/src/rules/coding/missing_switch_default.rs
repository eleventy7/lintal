//! MissingSwitchDefault rule implementation.
//!
//! Checks that switch statements have a default clause.
//!
//! Checkstyle equivalent: MissingSwitchDefaultCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: switch without default clause.
#[derive(Debug, Clone)]
pub struct MissingSwitchDefaultViolation;

impl Violation for MissingSwitchDefaultViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        "switch without \"default\" clause.".to_string()
    }
}

/// Configuration for MissingSwitchDefault rule.
#[derive(Debug, Clone, Default)]
pub struct MissingSwitchDefault;

const RELEVANT_KINDS: &[&str] = &["switch_expression"];

impl FromConfig for MissingSwitchDefault {
    const MODULE_NAME: &'static str = "MissingSwitchDefault";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for MissingSwitchDefault {
    fn name(&self) -> &'static str {
        "MissingSwitchDefault"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "switch_expression" {
            return vec![];
        }

        // Find the switch_block child
        let Some(switch_block) = node.children().find(|c| c.kind() == "switch_block") else {
            return vec![];
        };

        // Check if any label contains "default"
        if self.has_default_label(&switch_block) {
            return vec![];
        }

        // Report violation on the switch keyword (first child)
        vec![Diagnostic::new(MissingSwitchDefaultViolation, node.range())]
    }
}

impl MissingSwitchDefault {
    /// Check if a switch_block contains a default label anywhere.
    fn has_default_label(&self, switch_block: &CstNode) -> bool {
        let ts_node = switch_block.inner();
        let mut cursor = ts_node.walk();

        for child in ts_node.children(&mut cursor) {
            if child.kind() == "switch_block_statement_group" || child.kind() == "switch_rule" {
                let mut inner_cursor = child.walk();
                for inner_child in child.children(&mut inner_cursor) {
                    if inner_child.kind() == "switch_label" {
                        let mut label_cursor = inner_child.walk();
                        for label_child in inner_child.children(&mut label_cursor) {
                            if label_child.kind() == "default" {
                                return true;
                            }
                        }
                    }
                }
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
        let rule = MissingSwitchDefault;
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
    fn test_switch_with_default_no_violation() {
        let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1: break;
            default: break;
        }
    }
}
"#;
        let violations = check_source(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_switch_without_default_violation() {
        let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
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
    fn test_arrow_switch_with_default_no_violation() {
        let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1 -> System.out.println(1);
            default -> System.out.println(0);
        }
    }
}
"#;
        let violations = check_source(source);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_arrow_switch_without_default_violation() {
        let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
            case 1 -> System.out.println(1);
            case 2 -> System.out.println(2);
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_empty_switch_violation() {
        let source = r#"
class Foo {
    void method(int i) {
        switch (i) {
        }
    }
}
"#;
        let violations = check_source(source);
        assert_eq!(violations.len(), 1);
    }
}
