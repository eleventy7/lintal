# Missing Checks for Platform Compatibility

Analysis of checkstyle rules configured in `/Users/shaunlaurens/src/platform/config/checkstyle/checkstyle.xml` compared to lintal's current implementation.

## Summary

| Category | Platform Uses | Lintal Has | Missing |
|----------|---------------|------------|---------|
| Checker Rules | 6 | 1 | 5 |
| TreeWalker Rules | 61 | 35 | 26 |
| **Total Actionable** | 67 | 36 | **13** |

Note: Several "missing" items are suppression/filter infrastructure (already supported differently) or naming rules (already implemented).

---

## Priority 1: Auto-fixable (Safe)

These rules can be automatically fixed with high confidence and no semantic changes.

### ~~EmptyStatement~~ ✅ IMPLEMENTED
**Checkstyle module:** `EmptyStatement`
**Status:** Implemented in lintal with auto-fix
**Fix:** Removes unnecessary semicolons

### ~~FinalClass~~ ✅ IMPLEMENTED
**Checkstyle module:** `FinalClass`
**Status:** Implemented in lintal with auto-fix
**Fix:** Adds `final` modifier to classes with only private constructors

### ~~DefaultComesLast~~ ✅ IMPLEMENTED
**Checkstyle module:** `DefaultComesLast`
**Status:** Implemented in lintal (detection only, no auto-fix)
**Config:** Supports `skipIfLastAndSharedWithCase` option

### ~~StringLiteralEquality~~ ✅ IMPLEMENTED
**Checkstyle module:** `StringLiteralEquality`
**Status:** Implemented in lintal with auto-fix
**Fix:** `str == "literal"` → `"literal".equals(str)` (null-safe)

### ~~SimplifyBooleanExpression~~ ✅ IMPLEMENTED
**Checkstyle module:** `SimplifyBooleanExpression`
**Status:** Implemented in lintal with auto-fix
**Fix:** Simplifies `b == true` → `b`, `b == false` → `!b`, etc.

---

## Priority 2: Auto-fixable (Sometimes/Unsafe)

These can be auto-fixed but may require user confirmation or have edge cases.

### DeclarationOrder
**Checkstyle module:** `DeclarationOrder`
**Fix strategy:** Reorder class members (static fields → instance fields → constructors → methods)
**Complexity:** High (large diffs, must preserve comments/annotations)
**Risk:** Medium - changes are safe but produce large diffs

### HideUtilityClassConstructor
**Checkstyle module:** `HideUtilityClassConstructor`
**Fix strategy:** Add private constructor to utility class
**Complexity:** Medium
**Risk:** Low - adding code is safe
**Example:**
```java
// Before
public class Utils {
    public static void helper() {}
}

// After
public class Utils {
    private Utils() {}
    public static void helper() {}
}
```

### MutableException
**Checkstyle module:** `MutableException`
**Fix strategy:** Add `final` to exception class fields
**Complexity:** Low
**Risk:** Low - may affect subclasses

---

## Priority 3: Check-only (No auto-fix)

These require semantic understanding or human judgment to fix.

### MethodLength
**Checkstyle module:** `MethodLength`
**Detection:** Methods exceeding configured line count
**Why no fix:** Requires semantic refactoring

### NestedTryDepth
**Checkstyle module:** `NestedTryDepth`
**Detection:** Excessive nesting of try blocks
**Why no fix:** Requires semantic refactoring

### CovariantEquals
**Checkstyle module:** `CovariantEquals`
**Detection:** Classes with `equals(SpecificType)` but no `equals(Object)`
**Why no fix:** Requires code generation

### HiddenField
**Checkstyle module:** `HiddenField`
**Detection:** Local variables/parameters hiding class fields
**Why no fix:** Requires renaming (user choice)

### InnerAssignment
**Checkstyle module:** `InnerAssignment`
**Detection:** Assignments inside expressions
**Why no fix:** Requires semantic restructuring

### MissingSwitchDefault
**Checkstyle module:** `MissingSwitchDefault`
**Detection:** Switch statements without default case
**Why no fix:** Cannot determine appropriate default behavior
**Note:** Could offer "add empty default" as unsafe fix

### EqualsHashCode
**Checkstyle module:** `EqualsHashCode`
**Detection:** Classes overriding one of equals/hashCode but not both
**Why no fix:** Requires code generation

### FallThrough
**Checkstyle module:** `FallThrough`
**Detection:** Case statements falling through without break/comment
**Why no fix:** Cannot determine if intentional
**Note:** Could offer "add break" as unsafe fix

### PackageDeclaration
**Checkstyle module:** `PackageDeclaration`
**Detection:** Missing package declaration
**Why no fix:** Cannot determine correct package from file alone

### TodoComment
**Checkstyle module:** `TodoComment`
**Detection:** TODO/FIXME comments in code
**Why no fix:** Informational only

### IllegalType
**Checkstyle module:** `IllegalType`
**Detection:** Usage of banned types
**Why no fix:** Requires semantic refactoring

### DescendantToken
**Checkstyle module:** `DescendantToken`
**Detection:** General AST pattern matching
**Why no fix:** Too general for auto-fix

### JavadocMethod
**Checkstyle module:** `JavadocMethod`
**Detection:** Missing/incomplete method Javadoc
**Why no fix:** Cannot generate meaningful documentation

---

## Infrastructure (Already Supported)

These are not linting rules but configuration/infrastructure:

| Module | Status |
|--------|--------|
| `SuppressionFilter` | Supported via suppressions.xml |
| `SuppressWithPlainTextCommentFilter` | Supported via CHECKSTYLE:OFF comments |
| `SuppressWarningsFilter` | Supported via @SuppressWarnings |
| `SeverityMatchFilter` | Not needed (lintal handles severity differently) |
| `SuppressWarningsHolder` | Internal helper for SuppressWarningsFilter |
| `RegexpSinglelineJava` | Not implemented (low priority, project-specific) |

---

## Size Checks

### LineLength
**Checkstyle module:** `LineLength`
**Detection:** Lines exceeding configured length
**Auto-fix:** Not recommended (would require intelligent line breaking)
**Complexity:** High
**Status:** Could implement as check-only

---

## Recommended Implementation Order

Based on platform usage, auto-fix capability, and complexity:

1. ~~**EmptyStatement**~~ ✅ Implemented
2. ~~**SimplifyBooleanExpression**~~ ✅ Implemented
3. ~~**FinalClass**~~ ✅ Implemented
4. ~~**StringLiteralEquality**~~ ✅ Implemented
5. ~~**DefaultComesLast**~~ ✅ Implemented
6. **HideUtilityClassConstructor** - Simple code addition
7. **MutableException** - Simple modifier addition
8. **MethodLength** - Check-only, useful metric
9. **LineLength** - Check-only, useful metric
10. **DeclarationOrder** - Complex but valuable

---

## Already Implemented in Lintal

For reference, these platform checks are already working:

**Whitespace:** WhitespaceAround, WhitespaceAfter, NoWhitespaceBefore, NoWhitespaceAfter, SingleSpaceSeparator, ParenPad, TypecastParenPad, MethodParamPad, EmptyForInitializerPad, OperatorWrap, EmptyLineSeparator, FileTabCharacter, Indentation

**Blocks:** LeftCurly, RightCurly, NeedBraces, EmptyBlock, EmptyCatchBlock, AvoidNestedBlocks

**Modifiers:** FinalClass, FinalLocalVariable, FinalParameters, ModifierOrder, RedundantModifier

**Style:** UpperEll, ArrayTypeStyle

**Imports:** RedundantImport, UnusedImports

**Coding:** DefaultComesLast, EmptyStatement, MultipleVariableDeclarations, OneStatementPerLine, SimplifyBooleanExpression, SimplifyBooleanReturn, StringLiteralEquality

**Naming:** ConstantName, LocalFinalVariableName, LocalVariableName, MemberName, MethodName, PackageName, ParameterName, StaticVariableName, TypeName
