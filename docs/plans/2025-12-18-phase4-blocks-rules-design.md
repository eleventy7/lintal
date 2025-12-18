# Phase 4: Blocks Rules Design

Implementation plan for 6 blocks rules with 100% checkstyle compatibility.

## Implementation Order

Based on checkstyle test coverage (more tests = higher confidence):

| Priority | Rule | Test Files | Complexity | Notes |
|----------|------|------------|------------|-------|
| 1 | RightCurly | 63 | High | 3 options, multi-block handling |
| 2 | LeftCurly | 31 | Medium | 3 options, ignoreEnums flag |
| 3 | NeedBraces | 19 | Medium | Single-line detection |
| 4 | EmptyBlock | 16 | Medium | 2 options (STATEMENT/TEXT) |
| 5 | EmptyCatchBlock | 3 | Low | Regex matching |
| 6 | AvoidNestedBlocks | 2 | Low | SLIST inside SLIST |

## Architecture

### File Structure

```
crates/lintal_linter/src/rules/
├── mod.rs                    # Add blocks export
├── whitespace/               # Existing (Phase 2-3)
└── blocks/                   # NEW
    ├── mod.rs
    ├── common.rs             # Shared helpers
    ├── right_curly.rs
    ├── left_curly.rs
    ├── need_braces.rs
    ├── empty_block.rs
    ├── empty_catch_block.rs
    └── avoid_nested_blocks.rs
```

### Shared Helpers Module

Create `rules/blocks/common.rs` with generic helpers:

```rust
/// Check if token is alone on its line (only whitespace before it)
pub fn is_alone_on_line(source: &str, node: &CstNode) -> bool

/// Check if two nodes are on the same line
pub fn are_on_same_line(a: &CstNode, b: &CstNode) -> bool

/// Get the next token after a node (skipping whitespace/comments)
pub fn get_next_token(node: &CstNode) -> Option<CstNode>

/// Check if there's a line break between two positions
pub fn has_line_break_between(source: &str, start: TextSize, end: TextSize) -> bool

/// Find matching right curly for a block
pub fn find_right_curly(block: &CstNode) -> Option<CstNode>

/// Find matching left curly for a block
pub fn find_left_curly(block: &CstNode) -> Option<CstNode>
```

## Rule Specifications

### 1. RightCurly

**Purpose**: Checks placement of right curly braces (`}`).

**Config options**:
- `option` - `same` (default), `alone`, or `alone_or_singleline`
- `tokens` - which constructs to check

**Options explained**:
- `same` - `}` must be on same line as next part (else, catch, finally) OR alone if last
- `alone` - `}` must always be alone on line
- `alone_or_singleline` - `}` alone OR entire block on single line allowed

**Tokens**: LITERAL_TRY, LITERAL_CATCH, LITERAL_FINALLY, LITERAL_IF, LITERAL_ELSE, CLASS_DEF, METHOD_DEF, CTOR_DEF, LITERAL_FOR, LITERAL_WHILE, LITERAL_DO, STATIC_INIT, INSTANCE_INIT, ANNOTATION_DEF, ENUM_DEF, INTERFACE_DEF, RECORD_DEF, COMPACT_CTOR_DEF, LITERAL_SWITCH, LITERAL_CASE

**Violations**:
- `line.break.before` - "'}' at column N should have line break before"
- `line.same` - "'}' at column N should be on the same line as the next part"
- `line.alone` - "'}' at column N should be alone on a line"

### 2. LeftCurly

**Purpose**: Checks placement of left curly braces (`{`).

**Config options**:
- `option` - `eol` (default), `nl`, or `nlow`
- `ignoreEnums` - ignore enums when option is EOL (default: true)

**Options explained**:
- `eol` - `{` at end of line
- `nl` - `{` on new line
- `nlow` - `{` on new line if it won't fit

**Violations**:
- `line.new` - "'{' at column N should be on a new line"
- `line.previous` - "'{' at column N should be on the previous line"
- `line.break.after` - "'{' at column N should have line break after"

### 3. NeedBraces

**Purpose**: Checks that code blocks have braces around them.

**Config options**:
- `allowSingleLineStatement` - allow single-line statements without braces (default: false)
- `allowEmptyLoopBody` - allow loops with empty bodies (default: false)
- `tokens` - which constructs to check

**Tokens**: LITERAL_DO, LITERAL_ELSE, LITERAL_FOR, LITERAL_IF, LITERAL_WHILE, LITERAL_CASE, LITERAL_DEFAULT, LAMBDA

**Violations**:
- `needBraces` - "'X' construct must use '{}'s"

### 4. EmptyBlock

**Purpose**: Checks for empty blocks.

**Config options**:
- `option` - `statement` (default) or `text`
- `tokens` - which constructs to check

**Options explained**:
- `statement` - block must have at least one statement
- `text` - block must have any text (including comments)

**Tokens**: LITERAL_WHILE, LITERAL_TRY, LITERAL_CATCH, LITERAL_FINALLY, LITERAL_DO, LITERAL_IF, LITERAL_ELSE, LITERAL_FOR, INSTANCE_INIT, STATIC_INIT, LITERAL_SWITCH, LITERAL_SYNCHRONIZED, LITERAL_CASE, LITERAL_DEFAULT, ARRAY_INIT

**Violations**:
- `block.noStatement` - "Must have at least one statement"
- `block.empty` - "Empty X block"

### 5. EmptyCatchBlock

**Purpose**: Checks for empty catch blocks with configurable exceptions.

**Config options**:
- `exceptionVariableName` - regex for variable name (default: ^$ - match nothing)
- `commentFormat` - regex for comment content (default: .* - match anything)

If variable name OR comment matches regex, empty catch is allowed.

**Violations**:
- `catch.block.empty` - "Empty catch block"

### 6. AvoidNestedBlocks

**Purpose**: Detects unnecessary nested blocks.

**Config options**:
- `allowInSwitchCase` - allow nested blocks in switch cases (default: false)

**Violations**:
- `block.nested` - "Avoid nested blocks"

## Testing Strategy

### Test Structure

```
tests/
├── checkstyle_rightcurly.rs        # 63 test cases
├── checkstyle_leftcurly.rs         # 31 test cases
├── checkstyle_needbraces.rs        # 19 test cases
├── checkstyle_emptyblock.rs        # 16 test cases
├── checkstyle_emptycatchblock.rs   # 3 test cases
├── checkstyle_avoidnestedblocks.rs # 2 test cases
└── checkstyle_repo.rs              # Shared (already exists)
```

### Test Development Flow (TDD)

1. Study checkstyle test class (e.g., `RightCurlyCheckTest.java`)
2. Extract expected violations from test methods
3. Write compatibility test loading fixture + asserting violations
4. Run test → fails (RED)
5. Implement rule → pass (GREEN)
6. Refactor if needed

### Definition of Done (per rule)

- [ ] Rule struct with config options
- [ ] `FromConfig` implementation
- [ ] `Rule` implementation
- [ ] All checkstyle test fixtures passing
- [ ] Registered in registry
- [ ] Unit tests for edge cases

## Deliverables Summary

| Deliverable | Count |
|-------------|-------|
| New rule modules | 6 |
| Shared helpers module | 1 |
| Compatibility test files | 6 |
| Estimated test cases | ~134 |
