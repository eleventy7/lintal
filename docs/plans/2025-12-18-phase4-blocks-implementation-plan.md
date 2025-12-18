# Phase 4: Blocks Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement 6 blocks rules with 100% checkstyle compatibility.

**Architecture:** Create `rules/blocks/` module with shared helpers in `common.rs`. Each rule follows the existing pattern: `FromConfig` + `Rule` traits. Use checkstyle test fixtures as oracle for TDD.

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics, checkstyle fixtures

---

## Task 1: Create blocks module structure and shared helpers

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Create: `crates/lintal_linter/src/rules/blocks/common.rs`
- Modify: `crates/lintal_linter/src/rules/mod.rs`

**Step 1: Create the blocks module directory and mod.rs**

```rust
// crates/lintal_linter/src/rules/blocks/mod.rs
//! Blocks rules for checking brace placement and block structure.

pub mod common;

// Rules will be added as they're implemented
```

**Step 2: Create common.rs with shared helpers**

```rust
// crates/lintal_linter/src/rules/blocks/common.rs
//! Shared helpers for blocks rules.

use lintal_java_cst::CstNode;
use lintal_source_file::{LineIndex, SourceCode};
use lintal_text_size::TextSize;

/// Check if two nodes are on the same line.
pub fn are_on_same_line(source: &str, a: &CstNode, b: &CstNode) -> bool {
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);
    let a_line = source_code.line_column(a.range().start()).line;
    let b_line = source_code.line_column(b.range().start()).line;
    a_line == b_line
}

/// Check if a node is alone on its line (only whitespace before it).
pub fn is_alone_on_line(source: &str, node: &CstNode) -> bool {
    let line_index = LineIndex::from_source_text(source);
    let line_start = line_index.line_start(
        SourceCode::new(source, &line_index).line_column(node.range().start()).line,
        source,
    );
    let before = &source[usize::from(line_start)..usize::from(node.range().start())];
    before.chars().all(|c| c.is_whitespace())
}

/// Check if there's a line break before a position.
pub fn has_line_break_before(source: &str, pos: TextSize) -> bool {
    let before = &source[..usize::from(pos)];
    before.chars().rev().take_while(|c| *c != '\n').all(|c| c.is_whitespace())
        && before.contains('\n')
}

/// Get column number (1-indexed) for a node.
pub fn get_column(source: &str, node: &CstNode) -> usize {
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);
    source_code.line_column(node.range().start()).column.get()
}

/// Find the next sibling node, skipping comments and whitespace.
pub fn get_next_sibling(node: &CstNode) -> Option<CstNode> {
    let mut current = node.next_sibling();
    while let Some(sibling) = current {
        if !matches!(sibling.kind(), "line_comment" | "block_comment") {
            return Some(sibling);
        }
        current = sibling.next_sibling();
    }
    None
}
```

**Step 3: Update rules/mod.rs to export blocks**

Add to `crates/lintal_linter/src/rules/mod.rs`:
```rust
pub mod blocks;
```

**Step 4: Verify it compiles**

Run: `cargo check --package lintal_linter`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add crates/lintal_linter/src/rules/blocks/
git add crates/lintal_linter/src/rules/mod.rs
git commit -m "feat(blocks): add blocks module structure and shared helpers"
```

---

## Task 2: Implement RightCurly rule - basic structure and SAME option

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/right_curly.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_rightcurly.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/RightCurlyCheck.java` and `/Users/shaunlaurens/src/checkstyle/src/test/java/com/puppycrawl/tools/checkstyle/checks/blocks/RightCurlyCheckTest.java`

**Step 1: Create test file with first few test cases**

Look at RightCurlyCheckTest.java to find test methods and expected violations. Start with basic SAME option tests.

**Step 2: Implement RightCurly rule struct and FromConfig**

```rust
// Key structures:
pub enum RightCurlyOption {
    Same,
    Alone,
    AloneOrSingleline,
}

pub struct RightCurly {
    option: RightCurlyOption,
    tokens: HashSet<RightCurlyToken>,
}
```

**Step 3: Implement Rule trait with SAME option logic**

The SAME option requires:
- `}` should be on same line as next part of multi-block (else, catch, finally)
- OR alone if it's the last part
- Line break before `}` if not on same line as `{`

**Step 4: Register in registry.rs**

**Step 5: Run tests, iterate until passing**

**Step 6: Commit**

---

## Task 3: Implement RightCurly ALONE and ALONE_OR_SINGLELINE options

**Files:**
- Modify: `crates/lintal_linter/src/rules/blocks/right_curly.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_rightcurly.rs`

**Step 1: Add more test cases for ALONE option**

**Step 2: Implement ALONE option logic**
- `}` must always be alone on its line

**Step 3: Add test cases for ALONE_OR_SINGLELINE**

**Step 4: Implement ALONE_OR_SINGLELINE logic**
- `}` alone on line OR entire block on single line

**Step 5: Run all RightCurly tests**

**Step 6: Commit**

---

## Task 4: Implement LeftCurly rule

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/left_curly.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_leftcurly.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/LeftCurlyCheck.java`

**Step 1: Create test file from LeftCurlyCheckTest.java**

**Step 2: Implement LeftCurly rule**

```rust
pub enum LeftCurlyOption {
    Eol,   // { at end of line
    Nl,    // { on new line
    Nlow,  // { on new line if won't fit (complex)
}

pub struct LeftCurly {
    option: LeftCurlyOption,
    ignore_enums: bool,
}
```

**Violations:**
- `line.new` - `{` should be on new line (when EOL but found on new line)
- `line.previous` - `{` should be on previous line (when NL but found at EOL)
- `line.break.after` - `{` should have line break after

**Step 3: Register in registry.rs**

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 5: Implement NeedBraces rule

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/need_braces.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_needbraces.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/NeedBracesCheck.java`

**Step 1: Create test file from NeedBracesCheckTest.java**

**Step 2: Implement NeedBraces rule**

```rust
pub struct NeedBraces {
    allow_single_line_statement: bool,
    allow_empty_loop_body: bool,
    tokens: HashSet<NeedBracesToken>,
}
```

Check if statements (if, else, for, while, do) have SLIST children. If not, they're missing braces.

**Step 3: Register in registry.rs**

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 6: Implement EmptyBlock rule

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/empty_block.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_emptyblock.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/EmptyBlockCheck.java`

**Step 1: Create test file from EmptyBlockCheckTest.java**

**Step 2: Implement EmptyBlock rule**

```rust
pub enum BlockOption {
    Statement,  // Must have at least one statement
    Text,       // Must have any text (including comments)
}

pub struct EmptyBlock {
    option: BlockOption,
    tokens: HashSet<EmptyBlockToken>,
}
```

**Step 3: Register in registry.rs**

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 7: Implement EmptyCatchBlock rule

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/empty_catch_block.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_emptycatchblock.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/EmptyCatchBlockCheck.java`

**Step 1: Create test file from EmptyCatchBlockCheckTest.java**

**Step 2: Implement EmptyCatchBlock rule**

```rust
pub struct EmptyCatchBlock {
    exception_variable_name: Regex,  // default: ^$
    comment_format: Regex,           // default: .*
}
```

Check catch blocks - if empty, check if variable name or comment matches regex to suppress.

**Step 3: Register in registry.rs**

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 8: Implement AvoidNestedBlocks rule

**Files:**
- Create: `crates/lintal_linter/src/rules/blocks/avoid_nested_blocks.rs`
- Modify: `crates/lintal_linter/src/rules/blocks/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_avoidnestedblocks.rs`

**Reference:** Study `/Users/shaunlaurens/src/checkstyle/src/main/java/com/puppycrawl/tools/checkstyle/checks/blocks/AvoidNestedBlocksCheck.java`

**Step 1: Create test file from AvoidNestedBlocksCheckTest.java**

**Step 2: Implement AvoidNestedBlocks rule**

```rust
pub struct AvoidNestedBlocks {
    allow_in_switch_case: bool,
}
```

Simple rule: check if SLIST parent is also SLIST (nested block).

**Step 3: Register in registry.rs**

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 9: Update CI and final integration

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add all new test files to CI**

```yaml
- name: Run compatibility tests
  run: |
    cargo test --package lintal_linter --test checkstyle_compat
    cargo test --package lintal_linter --test checkstyle_whitespaceafter
    # ... existing ...
    cargo test --package lintal_linter --test checkstyle_rightcurly
    cargo test --package lintal_linter --test checkstyle_leftcurly
    cargo test --package lintal_linter --test checkstyle_needbraces
    cargo test --package lintal_linter --test checkstyle_emptyblock
    cargo test --package lintal_linter --test checkstyle_emptycatchblock
    cargo test --package lintal_linter --test checkstyle_avoidnestedblocks
```

**Step 2: Run clippy and fix any warnings**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

**Step 3: Run all tests**

Run: `cargo test --package lintal_linter`

**Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add Phase 4 blocks compatibility tests"
```
