# Phase 3: Whitespace Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement 9 whitespace rules with 100% checkstyle compatibility.

**Architecture:** Each rule follows the established pattern: struct with config, `FromConfig` impl, `Rule` impl. Shared helpers extracted to `common.rs`. TDD with checkstyle fixtures as oracle.

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics

---

## Task 1: Create Shared Helpers Module

**Files:**
- Create: `crates/lintal_linter/src/rules/whitespace/common.rs`
- Modify: `crates/lintal_linter/src/rules/whitespace/mod.rs`

**Step 1: Create common.rs with helper functions**

```rust
//! Shared helpers for whitespace rules.

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};

/// Check if character before position is whitespace.
pub fn has_whitespace_before(source: &str, pos: TextSize) -> bool {
    if pos == TextSize::new(0) {
        return true; // Start of file counts as whitespace
    }
    let idx = usize::from(pos);
    source[..idx]
        .chars()
        .last()
        .is_some_and(|c| c.is_whitespace())
}

/// Check if character after position is whitespace.
pub fn has_whitespace_after(source: &str, pos: TextSize) -> bool {
    let idx = usize::from(pos);
    source[idx..]
        .chars()
        .next()
        .is_some_and(|c| c.is_whitespace())
}

/// Get the character before a position.
pub fn char_before(source: &str, pos: TextSize) -> Option<char> {
    if pos == TextSize::new(0) {
        return None;
    }
    let idx = usize::from(pos);
    source[..idx].chars().last()
}

/// Get the character after a position.
pub fn char_after(source: &str, pos: TextSize) -> Option<char> {
    let idx = usize::from(pos);
    source[idx..].chars().next()
}

/// Check if character before position is a newline.
pub fn has_newline_before(source: &str, pos: TextSize) -> bool {
    char_before(source, pos).is_some_and(|c| c == '\n')
}

/// Check if character after position is a newline.
pub fn has_newline_after(source: &str, pos: TextSize) -> bool {
    char_after(source, pos).is_some_and(|c| c == '\n')
}

/// Find the range of whitespace before a position.
/// Returns None if no whitespace before.
pub fn whitespace_range_before(source: &str, pos: TextSize) -> Option<TextRange> {
    let idx = usize::from(pos);
    let before = &source[..idx];

    let ws_len = before.chars().rev().take_while(|c| c.is_whitespace()).count();
    if ws_len == 0 {
        return None;
    }

    // Count bytes, not chars
    let ws_bytes: usize = before.chars().rev().take(ws_len).map(|c| c.len_utf8()).sum();
    let start = TextSize::new((idx - ws_bytes) as u32);
    Some(TextRange::new(start, pos))
}

/// Find the range of whitespace after a position.
/// Returns None if no whitespace after.
pub fn whitespace_range_after(source: &str, pos: TextSize) -> Option<TextRange> {
    let idx = usize::from(pos);
    let after = &source[idx..];

    let ws_len = after.chars().take_while(|c| c.is_whitespace()).count();
    if ws_len == 0 {
        return None;
    }

    // Count bytes, not chars
    let ws_bytes: usize = after.chars().take(ws_len).map(|c| c.len_utf8()).sum();
    let end = TextSize::new((idx + ws_bytes) as u32);
    Some(TextRange::new(pos, end))
}

// ============================================================================
// Violation types shared across whitespace rules
// ============================================================================

/// Violation: token is not followed by whitespace.
#[derive(Debug, Clone)]
pub struct NotFollowed {
    pub token: String,
}

impl Violation for NotFollowed {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is not followed by whitespace", self.token)
    }
}

/// Violation: token is not preceded by whitespace.
#[derive(Debug, Clone)]
pub struct NotPreceded {
    pub token: String,
}

impl Violation for NotPreceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is not preceded by whitespace", self.token)
    }
}

/// Violation: token is followed by whitespace (when it shouldn't be).
#[derive(Debug, Clone)]
pub struct Followed {
    pub token: String,
}

impl Violation for Followed {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is followed by whitespace", self.token)
    }
}

/// Violation: token is preceded by whitespace (when it shouldn't be).
#[derive(Debug, Clone)]
pub struct Preceded {
    pub token: String,
}

impl Violation for Preceded {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("'{}' is preceded by whitespace", self.token)
    }
}

// ============================================================================
// Diagnostic builders
// ============================================================================

/// Create diagnostic for missing whitespace after token.
pub fn diag_not_followed(token: &CstNode) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(NotFollowed { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), range.end())))
}

/// Create diagnostic for missing whitespace before token.
pub fn diag_not_preceded(token: &CstNode) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(NotPreceded { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::insertion(" ".to_string(), range.start())))
}

/// Create diagnostic for unexpected whitespace after token.
pub fn diag_followed(token: &CstNode, ws_range: TextRange) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(Followed { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::deletion(ws_range)))
}

/// Create diagnostic for unexpected whitespace before token.
pub fn diag_preceded(token: &CstNode, ws_range: TextRange) -> Diagnostic {
    let range = token.range();
    let text = token.text().to_string();
    Diagnostic::new(Preceded { token: text }, range)
        .with_fix(Fix::safe_edit(Edit::deletion(ws_range)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_whitespace_before() {
        assert!(has_whitespace_before("a b", TextSize::new(2)));
        assert!(!has_whitespace_before("ab", TextSize::new(1)));
        assert!(has_whitespace_before("a", TextSize::new(0))); // start of file
    }

    #[test]
    fn test_has_whitespace_after() {
        assert!(has_whitespace_after("a b", TextSize::new(1)));
        assert!(!has_whitespace_after("ab", TextSize::new(1)));
    }

    #[test]
    fn test_whitespace_range_before() {
        let range = whitespace_range_before("a  b", TextSize::new(3));
        assert!(range.is_some());
        let r = range.unwrap();
        assert_eq!(r.start(), TextSize::new(1));
        assert_eq!(r.end(), TextSize::new(3));
    }

    #[test]
    fn test_whitespace_range_after() {
        let range = whitespace_range_after("a  b", TextSize::new(1));
        assert!(range.is_some());
        let r = range.unwrap();
        assert_eq!(r.start(), TextSize::new(1));
        assert_eq!(r.end(), TextSize::new(3));
    }
}
```

**Step 2: Update mod.rs to include common module**

In `crates/lintal_linter/src/rules/whitespace/mod.rs`:

```rust
//! Whitespace-related rules.

pub mod common;
mod whitespace_around;

pub use whitespace_around::WhitespaceAround;
```

**Step 3: Run tests**

```bash
cargo test --package lintal_linter
```

Expected: All existing tests pass + new common module tests pass.

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/common.rs
git add crates/lintal_linter/src/rules/whitespace/mod.rs
git commit -m "feat(whitespace): add shared helpers module for whitespace rules"
```

---

## Task 2: WhitespaceAfter - Compatibility Test Setup

**Files:**
- Create: `crates/lintal_linter/tests/checkstyle_whitespaceafter.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_repo.rs`

**Step 1: Update checkstyle_repo.rs helper**

Add a more flexible path builder in `crates/lintal_linter/tests/checkstyle_repo.rs`:

```rust
/// Get path to a checkstyle test input file for any whitespace check.
pub fn whitespace_test_input(check_name: &str, file_name: &str) -> Option<PathBuf> {
    let repo = checkstyle_repo()?;
    let path = repo
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/whitespace")
        .join(check_name.to_lowercase())
        .join(file_name);

    if path.exists() { Some(path) } else { None }
}
```

**Step 2: Create WhitespaceAfter compatibility test file**

Create `crates/lintal_linter/tests/checkstyle_whitespaceafter.rs`:

```rust
//! WhitespaceAfter checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashSet;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Violation {
    line: usize,
    token: String,
}

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("whitespaceafter", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Extract expected violations from inline comments in test file.
/// Format: // violation '',' is not followed by whitespace'
fn extract_expected_violations(source: &str) -> Vec<(usize, String)> {
    let mut violations = vec![];
    for (line_num, line) in source.lines().enumerate() {
        if let Some(comment_start) = line.find("// violation") {
            let comment = &line[comment_start..];
            // Extract token from pattern: ''X' is not followed'
            if let Some(start) = comment.find("''") {
                let after_quote = &comment[start + 2..];
                if let Some(end) = after_quote.find("'") {
                    let token = after_quote[..end].to_string();
                    violations.push((line_num + 1, token)); // 1-indexed
                }
            }
        }
    }
    violations
}

// =============================================================================
// Test: testDefaultConfig
// File: InputWhitespaceAfterDefaultConfig.java
// =============================================================================

#[test]
fn test_whitespace_after_default_config() {
    let Some(source) = load_fixture("InputWhitespaceAfterDefaultConfig.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let expected = extract_expected_violations(&source);
    println!("Expected violations: {:?}", expected);

    // TODO: Implement WhitespaceAfter rule and uncomment
    // let violations = check_whitespace_after(&source);
    // verify_violations(&violations, &expected);

    // For now, just verify we can parse expected violations
    assert!(!expected.is_empty(), "Should find expected violations in comments");
}
```

**Step 3: Run test to verify setup**

```bash
cargo test --package lintal_linter --test checkstyle_whitespaceafter
```

Expected: Test runs, finds expected violations in comments, passes (placeholder).

**Step 4: Commit**

```bash
git add crates/lintal_linter/tests/checkstyle_repo.rs
git add crates/lintal_linter/tests/checkstyle_whitespaceafter.rs
git commit -m "test(whitespace): add WhitespaceAfter compatibility test scaffold"
```

---

## Task 3: WhitespaceAfter - Rule Implementation

**Files:**
- Create: `crates/lintal_linter/src/rules/whitespace/whitespace_after.rs`
- Modify: `crates/lintal_linter/src/rules/whitespace/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`

**Step 1: Create the rule implementation**

Create `crates/lintal_linter/src/rules/whitespace/whitespace_after.rs`:

```rust
//! WhitespaceAfter rule implementation.
//!
//! Checks that a token is followed by whitespace.
//! Checkstyle equivalent: WhitespaceAfter

use std::collections::HashSet;

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::rules::whitespace::common::{diag_not_followed, has_whitespace_after};
use crate::{CheckContext, FromConfig, Properties, Rule};

/// Tokens that can be checked by WhitespaceAfter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WhitespaceAfterToken {
    Comma,
    Semi,
    Typecast,
    LiteralIf,
    LiteralElse,
    LiteralWhile,
    LiteralDo,
    LiteralFor,
    DoWhile,
}

impl WhitespaceAfterToken {
    fn from_str(s: &str) -> Option<Self> {
        match s.trim() {
            "COMMA" => Some(Self::Comma),
            "SEMI" => Some(Self::Semi),
            "TYPECAST" => Some(Self::Typecast),
            "LITERAL_IF" => Some(Self::LiteralIf),
            "LITERAL_ELSE" => Some(Self::LiteralElse),
            "LITERAL_WHILE" => Some(Self::LiteralWhile),
            "LITERAL_DO" => Some(Self::LiteralDo),
            "LITERAL_FOR" => Some(Self::LiteralFor),
            "DO_WHILE" => Some(Self::DoWhile),
            _ => None,
        }
    }
}

/// Configuration for WhitespaceAfter rule.
#[derive(Debug, Clone)]
pub struct WhitespaceAfter {
    /// Which tokens to check.
    pub tokens: HashSet<WhitespaceAfterToken>,
}

impl Default for WhitespaceAfter {
    fn default() -> Self {
        let mut tokens = HashSet::new();
        tokens.insert(WhitespaceAfterToken::Comma);
        tokens.insert(WhitespaceAfterToken::Semi);
        Self { tokens }
    }
}

impl FromConfig for WhitespaceAfter {
    const MODULE_NAME: &'static str = "WhitespaceAfter";

    fn from_config(properties: &Properties) -> Self {
        let tokens_str = properties.get("tokens").copied().unwrap_or("COMMA, SEMI");
        let tokens: HashSet<_> = tokens_str
            .split(',')
            .filter_map(WhitespaceAfterToken::from_str)
            .collect();

        Self {
            tokens: if tokens.is_empty() {
                Self::default().tokens
            } else {
                tokens
            },
        }
    }
}

impl Rule for WhitespaceAfter {
    fn name(&self) -> &'static str {
        "WhitespaceAfter"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let mut diagnostics = vec![];

        match node.kind() {
            // Comma: array initializers, parameter lists, etc.
            "," if self.tokens.contains(&WhitespaceAfterToken::Comma) => {
                if !is_followed_by_whitespace_or_valid(ctx, node) {
                    diagnostics.push(diag_not_followed(node));
                }
            }

            // Semicolon: statement terminator, for loop parts
            ";" if self.tokens.contains(&WhitespaceAfterToken::Semi) => {
                // Skip semicolons at end of line or end of for loop
                if !is_semicolon_exempt(ctx, node) && !has_whitespace_after(ctx.source(), node.range().end()) {
                    diagnostics.push(diag_not_followed(node));
                }
            }

            // Cast expression: (Type) value
            "cast_expression" if self.tokens.contains(&WhitespaceAfterToken::Typecast) => {
                // Find the closing paren of the typecast
                if let Some(rparen) = node.children().find(|c| c.kind() == ")") {
                    if !has_whitespace_after(ctx.source(), rparen.range().end()) {
                        diagnostics.push(diag_not_followed(&rparen));
                    }
                }
            }

            // if keyword
            "if_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralIf) => {
                if let Some(kw) = find_keyword(node, "if") {
                    if !has_whitespace_after(ctx.source(), kw.range().end()) {
                        diagnostics.push(diag_not_followed(&kw));
                    }
                }
            }

            // else keyword
            "if_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralElse) => {
                if let Some(kw) = find_keyword(node, "else") {
                    if !has_whitespace_after(ctx.source(), kw.range().end()) {
                        diagnostics.push(diag_not_followed(&kw));
                    }
                }
            }

            // while keyword (in while statement)
            "while_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralWhile) => {
                if let Some(kw) = find_keyword(node, "while") {
                    if !has_whitespace_after(ctx.source(), kw.range().end()) {
                        diagnostics.push(diag_not_followed(&kw));
                    }
                }
            }

            // do keyword
            "do_statement" if self.tokens.contains(&WhitespaceAfterToken::LiteralDo) => {
                if let Some(kw) = find_keyword(node, "do") {
                    if !has_whitespace_after(ctx.source(), kw.range().end()) {
                        diagnostics.push(diag_not_followed(&kw));
                    }
                }
                // Also check while in do-while (DO_WHILE token)
                if self.tokens.contains(&WhitespaceAfterToken::DoWhile) {
                    if let Some(kw) = find_keyword(node, "while") {
                        if !has_whitespace_after(ctx.source(), kw.range().end()) {
                            diagnostics.push(diag_not_followed(&kw));
                        }
                    }
                }
            }

            // for keyword
            "for_statement" | "enhanced_for_statement"
                if self.tokens.contains(&WhitespaceAfterToken::LiteralFor) =>
            {
                if let Some(kw) = find_keyword(node, "for") {
                    if !has_whitespace_after(ctx.source(), kw.range().end()) {
                        diagnostics.push(diag_not_followed(&kw));
                    }
                }
            }

            _ => {}
        }

        diagnostics
    }
}

/// Find a keyword in node's children.
fn find_keyword<'a>(node: &CstNode<'a>, keyword: &str) -> Option<CstNode<'a>> {
    node.children().find(|c| c.kind() == keyword)
}

/// Check if comma is followed by whitespace or valid non-whitespace (like closing bracket).
fn is_followed_by_whitespace_or_valid(ctx: &CheckContext, node: &CstNode) -> bool {
    let after_pos = node.range().end();
    let source = ctx.source();

    if let Some(c) = source[usize::from(after_pos)..].chars().next() {
        // Whitespace is always OK
        if c.is_whitespace() {
            return true;
        }
        // Closing brackets/parens are OK after comma (empty trailing)
        if matches!(c, ')' | ']' | '}') {
            return true;
        }
    }
    false
}

/// Check if semicolon is exempt from whitespace-after check.
fn is_semicolon_exempt(ctx: &CheckContext, node: &CstNode) -> bool {
    let after_pos = node.range().end();
    let source = ctx.source();

    // Check what follows
    if let Some(c) = source[usize::from(after_pos)..].chars().next() {
        // End of line is OK
        if c == '\n' || c == '\r' {
            return true;
        }
        // Closing paren is OK (for loop)
        if c == ')' {
            return true;
        }
        // Another semicolon is OK (empty for parts)
        if c == ';' {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, &WhitespaceAfter::default())
    }

    fn check_source_with_config(source: &str, rule: &WhitespaceAfter) -> Vec<Diagnostic> {
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
    fn test_comma_without_space() {
        let diagnostics = check_source("class Foo { int[] a = {1,2}; }");
        assert!(
            diagnostics.iter().any(|d| d.kind.body.contains(",") && d.kind.body.contains("not followed")),
            "Should detect comma without space: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_comma_with_space() {
        let diagnostics = check_source("class Foo { int[] a = {1, 2}; }");
        let comma_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(","))
            .collect();
        assert!(comma_violations.is_empty(), "Should not flag comma with space");
    }

    #[test]
    fn test_semicolon_end_of_line() {
        let diagnostics = check_source("class Foo { int x = 1;\n}");
        let semi_violations: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.kind.body.contains(";"))
            .collect();
        assert!(semi_violations.is_empty(), "Should not flag semicolon at EOL");
    }

    #[test]
    fn test_for_loop_semicolon() {
        let diagnostics = check_source("class Foo { void m() { for (int i = 0;i < 10; i++) {} } }");
        assert!(
            diagnostics.iter().any(|d| d.kind.body.contains(";") && d.kind.body.contains("not followed")),
            "Should detect semicolon without space in for loop"
        );
    }

    #[test]
    fn test_all_diagnostics_have_fixes() {
        let diagnostics = check_source("class Foo { int[] a = {1,2,3}; }");
        for d in &diagnostics {
            assert!(d.fix.is_some(), "Diagnostic should have fix: {}", d.kind.body);
        }
    }
}
```

**Step 2: Update mod.rs to export the rule**

```rust
//! Whitespace-related rules.

pub mod common;
mod whitespace_after;
mod whitespace_around;

pub use whitespace_after::WhitespaceAfter;
pub use whitespace_around::WhitespaceAround;
```

**Step 3: Register the rule**

In `crates/lintal_linter/src/registry.rs`, update `register_builtins`:

```rust
fn register_builtins(&mut self) {
    use crate::rules::{WhitespaceAfter, WhitespaceAround};
    self.register::<WhitespaceAround>();
    self.register::<WhitespaceAfter>();
}
```

**Step 4: Run unit tests**

```bash
cargo test --package lintal_linter whitespace_after
```

Expected: Unit tests pass.

**Step 5: Commit**

```bash
git add crates/lintal_linter/src/rules/whitespace/whitespace_after.rs
git add crates/lintal_linter/src/rules/whitespace/mod.rs
git add crates/lintal_linter/src/registry.rs
git commit -m "feat(whitespace): implement WhitespaceAfter rule"
```

---

## Task 4: WhitespaceAfter - Compatibility Tests

**Files:**
- Modify: `crates/lintal_linter/tests/checkstyle_whitespaceafter.rs`

**Step 1: Complete the compatibility test**

Update `crates/lintal_linter/tests/checkstyle_whitespaceafter.rs` to use the real rule:

```rust
//! WhitespaceAfter checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::WhitespaceAfter;
use lintal_linter::{CheckContext, FromConfig, Properties, Rule};
use lintal_source_file::{LineIndex, SourceCode};
use std::collections::HashMap;

/// A violation at a specific location.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    column: usize,
    token: String,
}

/// Load a checkstyle whitespace test input file.
fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::whitespace_test_input("whitespaceafter", file_name)?;
    std::fs::read_to_string(&path).ok()
}

/// Run WhitespaceAfter rule on source.
fn check_whitespace_after(source: &str) -> Vec<Violation> {
    check_whitespace_after_with_config(source, &HashMap::new())
}

/// Run WhitespaceAfter with config properties.
fn check_whitespace_after_with_config(source: &str, props: &Properties) -> Vec<Violation> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = WhitespaceAfter::from_config(props);
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];
    for node in TreeWalker::new(result.tree.root_node(), source) {
        for diagnostic in rule.check(&ctx, &node) {
            let loc = source_code.line_column(diagnostic.range.start());
            let token = extract_token_from_message(&diagnostic.kind.body);
            violations.push(Violation {
                line: loc.line.get(),
                column: loc.column.get(),
                token,
            });
        }
    }
    violations
}

/// Extract token from message like "',' is not followed by whitespace"
fn extract_token_from_message(message: &str) -> String {
    if let Some(start) = message.find('\'') {
        let after = &message[start + 1..];
        if let Some(end) = after.find('\'') {
            return after[..end].to_string();
        }
    }
    message.to_string()
}

/// Print violations for debugging.
fn print_violations(label: &str, violations: &[Violation]) {
    println!("\n{}:", label);
    for v in violations {
        println!("  {}:{}: '{}'", v.line, v.column, v.token);
    }
}

// =============================================================================
// Test: testDefaultConfig
// =============================================================================

#[test]
fn test_whitespace_after_default_config() {
    let Some(source) = load_fixture("InputWhitespaceAfterDefaultConfig.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_whitespace_after(&source);
    print_violations("Actual violations", &violations);

    // Expected from checkstyle: line 45 comma, line 74 comma
    let comma_violations: Vec<_> = violations.iter().filter(|v| v.token == ",").collect();

    assert!(
        comma_violations.iter().any(|v| v.line == 45),
        "Should detect comma violation on line 45"
    );
    assert!(
        comma_violations.iter().any(|v| v.line == 74),
        "Should detect comma violation on line 74"
    );
}

// Additional tests for other fixtures...
```

**Step 2: Run compatibility tests**

```bash
cargo test --package lintal_linter --test checkstyle_whitespaceafter
```

Expected: Tests pass, matching checkstyle behavior.

**Step 3: Iterate on rule until all tests pass**

If tests fail, update `whitespace_after.rs` to match checkstyle behavior, then re-run.

**Step 4: Commit**

```bash
git add crates/lintal_linter/tests/checkstyle_whitespaceafter.rs
git commit -m "test(whitespace): add WhitespaceAfter compatibility tests"
```

---

## Task 5-12: Remaining Rules

Follow the same pattern for each remaining rule:

| Task | Rule | Key Patterns |
|------|------|--------------|
| 5-6 | ParenPad | Check inside `(` and `)`, option=space/nospace |
| 7-8 | NoWhitespaceAfter | Inverse of WhitespaceAfter, allowLineBreaks |
| 9-10 | SingleSpaceSeparator | Line-level scan, multiple spaces → one |
| 11-12 | MethodParamPad | Before `(` in method/ctor declarations |
| 13-14 | NoWhitespaceBefore | Inverse logic, allowLineBreaks |
| 15-16 | EmptyForInitializerPad | Only empty for initializers |
| 17-18 | TypecastParenPad | ParenPad scoped to cast_expression |
| 19-20 | FileTabCharacter | Text scan, no tree-sitter |

Each rule follows: **Test scaffold → Implementation → Compatibility tests → Iterate → Commit**

---

## Task 21: Final Integration

**Files:**
- Modify: `crates/lintal_linter/src/rules/whitespace/mod.rs`
- Verify: All rules registered

**Step 1: Verify all exports**

```rust
//! Whitespace-related rules.

pub mod common;
mod empty_for_initializer_pad;
mod file_tab_character;
mod method_param_pad;
mod no_whitespace_after;
mod no_whitespace_before;
mod paren_pad;
mod single_space_separator;
mod typecast_paren_pad;
mod whitespace_after;
mod whitespace_around;

pub use empty_for_initializer_pad::EmptyForInitializerPad;
pub use file_tab_character::FileTabCharacter;
pub use method_param_pad::MethodParamPad;
pub use no_whitespace_after::NoWhitespaceAfter;
pub use no_whitespace_before::NoWhitespaceBefore;
pub use paren_pad::ParenPad;
pub use single_space_separator::SingleSpaceSeparator;
pub use typecast_paren_pad::TypecastParenPad;
pub use whitespace_after::WhitespaceAfter;
pub use whitespace_around::WhitespaceAround;
```

**Step 2: Run full test suite**

```bash
cargo test --all
```

Expected: All tests pass.

**Step 3: Run clippy and format**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

**Step 4: Final commit**

```bash
git add -A
git commit -m "feat(whitespace): complete Phase 3 - all 9 whitespace rules implemented"
```

---

## Summary

| Task | Description | Estimated Tests |
|------|-------------|-----------------|
| 1 | Shared helpers module | 4 |
| 2-4 | WhitespaceAfter | 29 |
| 5-6 | ParenPad | 27 |
| 7-8 | NoWhitespaceAfter | 19 |
| 9-10 | SingleSpaceSeparator | 11 |
| 11-12 | MethodParamPad | 11 |
| 13-14 | NoWhitespaceBefore | 10 |
| 15-16 | EmptyForInitializerPad | 5 |
| 17-18 | TypecastParenPad | 3 |
| 19-20 | FileTabCharacter | 3 |
| 21 | Final integration | - |
| **Total** | | **~122** |
