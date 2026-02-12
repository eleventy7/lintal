//! MethodLength rule implementation.
//!
//! Checks that methods and constructors do not exceed a specified number of lines.
//!
//! Checkstyle equivalent: MethodLengthCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: method is too long.
#[derive(Debug, Clone)]
pub struct MethodLengthViolation {
    pub len: usize,
    pub max: usize,
}

impl Violation for MethodLengthViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!(
            "Method length is {} lines (max allowed is {}).",
            self.len, self.max
        )
    }
}

/// Configuration for MethodLength rule.
#[derive(Debug, Clone)]
pub struct MethodLength {
    /// Maximum allowed method length in lines (default: 150).
    pub max: usize,
    /// Whether to check METHOD_DEF tokens.
    pub check_methods: bool,
    /// Whether to check CTOR_DEF tokens.
    pub check_constructors: bool,
    /// Whether to count empty lines (default: true).
    pub count_empty: bool,
}

const RELEVANT_KINDS: &[&str] = &["method_declaration", "constructor_declaration"];

impl Default for MethodLength {
    fn default() -> Self {
        Self {
            max: 150,
            check_methods: true,
            check_constructors: true,
            count_empty: true,
        }
    }
}

impl FromConfig for MethodLength {
    const MODULE_NAME: &'static str = "MethodLength";

    fn from_config(properties: &Properties) -> Self {
        let max = properties
            .get("max")
            .and_then(|s| s.parse().ok())
            .unwrap_or(150);

        let count_empty = properties
            .get("countEmpty")
            .map(|s| *s != "false")
            .unwrap_or(true);

        // Parse tokens property to determine what to check
        let (check_methods, check_constructors) = if let Some(tokens) = properties.get("tokens") {
            let has_method = tokens.contains("METHOD_DEF");
            let has_ctor = tokens.contains("CTOR_DEF");
            (has_method, has_ctor)
        } else {
            (true, true)
        };

        Self {
            max,
            check_methods,
            check_constructors,
            count_empty,
        }
    }
}

impl Rule for MethodLength {
    fn name(&self) -> &'static str {
        "MethodLength"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "method_declaration" if self.check_methods => {}
            "constructor_declaration" if self.check_constructors => {}
            _ => return vec![],
        }

        // Find the body block
        let Some(body) = node.child_by_field_name("body") else {
            // Abstract methods have no body
            return vec![];
        };

        let ts_body = body.inner();
        let start_line = ts_body.start_position().row;
        let end_line = ts_body.end_position().row;

        // Line count includes opening and closing braces
        let total_lines = end_line - start_line + 1;

        let line_count = if self.count_empty {
            total_lines
        } else {
            // Count non-empty lines within the body
            let source = ctx.source();
            let mut count = 0;
            for line_no in start_line..=end_line {
                if let Some(line) = source.lines().nth(line_no)
                    && !line.trim().is_empty()
                {
                    count += 1;
                }
            }
            count
        };

        if line_count > self.max {
            let range = node.range();
            return vec![Diagnostic::new(
                MethodLengthViolation {
                    len: line_count,
                    max: self.max,
                },
                range,
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
        check_source_with_config(source, max, true)
    }

    fn check_source_with_config(source: &str, max: usize, count_empty: bool) -> Vec<usize> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = MethodLength {
            max,
            check_methods: true,
            check_constructors: true,
            count_empty,
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
    fn test_short_method_no_violation() {
        let source = r#"
class Foo {
    void method() {
        int x = 1;
    }
}
"#;
        let violations = check_source(source, 10);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_long_method_violation() {
        let mut lines = vec!["class Foo {".to_string(), "    void method() {".to_string()];
        for i in 0..20 {
            lines.push(format!("        int x{} = {};", i, i));
        }
        lines.push("    }".to_string());
        lines.push("}".to_string());
        let source = lines.join("\n");

        let violations = check_source(&source, 10);
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_count_empty_false() {
        let mut lines = vec!["class Foo {".to_string(), "    void method() {".to_string()];
        for _ in 0..5 {
            lines.push("        int x = 1;".to_string());
            lines.push(String::new()); // empty line
        }
        lines.push("    }".to_string());
        lines.push("}".to_string());
        let source = lines.join("\n");

        // With countEmpty=true, body is 12 lines (5 code + 5 empty + 2 braces)
        let violations = check_source_with_config(&source, 8, true);
        assert_eq!(violations.len(), 1);

        // With countEmpty=false, body is 7 lines (5 code + 2 braces)
        let violations = check_source_with_config(&source, 8, false);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_abstract_method_no_body() {
        let source = r#"
abstract class Foo {
    abstract void method();
}
"#;
        let violations = check_source(source, 1);
        assert!(violations.is_empty());
    }
}
