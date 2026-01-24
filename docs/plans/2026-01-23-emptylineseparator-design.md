# EmptyLineSeparator Check Implementation Design

## Overview

Enhance EmptyLineSeparator to achieve 100% checkstyle compatibility and add auto-fix capability.

## Current State

The current implementation handles:
- Class/interface/enum body member separation
- `allowNoEmptyLineBetweenFields` configuration
- `allowMultipleEmptyLines` (partial - only for "before" violations)
- Token-based filtering

## Gap Analysis

### Missing Features

| Feature | Description | Violation Message |
|---------|-------------|-------------------|
| File-level checks | Package/import/class separation at file level | `'IMPORT' should be separated from previous line.` |
| Multiple empty lines AFTER | Trailing blank lines after `}` | `'}' has more than 1 empty lines after.` |
| Multiple empty lines INSIDE | Multiple blank lines inside method/ctor/init bodies | `'There is more than 1 empty line after this line.'` |
| Auto-fix | Insert/remove blank lines | N/A |

### Test Fixtures (50 files)

Key fixtures to test:
1. `InputEmptyLineSeparator.java` - Basic separation (14 expected violations)
2. `InputEmptyLineSeparatorMultipleEmptyLines.java` - `allowMultipleEmptyLines=false` (8 violations)
3. `InputEmptyLineSeparatorMultipleEmptyLinesInside.java` - `allowMultipleEmptyLinesInsideClassMembers=false` (6 violations)
4. `InputEmptyLineSeparatorImports.java` - Import group separation
5. `InputEmptyLineSeparatorRecordsAndCompactCtors.java` - Records support

## Implementation Plan

### Phase 1: File-Level Checks

Add `program` to `RELEVANT_KINDS` and implement checks for:
1. Package → Import separation
2. Import → Import (static vs regular) separation
3. Import → Class/Interface/Enum separation

Tree-sitter node structure at file level:
```
program
  package_declaration
  import_declaration (multiple)
  class_declaration / interface_declaration / enum_declaration
```

### Phase 2: Multiple Empty Lines AFTER

When `allowMultipleEmptyLines=false`, check for >1 blank lines after:
- Class members (method, constructor, field, init blocks)
- Before closing `}` of class body

This requires tracking the END of each member and checking blank lines before the next member or closing brace.

### Phase 3: Multiple Empty Lines INSIDE

When `allowMultipleEmptyLinesInsideClassMembers=false`, scan inside:
- Method bodies
- Constructor bodies
- Static initializer bodies
- Instance initializer bodies

Check for consecutive blank lines within these blocks.

### Phase 4: Auto-Fix

For "should be separated" violations:
- Insert single blank line before the element

For "has more than 1 empty lines before" violations:
- Remove excess blank lines, keep exactly one

For "more than 1 empty line after" violations:
- Remove excess blank lines inside block

## Test Harness

Parse expected violations from checkstyle test file comments:
- `// violation ''X' should be separated from previous line.'`
- `// violation ''X' has more than 1 empty lines before.'`
- `// violation 'There is more than 1 empty line after this line.'`
- `// violation ''}' has more than 1 empty lines after.'`

Report correct matches, missing matches, and false positives for each fixture.

## Configuration Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `allowNoEmptyLineBetweenFields` | bool | false | Allow fields without blank lines between |
| `allowMultipleEmptyLines` | bool | true | Allow multiple blank lines between members |
| `allowMultipleEmptyLinesInsideClassMembers` | bool | true | Allow multiple blank lines inside method bodies |
| `tokens` | set | all 13 | Which token types to check |

## Complexity Assessment

- File-level checks: Medium (new node type handling)
- Multiple empty lines AFTER: Medium (track member end positions)
- Multiple empty lines INSIDE: High (traverse into method bodies)
- Auto-fix: Medium (line manipulation)

Total: ~50 test fixtures, estimate significant implementation effort.
