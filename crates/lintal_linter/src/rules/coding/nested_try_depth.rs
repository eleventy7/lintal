//! NestedTryDepth rule implementation.
//!
//! Checks that try blocks are not nested too deeply.
//!
//! Checkstyle equivalent: NestedTryDepthCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: try nesting too deep.
#[derive(Debug, Clone)]
pub struct NestedTryDepthViolation {
    pub depth: usize,
    pub max: usize,
}

impl Violation for NestedTryDepthViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Nested try depth is {} (max allowed is {}).",
            self.depth, self.max
        )
    }
}

/// Configuration for NestedTryDepth rule.
#[derive(Debug, Clone)]
pub struct NestedTryDepth {
    /// Maximum allowed nesting depth (default: 1).
    pub max: usize,
}

const RELEVANT_KINDS: &[&str] = &["try_statement", "try_with_resources_statement"];

impl Default for NestedTryDepth {
    fn default() -> Self {
        Self { max: 1 }
    }
}

impl FromConfig for NestedTryDepth {
    const MODULE_NAME: &'static str = "NestedTryDepth";

    fn from_config(properties: &Properties) -> Self {
        let max = properties
            .get("max")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        Self { max }
    }
}

impl Rule for NestedTryDepth {
    fn name(&self) -> &'static str {
        "NestedTryDepth"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();
        if kind != "try_statement" && kind != "try_with_resources_statement" {
            return vec![];
        }

        // Count ancestor try statements
        let mut depth = 0;
        let mut current = node.parent();
        while let Some(parent) = current {
            let pk = parent.kind();
            if pk == "try_statement" || pk == "try_with_resources_statement" {
                depth += 1;
            }
            current = parent.parent();
        }

        if depth > self.max {
            return vec![Diagnostic::new(
                NestedTryDepthViolation {
                    depth,
                    max: self.max,
                },
                node.range(),
            )];
        }

        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::{LineIndex, SourceCode};

    fn check_source(source: &str, max: usize) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = NestedTryDepth { max };
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
    fn test_single_try_no_violation() {
        let source = r#"
class Foo {
    void method() {
        try {
            doSomething();
        } catch (Exception e) {
        }
    }
}
"#;
        let violations = check_source(source, 1);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_nested_try_at_limit_no_violation() {
        let source = r#"
class Foo {
    void method() {
        try {
            try {
                doSomething();
            } catch (Exception e) {
            }
        } catch (Exception e) {
        }
    }
}
"#;
        let violations = check_source(source, 1);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_nested_try_exceeds_limit() {
        let source = r#"
class Foo {
    void method() {
        try {
            try {
                try {
                    doSomething();
                } catch (Exception e) {
                }
            } catch (Exception e) {
            }
        } catch (Exception e) {
        }
    }
}
"#;
        let violations = check_source(source, 1);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0], 6); // innermost try
    }

    #[test]
    fn test_max_zero() {
        let source = r#"
class Foo {
    void method() {
        try {
            try {
                doSomething();
            } catch (Exception e) {
            }
        } catch (Exception e) {
        }
    }
}
"#;
        let violations = check_source(source, 0);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0], 5); // the inner try at depth 1
    }
}
