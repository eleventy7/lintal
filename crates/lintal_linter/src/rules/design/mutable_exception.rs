//! MutableException rule implementation.
//!
//! Checks that exception classes have only final fields.
//!
//! Checkstyle equivalent: MutableExceptionCheck

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use regex::Regex;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: exception field must be final.
#[derive(Debug, Clone)]
pub struct MutableExceptionViolation {
    pub name: String,
}

impl Violation for MutableExceptionViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    fn message(&self) -> String {
        format!("The field '{}' must be declared final.", self.name)
    }
}

/// Configuration for MutableException rule.
#[derive(Debug, Clone)]
pub struct MutableException {
    /// Regex pattern for class names that are considered exception classes.
    pub format: Regex,
    /// Regex pattern for superclass names that indicate an exception class.
    pub extended_class_name_format: Regex,
}

impl Default for MutableException {
    fn default() -> Self {
        Self {
            format: Regex::new(r"^.*Exception$|^.*Error$|^.*Throwable$").unwrap(),
            extended_class_name_format: Regex::new(r"^.*Exception$|^.*Error$|^.*Throwable$")
                .unwrap(),
        }
    }
}

const RELEVANT_KINDS: &[&str] = &["class_declaration"];

impl FromConfig for MutableException {
    const MODULE_NAME: &'static str = "MutableException";

    fn from_config(properties: &Properties) -> Self {
        let format = properties
            .get("format")
            .and_then(|v| Regex::new(v).ok())
            .unwrap_or_else(|| Regex::new(r"^.*Exception$|^.*Error$|^.*Throwable$").unwrap());
        let extended_class_name_format = properties
            .get("extendedClassNameFormat")
            .and_then(|v| Regex::new(v).ok())
            .unwrap_or_else(|| Regex::new(r"^.*Exception$|^.*Error$|^.*Throwable$").unwrap());

        Self {
            format,
            extended_class_name_format,
        }
    }
}

impl Rule for MutableException {
    fn name(&self) -> &'static str {
        "MutableException"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if node.kind() != "class_declaration" {
            return vec![];
        }

        // Check if this is an exception class
        if !self.is_exception_class(ctx, node) {
            return vec![];
        }

        let Some(body) = node.child_by_field_name("body") else {
            return vec![];
        };

        let mut diagnostics = vec![];

        for child in body.children() {
            if child.kind() != "field_declaration" {
                continue;
            }

            // Check if the field has the final modifier
            let has_final = child
                .children()
                .any(|c| c.kind() == "modifiers" && c.children().any(|m| m.kind() == "final"));

            if has_final {
                continue;
            }

            // Report each variable declarator in the non-final field
            for decl_child in child.children() {
                if decl_child.kind() == "variable_declarator"
                    && let Some(name_node) = decl_child.child_by_field_name("name")
                {
                    let name = &ctx.source()[name_node.range()];
                    diagnostics.push(Diagnostic::new(
                        MutableExceptionViolation {
                            name: name.to_string(),
                        },
                        name_node.range(),
                    ));
                }
            }
        }

        diagnostics
    }
}

impl MutableException {
    /// Check if a class is an exception class by checking the class name
    /// against the format pattern. The class must also extend something.
    fn is_exception_class(&self, ctx: &CheckContext, node: &CstNode) -> bool {
        // Must extend something
        if node.child_by_field_name("superclass").is_none() {
            return false;
        }

        // Check own class name against format pattern
        if let Some(name_node) = node.child_by_field_name("name") {
            let name = &ctx.source()[name_node.range()];
            if self.format.is_match(name) {
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
        let rule = MutableException::default();
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
    fn test_mutable_field_in_exception() {
        let source = r#"
class MyException extends Exception {
    int errorCode;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_final_field_in_exception() {
        let source = r#"
class MyException extends Exception {
    final int errorCode = 0;
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_non_exception_class() {
        let source = r#"
class MyService {
    int value;
}
"#;
        assert!(check_source(source).is_empty());
    }

    #[test]
    fn test_error_class() {
        let source = r#"
class MyError extends Error {
    String detail;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn test_mixed_fields() {
        let source = r#"
class MyException extends RuntimeException {
    final String message = "";
    int code;
    String detail;
}
"#;
        let v = check_source(source);
        assert_eq!(v.len(), 2); // code and detail
    }

    // Regression: class extending exception but name not matching pattern should NOT be flagged
    #[test]
    fn test_non_matching_name_extends_exception() {
        let source = r#"
class FooExceptionThisIsNot extends RuntimeException {
    int errorCode;
}
"#;
        assert!(check_source(source).is_empty());
    }
}
