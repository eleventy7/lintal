# EmptyLineSeparator Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement EmptyLineSeparator rule that enforces blank lines between class members.

**Architecture:** Add to existing whitespace/ module. Check that specified tokens are preceded by an empty line (unless first in container or preceded by a comment).

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics

---

## Rule Overview

Checkstyle's EmptyLineSeparator checks that certain declarations are separated by blank lines.

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `allowNoEmptyLineBetweenFields` | false | If true, fields don't need blank lines between them |
| `allowMultipleEmptyLines` | true | If false, reports multiple consecutive blank lines before tokens |
| `allowMultipleEmptyLinesInsideClassMembers` | true | If false, reports multiple blank lines inside methods |
| `tokens` | all | Which tokens to check |

### Default Tokens

PACKAGE_DEF, IMPORT, STATIC_IMPORT, CLASS_DEF, INTERFACE_DEF, ENUM_DEF, STATIC_INIT, INSTANCE_INIT, METHOD_DEF, CTOR_DEF, VARIABLE_DEF, RECORD_DEF, COMPACT_CTOR_DEF

### Real-World Usage (aeron/artio/agrona)

All three projects use:
```xml
<module name="EmptyLineSeparator">
    <property name="allowNoEmptyLineBetweenFields" value="true"/>
    <property name="tokens" value="IMPORT, CLASS_DEF, INTERFACE_DEF, ENUM_DEF, STATIC_INIT, INSTANCE_INIT, METHOD_DEF, CTOR_DEF"/>
</module>
```

## Implementation Approach

### Core Algorithm

For each checked token:
1. Find the previous sibling element (skip comments - they attach to the next token)
2. If no previous sibling (first in container), no violation
3. Calculate empty lines between previous sibling's end line and current token's start line
4. If 0 empty lines → violation: "should be separated from previous line"
5. If `allowMultipleEmptyLines=false` and >1 empty lines → violation: "has more than 1 empty lines before"

### Tree-sitter Node Mapping

| Checkstyle Token | Tree-sitter Node Kind |
|-----------------|----------------------|
| PACKAGE_DEF | `package_declaration` |
| IMPORT | `import_declaration` |
| STATIC_IMPORT | `import_declaration` (with static) |
| CLASS_DEF | `class_declaration` |
| INTERFACE_DEF | `interface_declaration` |
| ENUM_DEF | `enum_declaration` |
| STATIC_INIT | `static_initializer` |
| INSTANCE_INIT | `block` (direct child of class_body) |
| METHOD_DEF | `method_declaration` |
| CTOR_DEF | `constructor_declaration` |
| VARIABLE_DEF | `field_declaration` |
| RECORD_DEF | `record_declaration` |
| COMPACT_CTOR_DEF | `compact_constructor_declaration` |

### Comment Handling

Comments before a token "attach" to that token. A blank line before a comment satisfies the requirement:

```java
class Foo {
    void method1() {}

    // This comment attaches to method2
    void method2() {}  // OK - blank line before comment
}
```

### Violation Messages

- `'{token}' should be separated from previous line.`
- `'{token}' has more than 1 empty lines before.`

## Phased Implementation

### Phase 1: Core Detection (MVP)

Support the most common use case from real-world projects:
- tokens: METHOD_DEF, CTOR_DEF, CLASS_DEF, INTERFACE_DEF, ENUM_DEF, STATIC_INIT, INSTANCE_INIT
- allowNoEmptyLineBetweenFields option (skip field checks when true)
- No auto-fix initially

### Phase 2: Additional Tokens

- IMPORT (first import after package needs blank line)
- PACKAGE_DEF
- VARIABLE_DEF (when allowNoEmptyLineBetweenFields=false)

### Phase 3: Multiple Empty Lines

- allowMultipleEmptyLines option
- allowMultipleEmptyLinesInsideClassMembers option

### Phase 4: Auto-fix

- Insert blank line where missing
- Remove extra blank lines when disallowed

## Testing Strategy

1. **Unit tests**: Basic cases for each token type
2. **Checkstyle compatibility**: Run against checkstyle fixtures
3. **Real-world validation**: Run against aeron/artio/agrona (should find 0 violations)

## Scope Decision

Given complexity (50 test files), implement Phase 1 first to cover real-world usage, then iterate.
