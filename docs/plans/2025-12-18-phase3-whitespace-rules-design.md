# Phase 3: Whitespace Rules Design

Implementation plan for 9 remaining whitespace rules with 100% checkstyle compatibility.

## Implementation Order

Based on checkstyle test coverage (more tests = higher confidence):

| Priority | Rule | Test Files | Notes |
|----------|------|------------|-------|
| 1 | WhitespaceAfter | 29 | Complements WhitespaceAround |
| 2 | ParenPad | 27 | Foundation for other pad rules |
| 3 | NoWhitespaceAfter | 19 | Inverse of WhitespaceAfter |
| 4 | SingleSpaceSeparator | 11 | Standalone text-scan rule |
| 5 | MethodParamPad | 11 | Builds on ParenPad patterns |
| 6 | NoWhitespaceBefore | 10 | Inverse logic |
| 7 | EmptyForInitializerPad | 5 | Narrow scope, uses ParenPad patterns |
| 8 | TypecastParenPad | 3 | Narrow scope, uses ParenPad patterns |
| 9 | FileTabCharacter | 3 | Simplest, character replacement |

## Architecture

### Shared Helpers Module

Create `rules/whitespace/common.rs` with generic helpers:

```rust
/// Check if character before position is whitespace
pub fn has_whitespace_before(source: &str, pos: TextSize) -> bool

/// Check if character after position is whitespace
pub fn has_whitespace_after(source: &str, pos: TextSize) -> bool

/// Get the character before a position (if any)
pub fn char_before(source: &str, pos: TextSize) -> Option<char>

/// Get the character after a position (if any)
pub fn char_after(source: &str, pos: TextSize) -> Option<char>

/// Create a "missing whitespace before" diagnostic with fix
pub fn missing_space_before(token: &CstNode, ctx: &CheckContext) -> Diagnostic

/// Create a "missing whitespace after" diagnostic with fix
pub fn missing_space_after(token: &CstNode, ctx: &CheckContext) -> Diagnostic

/// Create an "unexpected whitespace before" diagnostic with fix
pub fn unexpected_space_before(token: &CstNode, ws_range: TextRange, ctx: &CheckContext) -> Diagnostic

/// Create an "unexpected whitespace after" diagnostic with fix
pub fn unexpected_space_after(token: &CstNode, ws_range: TextRange, ctx: &CheckContext) -> Diagnostic
```

Rule-specific logic stays in each rule module.

### File Structure

```
crates/lintal_linter/src/rules/whitespace/
├── mod.rs                       # Export all rules
├── common.rs                    # Shared helpers
├── whitespace_around.rs         # Existing (Phase 2)
├── whitespace_after.rs          # NEW
├── paren_pad.rs                 # NEW
├── no_whitespace_after.rs       # NEW
├── single_space_separator.rs    # NEW
├── method_param_pad.rs          # NEW
├── no_whitespace_before.rs      # NEW
├── empty_for_initializer_pad.rs # NEW
├── typecast_paren_pad.rs        # NEW
└── file_tab_character.rs        # NEW
```

## Rule Specifications

### 1. WhitespaceAfter

**Purpose**: Ensures whitespace after specific tokens.

**Checkstyle tokens**: `COMMA`, `SEMI`, `TYPECAST`, `LITERAL_IF`, `LITERAL_ELSE`, `LITERAL_WHILE`, `LITERAL_DO`, `LITERAL_FOR`, `DO_WHILE`

**Config options**:
- `tokens` - which tokens to check (default: COMMA, SEMI)

**Violations**:
- `ws.notFollowed` - "'{0}' is not followed by whitespace"

**Fix**: Insert single space after token.

### 2. ParenPad

**Purpose**: Controls whitespace inside parentheses.

**Config options**:
- `option` - `nospace` (default) or `space`
- `tokens` - which constructs to check (METHOD_CALL, CTOR_CALL, etc.)

**Violations**:
- `ws.followed` - "'{0}' is followed by whitespace" (when option=nospace)
- `ws.notFollowed` - "'{0}' is not followed by whitespace" (when option=space)
- `ws.preceded` - "'{0}' is preceded by whitespace"
- `ws.notPreceded` - "'{0}' is not preceded by whitespace"

**Fix**: Insert or remove space inside parens based on option.

### 3. NoWhitespaceAfter

**Purpose**: Ensures NO whitespace after specific tokens.

**Checkstyle tokens**: `AT`, `BNOT`, `DEC`, `DOT`, `INC`, `LNOT`, `UNARY_MINUS`, `UNARY_PLUS`, `ARRAY_DECLARATOR`, `INDEX_OP`

**Config options**:
- `tokens` - which tokens to check
- `allowLineBreaks` - allow newlines (default: true)

**Violations**:
- `ws.followed` - "'{0}' is followed by whitespace"

**Fix**: Remove whitespace after token (respecting allowLineBreaks).

### 4. SingleSpaceSeparator

**Purpose**: Ensures tokens are separated by exactly one space.

**Config options**:
- `validateComments` - check spaces before comments (default: false)

**Violations**:
- `ws.single.space.separator` - "Use a single space to separate tokens"

**Fix**: Replace multiple spaces with single space.

**Note**: Line-level scan, not token-based. Ignores leading whitespace.

### 5. MethodParamPad

**Purpose**: Controls whitespace before method/constructor parameter list opening paren.

**Config options**:
- `option` - `nospace` (default) or `space`
- `allowLineBreaks` - allow newlines before `(` (default: false)
- `tokens` - METHOD_DEF, CTOR_DEF, etc.

**Violations**:
- `ws.preceded` - "'{0}' is preceded by whitespace"
- `ws.notPreceded` - "'{0}' is not preceded by whitespace"
- `line.previous` - "'{0}' should be on the previous line"

**Fix**: Insert/remove space before `(`, or move to previous line.

### 6. NoWhitespaceBefore

**Purpose**: Ensures NO whitespace before specific tokens.

**Checkstyle tokens**: `COMMA`, `SEMI`, `POST_INC`, `POST_DEC`, `DOT`, `LABELED_STAT`, `METHOD_REF`

**Config options**:
- `tokens` - which tokens to check
- `allowLineBreaks` - allow newlines (default: false)

**Violations**:
- `ws.preceded` - "'{0}' is preceded by whitespace"

**Fix**: Remove whitespace before token (respecting allowLineBreaks).

### 7. EmptyForInitializerPad

**Purpose**: Controls whitespace in empty for-loop initializer: `for ( ; ...)`

**Config options**:
- `option` - `nospace` (default) or `space`

**Violations**:
- `ws.preceded` - "';' is preceded by whitespace" (when option=nospace)
- `ws.notPreceded` - "';' is not preceded by whitespace" (when option=space)

**Fix**: Insert/remove space before the semicolon.

**Scope**: Only applies to `for` statements with empty initializer.

### 8. TypecastParenPad

**Purpose**: Controls whitespace inside typecast parentheses: `(String) x`

**Config options**:
- `option` - `nospace` (default) or `space`

**Violations**:
- `ws.followed` / `ws.notFollowed` - for `(`
- `ws.preceded` / `ws.notPreceded` - for `)`

**Fix**: Insert/remove space inside typecast parens.

**Note**: Reuses ParenPad logic, scoped to `cast_expression` nodes.

### 9. FileTabCharacter

**Purpose**: Detects tab characters in source files.

**Config options**:
- `eachLine` - report once per line (true) or once per file (false, default)
- `fileExtensions` - which file types to check

**Violations**:
- `file.containsTab` - "File contains tab characters"

**Fix**: Replace tabs with spaces (configurable tab width).

**Note**: Pure text scan, no tree-sitter needed.

## Testing Strategy

### Test Structure

```
tests/
├── checkstyle_compat.rs              # Existing WhitespaceAround tests
├── checkstyle_whitespaceafter.rs     # WhitespaceAfter compat tests
├── checkstyle_parenpad.rs            # ParenPad compat tests
├── checkstyle_nowhitespaceafter.rs
├── checkstyle_singlespaceseparator.rs
├── checkstyle_methodparampad.rs
├── checkstyle_nowhitespacebefore.rs
├── checkstyle_emptyforinitializerpad.rs
├── checkstyle_typecastparenpad.rs
├── checkstyle_filetabcharacter.rs
└── checkstyle_repo.rs                # Repo fetching helper
```

### Test Development Flow (TDD)

1. Study checkstyle test class (e.g., `WhitespaceAfterCheckTest.java`)
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
| New rule modules | 9 |
| Shared helpers module | 1 |
| Compatibility test files | 9 |
| Estimated test cases | ~120 |
