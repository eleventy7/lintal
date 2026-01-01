//! UpperEll rule implementation.
//!
//! Checks that long literals use uppercase 'L' rather than lowercase 'l'.
//! The lowercase 'l' looks too similar to '1', which can cause confusion.
//!
//! Checkstyle equivalent: UpperEllCheck
//!
//! ## Examples
//!
//! ```java
//! long bad = 123l;   // violation
//! long good = 123L;  // ok
//! ```

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: long literal uses lowercase 'l' suffix.
#[derive(Debug, Clone)]
pub struct UpperEllViolation;

impl Violation for UpperEllViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Should use uppercase 'L'.".to_string()
    }
}

/// Configuration for UpperEll rule.
///
/// This rule has no configuration options.
#[derive(Debug, Clone, Default)]
pub struct UpperEll;

const RELEVANT_KINDS: &[&str] = &[
    "decimal_integer_literal",
    "hex_integer_literal",
    "octal_integer_literal",
    "binary_integer_literal",
];

impl FromConfig for UpperEll {
    const MODULE_NAME: &'static str = "UpperEll";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for UpperEll {
    fn name(&self) -> &'static str {
        "UpperEll"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Look for integer literals (decimal, hex, octal, binary)
        let kind = node.kind();
        if kind != "decimal_integer_literal"
            && kind != "hex_integer_literal"
            && kind != "octal_integer_literal"
            && kind != "binary_integer_literal"
        {
            return vec![];
        }

        let text = node.text();
        let range = node.range();

        // Check if the literal ends with lowercase 'l'
        if text.ends_with('l') {
            // Create a fix that replaces the 'l' with 'L'
            let l_start = range.end() - TextSize::from(1u32);
            let fix_range = TextRange::new(l_start, range.end());

            let diagnostic = Diagnostic::new(UpperEllViolation, range).with_fix(Fix::safe_edit(
                Edit::range_replacement("L".to_string(), fix_range),
            ));

            return vec![diagnostic];
        }

        vec![]
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
        let ctx = CheckContext::new(source);
        let rule = UpperEll;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_lowercase_l_violation() {
        let source = r#"
class Test {
    long x = 123l;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("uppercase 'L'"));
    }

    #[test]
    fn test_uppercase_l_ok() {
        let source = r#"
class Test {
    long x = 123L;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_no_suffix_ok() {
        let source = r#"
class Test {
    int x = 123;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_hex_lowercase_l_violation() {
        let source = r#"
class Test {
    long x = 0xABCl;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_hex_uppercase_l_ok() {
        let source = r#"
class Test {
    long x = 0xABCL;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_fix_replaces_lowercase_l_with_uppercase() {
        let source = r#"
class Test {
    long x = 123l;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);

        let fix = diagnostics[0].fix.as_ref().unwrap();
        let edits = fix.edits();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].content().unwrap(), "L");
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let source = r#"
class Test {
    long a = 1l;
    long b = 2l;
    long c = 0xFFl;
    long d = 0777l;
    long e = 0b1010l;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 5);
        for diag in &diagnostics {
            assert!(
                diag.fix.is_some(),
                "All UpperEll violations should have fixes"
            );
        }
    }
}
