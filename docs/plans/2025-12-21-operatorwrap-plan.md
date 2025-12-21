# OperatorWrap Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement OperatorWrap rule that checks operator position when expressions span multiple lines.

**Architecture:** Add to existing whitespace/ module. Check binary expressions, ternary expressions, and type bounds for operator position relative to line breaks.

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics

---

## Task 1: Create OperatorWrap rule with failing tests

**Files:**
- Create: `crates/lintal_linter/src/rules/whitespace/operator_wrap.rs`
- Modify: `crates/lintal_linter/src/rules/whitespace/mod.rs`

**Step 1: Create rule file with tests**

Create `crates/lintal_linter/src/rules/whitespace/operator_wrap.rs`:

```rust
//! OperatorWrap rule implementation.
//!
//! Checks that operators are on the correct line when expressions span multiple lines.
//!
//! Checkstyle equivalent: OperatorWrapCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: operator should be on a new line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnNewLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnNewLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on a new line.", self.operator)
    }
}

/// Violation: operator should be on the previous line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnPrevLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnPrevLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on the previous line.", self.operator)
    }
}

/// Option for where operators should be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapOption {
    /// Operator should be on a new line (default).
    #[default]
    Nl,
    /// Operator should be at end of line.
    Eol,
}

/// Configuration for OperatorWrap rule.
#[derive(Debug, Clone)]
pub struct OperatorWrap {
    pub option: WrapOption,
}

impl Default for OperatorWrap {
    fn default() -> Self {
        Self {
            option: WrapOption::Nl,
        }
    }
}

impl FromConfig for OperatorWrap {
    const MODULE_NAME: &'static str = "OperatorWrap";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match *v {
                "eol" | "EOL" => WrapOption::Eol,
                _ => WrapOption::Nl,
            })
            .unwrap_or_default();

        Self { option }
    }
}

impl Rule for OperatorWrap {
    fn name(&self) -> &'static str {
        "OperatorWrap"
    }

    fn check(&self, _ctx: &CheckContext, _node: &CstNode) -> Vec<Diagnostic> {
        // TODO: Implement
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source_nl(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap::default(); // nl option

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_eol(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap { option: WrapOption::Eol };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_nl_operator_at_end_of_line_violation() {
        // With nl option, operator at end of line is a violation
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert_eq!(diagnostics.len(), 1, "Operator at end of line should be violation with nl option");
    }

    #[test]
    fn test_nl_operator_on_new_line_ok() {
        // With nl option, operator on new line is OK
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(diagnostics.is_empty(), "Operator on new line should be OK with nl option");
    }

    #[test]
    fn test_eol_operator_on_new_line_violation() {
        // With eol option, operator on new line is a violation
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert_eq!(diagnostics.len(), 1, "Operator on new line should be violation with eol option");
    }

    #[test]
    fn test_eol_operator_at_end_of_line_ok() {
        // With eol option, operator at end of line is OK
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert!(diagnostics.is_empty(), "Operator at end of line should be OK with eol option");
    }

    #[test]
    fn test_same_line_no_violation() {
        // No violation if everything is on same line
        let source = r#"
class Test {
    void method() {
        int x = 1 + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(diagnostics.is_empty(), "Same line expression should not cause violation");
    }
}
```

**Step 2: Add to mod.rs**

Add to `crates/lintal_linter/src/rules/whitespace/mod.rs`:
- Add `mod operator_wrap;`
- Add `pub use operator_wrap::OperatorWrap;`

**Step 3: Run tests to verify they fail**

Run: `cargo test --package lintal_linter operator_wrap -- --nocapture`
Expected: Tests fail (check returns empty vec)

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/
git commit -m "test(OperatorWrap): add failing tests for rule"
```

---

## Task 2: Implement OperatorWrap rule

**Files:**
- Modify: `crates/lintal_linter/src/rules/whitespace/operator_wrap.rs`

**Step 1: Implement the check method**

The implementation needs to:
1. Check binary_expression nodes for operator position
2. Compare operator line with left operand end line and right operand start line
3. For `nl` option: violation if operator is on same line as left operand but different line than right
4. For `eol` option: violation if operator is on same line as right operand but different line than left

```rust
impl Rule for OperatorWrap {
    fn name(&self) -> &'static str {
        "OperatorWrap"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Check binary expressions
        if node.kind() != "binary_expression" {
            return vec![];
        }

        let ts_node = node.inner();
        let source_code = ctx.source_code();

        // Get the operator (middle child)
        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node.children(&mut cursor).collect();

        if children.len() < 3 {
            return vec![];
        }

        let left = &children[0];
        let operator = &children[1];
        let right = &children[2];

        // Check if expression spans multiple lines
        let left_end = lintal_text_size::TextSize::from(left.end_byte() as u32);
        let right_start = lintal_text_size::TextSize::from(right.start_byte() as u32);
        let op_start = lintal_text_size::TextSize::from(operator.start_byte() as u32);

        let left_end_line = source_code.line_column(left_end).line.get();
        let right_start_line = source_code.line_column(right_start).line.get();
        let op_line = source_code.line_column(op_start).line.get();

        // Only check if expression spans multiple lines
        if left_end_line == right_start_line {
            return vec![];
        }

        let op_text = operator.utf8_text(ctx.source().as_bytes()).unwrap_or("");
        let op_range = lintal_text_size::TextRange::new(
            op_start,
            lintal_text_size::TextSize::from(operator.end_byte() as u32),
        );

        match self.option {
            WrapOption::Nl => {
                // Operator should be on new line (same line as right operand)
                if op_line == left_end_line && op_line != right_start_line {
                    return vec![Diagnostic::new(
                        OperatorShouldBeOnNewLine { operator: op_text.to_string() },
                        op_range,
                    )];
                }
            }
            WrapOption::Eol => {
                // Operator should be at end of line (same line as left operand)
                if op_line == right_start_line && op_line != left_end_line {
                    return vec![Diagnostic::new(
                        OperatorShouldBeOnPrevLine { operator: op_text.to_string() },
                        op_range,
                    )];
                }
            }
        }

        vec![]
    }
}
```

**Step 2: Run tests to verify they pass**

Run: `cargo test --package lintal_linter operator_wrap -- --nocapture`
Expected: All tests pass

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/operator_wrap.rs
git commit -m "feat(OperatorWrap): implement rule for operator position checking"
```

---

## Task 3: Register and add checkstyle compatibility tests

**Files:**
- Modify: `crates/lintal_linter/src/rules/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_operatorwrap.rs`

**Step 1: Export from rules/mod.rs**

Add `OperatorWrap` to whitespace exports in rules/mod.rs (it should already be covered by `pub use whitespace::*;`)

**Step 2: Register in registry.rs**

Add `OperatorWrap` to imports and register it.

**Step 3: Create checkstyle compatibility tests**

Create `crates/lintal_linter/tests/checkstyle_operatorwrap.rs` with tests against checkstyle fixtures.

**Step 4: Run tests and commit**

```bash
cargo test --package lintal_linter operatorwrap -- --nocapture
git add -A
git commit -m "feat(OperatorWrap): register rule and add compat tests"
```

---

## Task 4: Final verification

**Step 1: Run full test suite**

Run: `cargo test --all`

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

**Step 3: Format and commit if needed**

```bash
cargo fmt --all
git add -A && git commit -m "chore: format code" --allow-empty
```
