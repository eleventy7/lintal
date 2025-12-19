# Phase 5: Modifier Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement 4 modifier rules with 100% checkstyle compatibility.

**Architecture:** Create `rules/modifier/` module with shared helpers in `common.rs`. Each rule follows the existing pattern: `FromConfig` + `Rule` traits. Use checkstyle test fixtures as oracle for TDD.

**Tech Stack:** Rust, tree-sitter-java, lintal_diagnostics, checkstyle fixtures

---

## Task 1: Create modifier module structure and shared helpers

**Files:**
- Create: `crates/lintal_linter/src/rules/modifier/mod.rs`
- Create: `crates/lintal_linter/src/rules/modifier/common.rs`
- Modify: `crates/lintal_linter/src/rules/mod.rs`

**Step 1: Create the modifier module directory and mod.rs**

```rust
// crates/lintal_linter/src/rules/modifier/mod.rs
//! Modifier rules for checking modifier usage and ordering.

pub mod common;

// Rules will be added as they're implemented
```

**Step 2: Create common.rs with shared helpers**

```rust
// crates/lintal_linter/src/rules/modifier/common.rs
//! Shared helpers for modifier rules.

use lintal_java_cst::CstNode;

/// JLS-recommended modifier order.
pub const JLS_MODIFIER_ORDER: &[&str] = &[
    "public", "protected", "private", "abstract", "default", "static",
    "sealed", "non-sealed", "final", "transient", "volatile",
    "synchronized", "native", "strictfp",
];

/// Get the index of a modifier in JLS order, or None if not found.
pub fn jls_order_index(modifier: &str) -> Option<usize> {
    JLS_MODIFIER_ORDER.iter().position(|&m| m == modifier)
}

/// Check if a modifiers node contains a specific modifier.
pub fn has_modifier(modifiers: &CstNode, modifier_kind: &str) -> bool {
    modifiers.children().any(|child| child.kind() == modifier_kind)
}

/// Check if we're inside an interface definition.
pub fn is_in_interface(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "interface_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an annotation definition.
pub fn is_in_annotation(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "annotation_type_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}

/// Check if the containing class is final.
pub fn is_in_final_class(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "class_declaration" {
            if let Some(modifiers) = parent.child_by_field_name("modifiers") {
                return has_modifier(&modifiers, "final");
            }
            return false;
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an anonymous class.
pub fn is_in_anonymous_class(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "object_creation_expression" {
            // Check if it has a class body (anonymous class)
            return parent.child_by_field_name("body").is_some();
        }
        current = parent.parent();
    }
    false
}

/// Check if we're inside an enum definition.
pub fn is_in_enum(node: &CstNode) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == "enum_declaration" {
            return true;
        }
        current = parent.parent();
    }
    false
}
```

**Step 3: Update rules/mod.rs to export modifier**

Add to `crates/lintal_linter/src/rules/mod.rs`:
```rust
pub mod modifier;
```

**Step 4: Verify it compiles**

Run: `cargo check --package lintal_linter`

**Step 5: Commit**

```bash
git add crates/lintal_linter/src/rules/modifier/
git add crates/lintal_linter/src/rules/mod.rs
git commit -m "feat(modifier): add modifier module structure and shared helpers"
```

---

## Task 2: Implement ModifierOrder rule - basic structure

**Files:**
- Create: `crates/lintal_linter/src/rules/modifier/modifier_order.rs`
- Modify: `crates/lintal_linter/src/rules/modifier/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_modifierorder.rs`

**Reference:** Study checkstyle's `ModifierOrderCheck.java` and `ModifierOrderCheckTest.java`

**Step 1: Create test file with first test cases**

Look at ModifierOrderCheckTest.java to extract expected violations. Start with `testItOne` and `testItTwo`.

**Step 2: Implement ModifierOrder rule struct**

```rust
pub struct ModifierOrder;

impl FromConfig for ModifierOrder {
    const MODULE_NAME: &'static str = "ModifierOrder";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}
```

**Step 3: Implement Rule trait**

The rule must:
1. Visit MODIFIERS nodes
2. Skip annotations at the start
3. Check remaining modifiers are in JLS order
4. Check no annotations appear after non-annotation modifiers
5. Report first violation found

**Step 4: Register in registry.rs**

**Step 5: Run tests, iterate until passing**

**Step 6: Commit**

---

## Task 3: Complete ModifierOrder with sealed/non-sealed and annotation order

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/modifier_order.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_modifierorder.rs`

**Step 1: Add test cases for sealed/non-sealed modifiers**

From `testModifierOrderSealedAndNonSealed`.

**Step 2: Add test cases for annotation ordering**

From `testSkipTypeAnnotationsOne`, `testAnnotationOnAnnotationDeclaration`.

**Step 3: Implement type annotation detection**

Type annotations (on types, not declarations) should be skipped.

**Step 4: Run all tests**

**Step 5: Commit**

---

## Task 4: Implement FinalParameters rule

**Files:**
- Create: `crates/lintal_linter/src/rules/modifier/final_parameters.rs`
- Modify: `crates/lintal_linter/src/rules/modifier/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_finalparameters.rs`

**Reference:** Study checkstyle's `FinalParametersCheck.java` and test files

**Step 1: Create test file from FinalParametersCheckTest.java**

**Step 2: Implement FinalParameters rule**

```rust
pub struct FinalParameters {
    tokens: HashSet<FinalParametersToken>,
    ignore_primitive_types: bool,
    ignore_unnamed_parameters: bool,
}

pub enum FinalParametersToken {
    MethodDef,
    CtorDef,
    LiteralCatch,
    ForEachClause,
}
```

**Step 3: Implement check logic**

- Skip methods without body (interface/abstract/native)
- Skip receiver parameters
- Skip primitive types if configured
- Skip unnamed `_` parameters if configured
- Report missing `final` on parameters

**Step 4: Register in registry.rs**

**Step 5: Run tests, iterate**

**Step 6: Commit**

---

## Task 5: Implement RedundantModifier rule - basic structure

**Files:**
- Create: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/src/rules/modifier/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Reference:** Study checkstyle's `RedundantModifierCheck.java`

**Step 1: Create test file with basic test cases**

Start with: `testItOne`, `testItTwo`, `testClassesInsideOfInterfaces`

**Step 2: Implement RedundantModifier rule struct**

```rust
pub struct RedundantModifier {
    jdk_version: u32,
}

impl FromConfig for RedundantModifier {
    const MODULE_NAME: &'static str = "RedundantModifier";

    fn from_config(properties: &Properties) -> Self {
        let jdk_version = properties
            .get("jdkVersion")
            .and_then(|v| parse_jdk_version(v))
            .unwrap_or(22);
        Self { jdk_version }
    }
}
```

**Step 3: Implement basic checks**

- Interface/annotation modifiers (public, abstract, static)
- Interface field modifiers (public, static, final)

**Step 4: Register in registry.rs**

**Step 5: Run tests, iterate**

**Step 6: Commit**

---

## Task 6: RedundantModifier - enum and constructor cases

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Step 1: Add test cases**

From: `testEnumConstructorIsImplicitlyPrivate`, `testNotPublicClassConstructorHasNotPublicModifier`, `testNestedStaticEnum`

**Step 2: Implement enum constructor check**

Enum constructors are implicitly private.

**Step 3: Implement class constructor check**

Public modifier is redundant on constructors of non-public classes.

**Step 4: Implement nested enum static check**

Nested enums are implicitly static.

**Step 5: Run tests, iterate**

**Step 6: Commit**

---

## Task 7: RedundantModifier - final method cases

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Step 1: Add test cases**

From: `testFinalInAnonymousClass`, `testFinalInInterface`, `testPrivateMethodInPrivateClass`, `testEnumMethods`

**Step 2: Implement final method checks**

- Final on methods in final classes
- Final on methods in anonymous classes
- Final on private methods
- Final on static methods in enums

**Step 3: Handle SafeVarargs exception**

Methods annotated with @SafeVarargs can be final.

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 8: RedundantModifier - try-with-resources and abstract method parameters

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Step 1: Add test cases**

From: `testFinalInTryWithResource`, `testTryWithResourcesBlock`, `testFinalInAbstractMethods`

**Step 2: Implement try-with-resources check**

Resources in try-with-resources are implicitly final.

**Step 3: Implement abstract method parameter check**

Final on parameters of abstract methods is redundant (no code to modify them).

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 9: RedundantModifier - records, sealed classes, strictfp

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Step 1: Add test cases**

From: `testRecords`, `testSealedClasses`, `testStrictfpWithJava17`, `testStrictfpWithDefaultVersion`

**Step 2: Implement record checks**

- Records are implicitly final
- Nested records are implicitly static

**Step 3: Implement strictfp check (JDK 17+)**

strictfp is redundant since JDK 17.

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 10: RedundantModifier - unnamed variables (JDK 22+)

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/redundant_modifier.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_redundantmodifier.rs`

**Step 1: Add test cases**

From: `testFinalUnnamedVariablesWithDefaultVersion`, `testFinalUnnamedVariablesWithOldVersion`

**Step 2: Implement unnamed variable check**

Final on unnamed variables (`_`) is redundant since JDK 22.

**Step 3: Run all RedundantModifier tests**

**Step 4: Commit**

---

## Task 11: Implement FinalLocalVariable rule - basic structure

**Files:**
- Create: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/src/rules/modifier/mod.rs`
- Modify: `crates/lintal_linter/src/registry.rs`
- Create: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Reference:** Study checkstyle's `FinalLocalVariableCheck.java` - this is a complex stateful check

**Step 1: Create test file with simple test cases**

Start with basic cases that don't involve complex control flow.

**Step 2: Implement data structures**

```rust
pub struct FinalLocalVariable {
    validate_enhanced_for_loop_variable: bool,
    validate_unnamed_variables: bool,
}

struct ScopeData {
    variables: HashMap<String, VariableCandidate>,
    uninitialized: HashSet<String>,
    contains_break: bool,
}

struct VariableCandidate {
    ident_range: TextRange,
    ident_text: String,
    assigned: bool,
    already_assigned: bool,
}
```

**Step 3: Implement basic scope tracking**

- Push scope on method/constructor/block entry
- Pop scope on exit, report violations
- Track variable declarations

**Step 4: Register in registry.rs**

**Step 5: Run tests, iterate**

**Step 6: Commit**

---

## Task 12: FinalLocalVariable - assignment tracking

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Step 1: Add test cases for assignments**

From tests involving simple variable assignments.

**Step 2: Implement assignment detection**

Track all forms of assignment:
- Simple: `x = value`
- Compound: `x += value`, `x -= value`, etc.
- Increment/decrement: `x++`, `++x`, `x--`, `--x`

**Step 3: Update candidate status on assignment**

- First assignment: mark as assigned
- Second assignment: mark as already_assigned (remove from candidates)

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 13: FinalLocalVariable - control flow (if/else)

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Step 1: Add test cases for if/else**

From tests involving branching.

**Step 2: Implement if/else handling**

Variables assigned in both branches should not be removed from candidates.

**Step 3: Handle uninitialized variables across branches**

Track which variables are uninitialized entering a branch.

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 14: FinalLocalVariable - control flow (switch)

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Step 1: Add test cases for switch**

From: `testCheckSwitchAssignment`, switch-related tests.

**Step 2: Implement switch handling**

Similar to if/else but with multiple branches.

**Step 3: Handle switch expressions (JDK 14+)

**Step 4: Run tests, iterate**

**Step 5: Commit**

---

## Task 15: FinalLocalVariable - loops

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Step 1: Add test cases for loops**

From tests involving for/while/do-while loops.

**Step 2: Implement loop handling**

Variables declared outside a loop but assigned inside cannot be final (may be assigned multiple times).

**Step 3: Handle for-each loops**

With `validateEnhancedForLoopVariable` option.

**Step 4: Handle break statements**

Break can affect which assignments are reachable.

**Step 5: Run tests, iterate**

**Step 6: Commit**

---

## Task 16: FinalLocalVariable - edge cases

**Files:**
- Modify: `crates/lintal_linter/src/rules/modifier/final_local_variable.rs`
- Modify: `crates/lintal_linter/tests/checkstyle_finallocalvariable.rs`

**Step 1: Add remaining test cases**

Lambda parameters, anonymous classes, multi-catch, etc.

**Step 2: Implement edge case handling**

- Skip lambda parameters
- Skip multi-catch parameters
- Handle constructor chaining
- Handle named variables (validateUnnamedVariables)

**Step 3: Run all FinalLocalVariable tests**

**Step 4: Commit**

---

## Task 17: Add auto-fix support for all rules

**Files:**
- Modify all rule files

**Step 1: ModifierOrder auto-fix**

Reorder modifiers to correct JLS order.

**Step 2: FinalParameters auto-fix**

Add `final ` before parameter type.

**Step 3: RedundantModifier auto-fix**

Remove redundant modifier keyword.

**Step 4: FinalLocalVariable auto-fix**

Add `final ` before variable type.

**Step 5: Run tests, verify fixes**

**Step 6: Commit**

---

## Task 18: Update CI and final integration

**Files:**
- Modify: `.github/workflows/ci.yml`

**Step 1: Add all new test files to CI**

```yaml
cargo test --package lintal_linter --test checkstyle_modifierorder
cargo test --package lintal_linter --test checkstyle_finalparameters
cargo test --package lintal_linter --test checkstyle_redundantmodifier
cargo test --package lintal_linter --test checkstyle_finallocalvariable
```

**Step 2: Run clippy and fix any warnings**

Run: `cargo clippy --all-targets --all-features -- -D warnings`

**Step 3: Run all tests**

Run: `cargo test --package lintal_linter`

**Step 4: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add Phase 5 modifier compatibility tests"
```
