# Import Rules Design

Phase 7 implementation: UnusedImports and RedundantImport rules.

## Module Structure

```
crates/lintal_linter/src/rules/imports/
├── mod.rs                 # Exports
├── common.rs              # Shared: import parsing, usage collection
├── redundant_import.rs    # RedundantImport rule
└── unused_imports.rs      # UnusedImports rule
```

## Data Structures

```rust
/// Represents a parsed import statement
pub struct ImportInfo {
    pub path: String,                 // "java.util.List" or "java.util.*"
    pub simple_name: Option<String>,  // "List" or None for wildcards
    pub is_static: bool,              // static import?
    pub is_wildcard: bool,            // ends with .*?
    pub range: TextRange,             // for diagnostic location
}

/// Types of redundancy detected
pub enum RedundancyKind {
    SamePackage,        // import from current package
    JavaLang,           // import from java.lang
    Duplicate(usize),   // duplicate of import at line N
}
```

## RedundantImport Rule

Detects three types of redundant imports:

1. **Same-package imports**: `import com.foo.Bar;` when file is in package `com.foo`
2. **java.lang imports**: `import java.lang.String;` or `import java.lang.*;`
3. **Duplicate imports**: Same import path appearing multiple times

### Logic

1. Extract current package from `package_declaration` node
2. For each import:
   - Check if path matches current package (not subpackage)
   - Check if path starts with `java.lang.` or equals `java.lang`
   - Track seen imports, flag duplicates

### Fix

Delete the entire import line including trailing newline.

## UnusedImports Rule

Detects imports not referenced in code or Javadoc.

### Configuration

- `processJavadoc` (default: true) - scan Javadoc for type references

### Usage Collection

**Code usages** - walk AST for:
- `type_identifier` nodes (class names)
- First part of `scoped_type_identifier` (e.g., `Map` in `Map.Entry`)
- Annotation names (`@Foo` means `Foo` is used)
- Static method/field call targets

**Javadoc usages** - regex parse block comments for:
- `{@link Type}`, `{@link Type#method}`, `{@link Type#method(ParamType)}`
- `{@linkplain Type text}`
- `@see Type`
- `@throws Type`, `@exception Type`

### Logic

1. Collect all imports
2. Collect all type usages (code + optionally Javadoc)
3. For each non-wildcard import:
   - Check if simple_name appears in usage set
4. Skip wildcard imports (can't verify without type resolution)

### Edge Cases

- Multi-line imports: `import java.io.\n    File;`
- Fully-qualified usage: `java.io.File` in code doesn't count as using `import java.io.File`
- Inner class: `JToolBar.Separator` means `JToolBar` import is used
- Static imports: check method/field name in static call context

### Fix

Delete the entire import line including trailing newline.

## Testing

Use checkstyle compatibility test pattern:

```rust
#[test]
fn test_redundant_import_checkstyle_compat() {
    // Clone checkstyle fixtures to target/checkstyle-tests/
    // Run lintal, compare violation lines with expected
}
```

Key test files:
- `redundantimport/InputRedundantImportWithChecker.java`
- `unusedimports/InputUnusedImports.java`
- `unusedimports/InputUnusedImportsFromJavaLang.java`
- `unusedimports/InputUnusedImportsWithValueTag.java`

## Implementation Order

1. Create `imports/` module structure
2. Implement `common.rs` with ImportInfo and import parsing
3. Implement RedundantImport (simpler, no usage analysis)
4. Add usage collection to common.rs
5. Implement UnusedImports
6. Add Javadoc parsing
7. Compatibility tests for both rules
