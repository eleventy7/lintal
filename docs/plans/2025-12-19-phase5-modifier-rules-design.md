# Phase 5: Modifier Rules Design

Implementation plan for 4 modifier rules with 100% checkstyle compatibility.

## Implementation Order

Based on complexity and dependencies:

| Priority | Rule | Test Fixtures | Complexity | Notes |
|----------|------|---------------|------------|-------|
| 1 | ModifierOrder | 8 | Medium | Stateless, JLS order check |
| 2 | FinalParameters | 12 | Low | Stateless, simple check |
| 3 | RedundantModifier | 27 | High | Many edge cases, JDK version aware |
| 4 | FinalLocalVariable | 33 | Very High | Stateful, data flow analysis |

## Architecture

### File Structure

```
crates/lintal_linter/src/rules/
├── mod.rs                    # Add modifier export
├── whitespace/               # Existing (Phase 2-3)
├── blocks/                   # Existing (Phase 4)
└── modifier/                 # NEW
    ├── mod.rs
    ├── common.rs             # Shared helpers
    ├── modifier_order.rs
    ├── final_parameters.rs
    ├── redundant_modifier.rs
    └── final_local_variable.rs
```

### Shared Helpers Module

Create `rules/modifier/common.rs` with:

```rust
/// JLS-recommended modifier order
pub const JLS_MODIFIER_ORDER: &[&str] = &[
    "public", "protected", "private", "abstract", "default", "static",
    "sealed", "non-sealed", "final", "transient", "volatile",
    "synchronized", "native", "strictfp",
];

/// Check if a node has a specific modifier
pub fn has_modifier(modifiers: &CstNode, modifier: &str) -> bool

/// Get all modifiers from a modifiers node
pub fn get_modifiers(modifiers: &CstNode) -> Vec<CstNode>

/// Check if we're inside an interface or annotation
pub fn is_interface_or_annotation_member(node: &CstNode) -> bool

/// Check if parent class is final
pub fn is_in_final_class(node: &CstNode) -> bool

/// Check if node is in anonymous class
pub fn is_in_anonymous_class(node: &CstNode) -> bool
```

## Rule Specifications

### 1. ModifierOrder

**Purpose**: Checks that modifiers appear in JLS-recommended order.

**Config options**: None (stateless check)

**JLS Order**:
1. public
2. protected
3. private
4. abstract
5. default
6. static
7. sealed
8. non-sealed
9. final
10. transient
11. volatile
12. synchronized
13. native
14. strictfp

**Additional rule**: All annotations must appear before all modifiers.

**Violations**:
- `mod.order` - "'X' modifier out of order with the JLS suggestions"
- `annotation.order` - "'@X' annotation modifier does not precede non-annotation modifiers"

**Auto-fix**: Reorder modifiers to correct order (safe fix).

### 2. FinalParameters

**Purpose**: Checks that parameters have `final` modifier.

**Config options**:
- `tokens` - which constructs to check (default: METHOD_DEF, CTOR_DEF)
  - Acceptable: METHOD_DEF, CTOR_DEF, LITERAL_CATCH, FOR_EACH_CLAUSE
- `ignorePrimitiveTypes` - skip primitive type parameters (default: false)
- `ignoreUnnamedParameters` - skip `_` parameters (default: true)

**Skipped cases**:
- Interface methods (no body)
- Abstract methods (no body)
- Native methods (no body)
- Receiver parameters (`this` parameter)

**Violations**:
- `final.parameter` - "Parameter 'X' should be final"

**Auto-fix**: Add `final` keyword before parameter type (safe fix).

### 3. RedundantModifier

**Purpose**: Detects modifiers that are redundant given the context.

**Config options**:
- `jdkVersion` - Java version for version-specific checks (default: 22)

**Redundant cases**:

| Context | Redundant Modifiers |
|---------|---------------------|
| Interface/annotation definition | `abstract`, `static` |
| Interface field | `public`, `static`, `final` |
| Interface method | `public`, `abstract` |
| Interface nested type | `public`, `static` |
| Annotation field | `public`, `static`, `final` |
| Annotation method | `public`, `abstract` |
| Enum constructor | any visibility modifier |
| Enum nested type | `static` |
| Record definition | `final` |
| Record nested type | `static` |
| Method in final class | `final` |
| Method in anonymous class | `final` |
| Private method | `final` |
| Static method in enum | `final` (if overridable) |
| Constructor in non-public class | `public` |
| Try-with-resources variable | `final` |
| Abstract method parameter | `final` |
| `strictfp` (JDK 17+) | `strictfp` |
| Unnamed variable `_` (JDK 22+) | `final` |

**Violations**:
- `redundantModifier` - "Redundant 'X' modifier"

**Auto-fix**: Remove redundant modifier (safe fix).

### 4. FinalLocalVariable

**Purpose**: Checks that local variables never reassigned should be `final`.

**Config options**:
- `validateEnhancedForLoopVariable` - check for-each loop variables (default: false)
- `validateUnnamedVariables` - check `_` variables (default: false)

**Complexity**: This rule requires **data flow analysis**:
1. Track variable declarations in scope stack
2. Track assignments to variables
3. Handle branching (if/else, switch)
4. Handle loops (variable in loop may be assigned multiple times)
5. Handle break statements
6. At scope exit, report variables assigned exactly once

**Skipped cases**:
- Class fields (only local variables)
- Variables in for-init (`for (int i = 0; ...`)
- Already final variables
- Lambda parameters
- Multi-catch parameters

**Violations**:
- `final.variable` - "Variable 'X' should be declared final"

**Auto-fix**: Add `final` keyword before variable type (safe fix, but requires careful analysis).

## Testing Strategy

### Test Structure

```
tests/
├── checkstyle_modifierorder.rs        # 8 test fixtures
├── checkstyle_finalparameters.rs      # 12 test fixtures
├── checkstyle_redundantmodifier.rs    # 27 test fixtures
├── checkstyle_finallocalvariable.rs   # 33 test fixtures
└── checkstyle_repo.rs                 # Shared (already exists)
```

### Test Development Flow (TDD)

1. Study checkstyle test class
2. Extract expected violations from test methods
3. Write compatibility test loading fixture + asserting violations
4. Run test -> fails (RED)
5. Implement rule -> pass (GREEN)
6. Refactor if needed

### Definition of Done (per rule)

- [ ] Rule struct with config options
- [ ] `FromConfig` implementation
- [ ] `Rule` implementation
- [ ] All checkstyle test fixtures passing
- [ ] Registered in registry
- [ ] Auto-fix implementation
- [ ] Unit tests for edge cases

## Implementation Notes

### ModifierOrder Implementation

```rust
pub struct ModifierOrder;

impl Rule for ModifierOrder {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        // 1. Get MODIFIERS node
        // 2. Collect all modifier children (including annotations)
        // 3. Check annotations come first
        // 4. Check remaining modifiers are in JLS order
        // 5. Report first out-of-order modifier
    }
}
```

### FinalParameters Implementation

```rust
pub struct FinalParameters {
    tokens: HashSet<FinalParametersToken>,
    ignore_primitive_types: bool,
    ignore_unnamed_parameters: bool,
}

impl Rule for FinalParameters {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        // 1. Check if method/ctor has body (SLIST)
        // 2. Get PARAMETERS node
        // 3. For each PARAMETER_DEF:
        //    - Skip if has FINAL modifier
        //    - Skip if receiver parameter
        //    - Skip if primitive and ignorePrimitiveTypes
        //    - Skip if unnamed and ignoreUnnamedParameters
        //    - Report violation
    }
}
```

### RedundantModifier Implementation

```rust
pub struct RedundantModifier {
    jdk_version: u32,
}

impl Rule for RedundantModifier {
    fn check(&self, ctx: &RuleContext) -> Vec<Diagnostic> {
        // Match on node type, check for redundant modifiers based on context
        // Many special cases - see specification above
    }
}
```

### FinalLocalVariable Implementation

This requires a stateful visitor pattern:

```rust
pub struct FinalLocalVariable {
    validate_enhanced_for_loop_variable: bool,
    validate_unnamed_variables: bool,
}

struct ScopeData {
    variables: HashMap<String, VariableCandidate>,
    uninitialized: Vec<String>,
    contains_break: bool,
}

struct VariableCandidate {
    ident: CstNode,
    assigned: bool,
    already_assigned: bool, // assigned more than once
}

// Need to implement as a stateful visitor that:
// 1. Pushes scope on entering method/block
// 2. Tracks variable declarations
// 3. Tracks assignments (including compound assignments, ++/--)
// 4. Handles control flow (if/else, switch, loops)
// 5. Pops scope and reports violations on exit
```

## Deliverables Summary

| Deliverable | Count |
|-------------|-------|
| New rule modules | 4 |
| Shared helpers module | 1 |
| Compatibility test files | 4 |
| Estimated test cases | ~80 |
