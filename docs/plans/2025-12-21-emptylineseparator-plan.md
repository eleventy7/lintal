# EmptyLineSeparator Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement EmptyLineSeparator rule for enforcing blank lines between class members.

**Architecture:** Add to whitespace/ module. Check class body children for proper blank line separation.

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics

---

## Task 1: Create EmptyLineSeparator rule skeleton with failing tests

**Files:**
- Create: `crates/lintal_linter/src/rules/whitespace/empty_line_separator.rs`
- Modify: `crates/lintal_linter/src/rules/whitespace/mod.rs`

**Step 1: Create rule file with violation types and config**

```rust
//! EmptyLineSeparator rule implementation.
//!
//! Checks that class members are separated by empty lines.
//!
//! Checkstyle equivalent: EmptyLineSeparatorCheck

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, FixAvailability, Violation};
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: element should be separated from previous line.
#[derive(Debug, Clone)]
pub struct ShouldBeSeparated {
    pub element: String,
}

impl Violation for ShouldBeSeparated {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be separated from previous line.", self.element)
    }
}

/// Violation: element has too many empty lines before it.
#[derive(Debug, Clone)]
pub struct TooManyEmptyLines {
    pub element: String,
}

impl Violation for TooManyEmptyLines {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' has more than 1 empty lines before.", self.element)
    }
}

/// Token types that can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmptyLineSeparatorToken {
    PackageDef,
    Import,
    StaticImport,
    ClassDef,
    InterfaceDef,
    EnumDef,
    StaticInit,
    InstanceInit,
    MethodDef,
    CtorDef,
    VariableDef,
    RecordDef,
    CompactCtorDef,
}

impl EmptyLineSeparatorToken {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "PACKAGE_DEF" => Some(Self::PackageDef),
            "IMPORT" => Some(Self::Import),
            "STATIC_IMPORT" => Some(Self::StaticImport),
            "CLASS_DEF" => Some(Self::ClassDef),
            "INTERFACE_DEF" => Some(Self::InterfaceDef),
            "ENUM_DEF" => Some(Self::EnumDef),
            "STATIC_INIT" => Some(Self::StaticInit),
            "INSTANCE_INIT" => Some(Self::InstanceInit),
            "METHOD_DEF" => Some(Self::MethodDef),
            "CTOR_DEF" => Some(Self::CtorDef),
            "VARIABLE_DEF" => Some(Self::VariableDef),
            "RECORD_DEF" => Some(Self::RecordDef),
            "COMPACT_CTOR_DEF" => Some(Self::CompactCtorDef),
            _ => None,
        }
    }

    fn to_checkstyle_name(self) -> &'static str {
        match self {
            Self::PackageDef => "PACKAGE_DEF",
            Self::Import => "IMPORT",
            Self::StaticImport => "STATIC_IMPORT",
            Self::ClassDef => "CLASS_DEF",
            Self::InterfaceDef => "INTERFACE_DEF",
            Self::EnumDef => "ENUM_DEF",
            Self::StaticInit => "STATIC_INIT",
            Self::InstanceInit => "INSTANCE_INIT",
            Self::MethodDef => "METHOD_DEF",
            Self::CtorDef => "CTOR_DEF",
            Self::VariableDef => "VARIABLE_DEF",
            Self::RecordDef => "RECORD_DEF",
            Self::CompactCtorDef => "COMPACT_CTOR_DEF",
        }
    }

    fn default_tokens() -> HashSet<Self> {
        [
            Self::PackageDef,
            Self::Import,
            Self::StaticImport,
            Self::ClassDef,
            Self::InterfaceDef,
            Self::EnumDef,
            Self::StaticInit,
            Self::InstanceInit,
            Self::MethodDef,
            Self::CtorDef,
            Self::VariableDef,
            Self::RecordDef,
            Self::CompactCtorDef,
        ]
        .into_iter()
        .collect()
    }
}

/// Configuration for EmptyLineSeparator rule.
#[derive(Debug, Clone)]
pub struct EmptyLineSeparator {
    pub allow_no_empty_line_between_fields: bool,
    pub allow_multiple_empty_lines: bool,
    pub allow_multiple_empty_lines_inside_class_members: bool,
    pub tokens: HashSet<EmptyLineSeparatorToken>,
}

impl Default for EmptyLineSeparator {
    fn default() -> Self {
        Self {
            allow_no_empty_line_between_fields: false,
            allow_multiple_empty_lines: true,
            allow_multiple_empty_lines_inside_class_members: true,
            tokens: EmptyLineSeparatorToken::default_tokens(),
        }
    }
}

impl FromConfig for EmptyLineSeparator {
    const MODULE_NAME: &'static str = "EmptyLineSeparator";

    fn from_config(properties: &Properties) -> Self {
        let allow_no_empty_line_between_fields = properties
            .get("allowNoEmptyLineBetweenFields")
            .map(|v| *v == "true")
            .unwrap_or(false);

        let allow_multiple_empty_lines = properties
            .get("allowMultipleEmptyLines")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let allow_multiple_empty_lines_inside_class_members = properties
            .get("allowMultipleEmptyLinesInsideClassMembers")
            .map(|v| *v == "true")
            .unwrap_or(true);

        let tokens = properties
            .get("tokens")
            .map(|v| {
                v.split(',')
                    .filter_map(|s| EmptyLineSeparatorToken::from_str(s.trim()))
                    .collect()
            })
            .unwrap_or_else(EmptyLineSeparatorToken::default_tokens);

        Self {
            allow_no_empty_line_between_fields,
            allow_multiple_empty_lines,
            allow_multiple_empty_lines_inside_class_members,
            tokens,
        }
    }
}

impl Rule for EmptyLineSeparator {
    fn name(&self) -> &'static str {
        "EmptyLineSeparator"
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

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = EmptyLineSeparator::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_with_config(source: &str, rule: EmptyLineSeparator) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_method_needs_blank_line() {
        let source = r#"
class Test {
    void method1() {}
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "method2 should need blank line before it"
        );
    }

    #[test]
    fn test_method_has_blank_line_ok() {
        let source = r#"
class Test {
    void method1() {}

    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "method2 has blank line, should be OK"
        );
    }

    #[test]
    fn test_constructor_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    Test() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.iter().any(|d| d.message().contains("CTOR_DEF")),
            "constructor should need blank line"
        );
    }

    #[test]
    fn test_field_needs_blank_line_default() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let diagnostics = check_source(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "field y should need blank line (default config)"
        );
    }

    #[test]
    fn test_field_no_blank_line_allowed() {
        let source = r#"
class Test {
    private int x;
    private int y;
}
"#;
        let rule = EmptyLineSeparator {
            allow_no_empty_line_between_fields: true,
            ..Default::default()
        };
        let diagnostics = check_source_with_config(source, rule);
        assert!(
            diagnostics.is_empty(),
            "fields without blank lines should be OK when allowNoEmptyLineBetweenFields=true"
        );
    }

    #[test]
    fn test_static_init_needs_blank_line() {
        let source = r#"
class Test {
    private int x;
    static {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message().contains("STATIC_INIT")),
            "static init should need blank line"
        );
    }

    #[test]
    fn test_comment_before_method_ok() {
        let source = r#"
class Test {
    void method1() {}

    // comment before method2
    void method2() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "blank line before comment should satisfy requirement"
        );
    }

    #[test]
    fn test_first_member_no_violation() {
        let source = r#"
class Test {
    void method1() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(
            diagnostics.is_empty(),
            "first member should not need blank line"
        );
    }
}
```

**Step 2: Add to mod.rs**

Add to `crates/lintal_linter/src/rules/whitespace/mod.rs`:
- Add `mod empty_line_separator;`
- Add `pub use empty_line_separator::EmptyLineSeparator;`

**Step 3: Run tests to verify they fail**

```bash
cargo test --package lintal_linter empty_line_separator -- --nocapture
```

Expected: Tests fail (check returns empty vec)

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/
git commit -m "test(EmptyLineSeparator): add failing tests for rule"
```

---

## Task 2: Implement class body member checking

**Files:**
- Modify: `crates/lintal_linter/src/rules/whitespace/empty_line_separator.rs`

**Step 1: Implement the check method for class_body**

The implementation needs to:
1. Only process `class_body`, `interface_body`, `enum_body` nodes (to check members once per class)
2. Iterate through children, tracking previous non-comment sibling
3. For each checked token, verify blank line exists before it

```rust
impl Rule for EmptyLineSeparator {
    fn name(&self) -> &'static str {
        "EmptyLineSeparator"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let kind = node.kind();

        // Only process container bodies
        if kind != "class_body" && kind != "interface_body" && kind != "enum_body" {
            return vec![];
        }

        let ts_node = node.inner();
        let source_code = ctx.source_code();
        let mut diagnostics = vec![];

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node.children(&mut cursor).collect();

        // Track previous non-comment element
        let mut prev_end_line: Option<usize> = None;
        let mut prev_was_field = false;

        for child in &children {
            // Skip braces and extra nodes
            if child.kind() == "{" || child.kind() == "}" || child.is_extra() {
                continue;
            }

            // Comments don't count as "previous" - they attach to next element
            if child.kind() == "line_comment" || child.kind() == "block_comment" {
                continue;
            }

            let token_type = self.node_to_token(child.kind());

            // Skip if this token type is not being checked
            if let Some(token) = token_type {
                if !self.tokens.contains(&token) {
                    // Still track it for prev_end_line
                    prev_end_line = Some(child.end_position().row);
                    prev_was_field = token == EmptyLineSeparatorToken::VariableDef;
                    continue;
                }
            }

            // Check if blank line is needed
            if let Some(prev_line) = prev_end_line {
                let current_start_line = self.find_start_line_before_comments(&children, child, ctx);
                let empty_lines = current_start_line.saturating_sub(prev_line + 1);

                // Check for field-to-field transition
                let is_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
                let field_to_field = prev_was_field && is_field;

                if empty_lines == 0 {
                    // Skip violation if field-to-field and allowed
                    if field_to_field && self.allow_no_empty_line_between_fields {
                        // OK, no violation
                    } else if let Some(token) = token_type {
                        let start = lintal_text_size::TextSize::from(child.start_byte() as u32);
                        let end = lintal_text_size::TextSize::from(child.start_byte() as u32 + 1);
                        diagnostics.push(Diagnostic::new(
                            ShouldBeSeparated {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            lintal_text_size::TextRange::new(start, end),
                        ));
                    }
                } else if empty_lines > 1 && !self.allow_multiple_empty_lines {
                    if let Some(token) = token_type {
                        let start = lintal_text_size::TextSize::from(child.start_byte() as u32);
                        let end = lintal_text_size::TextSize::from(child.start_byte() as u32 + 1);
                        diagnostics.push(Diagnostic::new(
                            TooManyEmptyLines {
                                element: token.to_checkstyle_name().to_string(),
                            },
                            lintal_text_size::TextRange::new(start, end),
                        ));
                    }
                }
            }

            prev_end_line = Some(child.end_position().row);
            prev_was_field = token_type == Some(EmptyLineSeparatorToken::VariableDef);
        }

        diagnostics
    }
}

impl EmptyLineSeparator {
    fn node_to_token(&self, kind: &str) -> Option<EmptyLineSeparatorToken> {
        match kind {
            "package_declaration" => Some(EmptyLineSeparatorToken::PackageDef),
            "import_declaration" => Some(EmptyLineSeparatorToken::Import),
            "class_declaration" => Some(EmptyLineSeparatorToken::ClassDef),
            "interface_declaration" => Some(EmptyLineSeparatorToken::InterfaceDef),
            "enum_declaration" => Some(EmptyLineSeparatorToken::EnumDef),
            "static_initializer" => Some(EmptyLineSeparatorToken::StaticInit),
            "block" => Some(EmptyLineSeparatorToken::InstanceInit), // instance init block
            "method_declaration" => Some(EmptyLineSeparatorToken::MethodDef),
            "constructor_declaration" => Some(EmptyLineSeparatorToken::CtorDef),
            "field_declaration" => Some(EmptyLineSeparatorToken::VariableDef),
            "record_declaration" => Some(EmptyLineSeparatorToken::RecordDef),
            "compact_constructor_declaration" => Some(EmptyLineSeparatorToken::CompactCtorDef),
            _ => None,
        }
    }

    fn find_start_line_before_comments(
        &self,
        children: &[tree_sitter::Node],
        target: &tree_sitter::Node,
        _ctx: &CheckContext,
    ) -> usize {
        // Find comments immediately before this node
        let target_idx = children.iter().position(|c| c.id() == target.id());

        if let Some(idx) = target_idx {
            // Walk backwards to find first comment in sequence before target
            let mut first_comment_line = target.start_position().row;
            for i in (0..idx).rev() {
                let prev = &children[i];
                if prev.kind() == "line_comment" || prev.kind() == "block_comment" {
                    first_comment_line = prev.start_position().row;
                } else if prev.kind() != "{" && prev.kind() != "}" && !prev.is_extra() {
                    break;
                }
            }
            first_comment_line
        } else {
            target.start_position().row
        }
    }
}
```

**Step 2: Run tests**

```bash
cargo test --package lintal_linter empty_line_separator -- --nocapture
```

Expected: Tests pass

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/empty_line_separator.rs
git commit -m "feat(EmptyLineSeparator): implement class member separation checking"
```

---

## Task 3: Register rule and add compatibility tests

**Files:**
- Modify: `crates/lintal_linter/src/registry.rs`
- Modify: `crates/lintal_linter/src/rules/mod.rs`
- Create: `crates/lintal_linter/tests/checkstyle_emptylineseparator.rs`

**Step 1: Export from rules/mod.rs**

Ensure `EmptyLineSeparator` is exported (should already be via `pub use whitespace::*;`)

**Step 2: Register in registry.rs**

Add to imports and register:
```rust
use crate::rules::EmptyLineSeparator;
// ...
self.register::<EmptyLineSeparator>();
```

**Step 3: Create checkstyle compatibility tests**

```rust
//! Checkstyle compatibility tests for EmptyLineSeparator rule.

use std::collections::HashSet;
use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::{CheckContext, Rule};
use lintal_linter::rules::EmptyLineSeparator;

struct ViolationInfo {
    line: usize,
    message: String,
}

fn check_empty_line_separator(source: &str) -> Vec<ViolationInfo> {
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();
    let ctx = CheckContext::new(source);
    let rule = EmptyLineSeparator::default();

    let mut violations = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        for diag in rule.check(&ctx, &node) {
            let line = ctx.source_code().line_column(diag.range().start()).line.get();
            violations.push(ViolationInfo {
                line,
                message: diag.message(),
            });
        }
    }
    violations
}

#[test]
fn test_basic_separation() {
    let source = r#"
class Test {
    void method1() {}
    void method2() {}
}
"#;
    let violations = check_empty_line_separator(source);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].message.contains("METHOD_DEF"));
}

#[test]
fn test_with_blank_line() {
    let source = r#"
class Test {
    void method1() {}

    void method2() {}
}
"#;
    let violations = check_empty_line_separator(source);
    assert!(violations.is_empty());
}
```

**Step 4: Run tests and commit**

```bash
cargo test --package lintal_linter emptylineseparator -- --nocapture
git add -A
git commit -m "feat(EmptyLineSeparator): register rule and add compat tests"
```

---

## Task 4: Final verification and real-world testing

**Step 1: Run full test suite**

```bash
cargo test --all
```

**Step 2: Run clippy**

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

**Step 3: Format and build release**

```bash
cargo fmt --all
cargo build --release
```

**Step 4: Test against real projects**

```bash
./target/release/lintal check target/agrona --config target/agrona/config/checkstyle/checkstyle.xml
./target/release/lintal check target/aeron --config target/aeron/config/checkstyle/checkstyle.xml
./target/release/lintal check target/artio --config target/artio/config/checkstyle/checkstyle.xml
```

Expected: 0 violations (projects are checkstyle-clean)

**Step 5: Update README**

Update README.md to show 29 rules, add EmptyLineSeparator to Whitespace section.

**Step 6: Commit**

```bash
git add -A
git commit -m "docs: add EmptyLineSeparator to README (29 rules)"
```
