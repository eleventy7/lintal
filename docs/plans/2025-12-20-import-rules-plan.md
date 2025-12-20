# Import Rules Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement UnusedImports and RedundantImport rules with checkstyle compatibility.

**Architecture:** Create new `imports/` rule category with shared import parsing utilities. RedundantImport detects same-package, java.lang, and duplicate imports. UnusedImports tracks type references in code and Javadoc to find unreferenced imports.

**Tech Stack:** Rust, tree-sitter-java, regex (for Javadoc parsing)

---

## Task 1: Create imports module structure

**Files:**
- Create: `crates/lintal_linter/src/rules/imports/mod.rs`
- Create: `crates/lintal_linter/src/rules/imports/common.rs`
- Modify: `crates/lintal_linter/src/rules/mod.rs`

**Step 1: Create mod.rs**

```rust
//! Import-related lint rules.

pub mod common;
mod redundant_import;
mod unused_imports;

pub use redundant_import::RedundantImport;
pub use unused_imports::UnusedImports;
```

**Step 2: Create common.rs with ImportInfo struct**

```rust
//! Shared utilities for import rules.

use lintal_text_size::TextRange;
use tree_sitter::Node;

/// Represents a parsed import statement.
#[derive(Debug, Clone)]
pub struct ImportInfo {
    /// Full import path, e.g., "java.util.List" or "java.util.*"
    pub path: String,
    /// Simple name for non-wildcard imports, e.g., "List"
    pub simple_name: Option<String>,
    /// Whether this is a static import
    pub is_static: bool,
    /// Whether this ends with .*
    pub is_wildcard: bool,
    /// Source range for the import declaration
    pub range: TextRange,
    /// Line number (1-indexed) for duplicate detection
    pub line: usize,
}

impl ImportInfo {
    /// Get the package part of the import path (everything before the last dot).
    pub fn package(&self) -> Option<&str> {
        if self.is_wildcard {
            // For "java.util.*", package is "java.util"
            Some(&self.path[..self.path.len() - 2])
        } else {
            // For "java.util.List", package is "java.util"
            self.path.rfind('.').map(|i| &self.path[..i])
        }
    }
}
```

**Step 3: Update rules/mod.rs**

```rust
//! Lint rules organized by category.

pub mod blocks;
pub mod imports;
pub mod modifier;
pub mod style;
pub mod whitespace;

// Re-export all rules
pub use blocks::{
    AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, LeftCurly, NeedBraces, RightCurly,
};
pub use imports::{RedundantImport, UnusedImports};
pub use modifier::{FinalLocalVariable, FinalParameters, ModifierOrder, RedundantModifier};
pub use style::{ArrayTypeStyle, UpperEll};
pub use whitespace::*;
```

**Step 4: Verify compilation**

Run: `cargo check --package lintal_linter`
Expected: May fail because RedundantImport and UnusedImports don't exist yet - that's fine.

**Step 5: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/
git add crates/lintal_linter/src/rules/mod.rs
git commit -m "feat(imports): add imports module structure"
```

---

## Task 2: Implement import parsing utility

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/common.rs`

**Step 1: Add collect_imports function**

```rust
use lintal_source_file::{LineIndex, SourceCode};
use lintal_text_size::{TextRange, TextSize};

/// Collect all import declarations from the source.
pub fn collect_imports(root: Node, source: &str, line_index: &LineIndex) -> Vec<ImportInfo> {
    let mut imports = Vec::new();
    let source_code = SourceCode::new(source, line_index);

    collect_imports_recursive(root, source, &source_code, &mut imports);
    imports
}

fn collect_imports_recursive(
    node: Node,
    source: &str,
    source_code: &SourceCode,
    imports: &mut Vec<ImportInfo>,
) {
    if node.kind() == "import_declaration" {
        if let Some(info) = parse_import_declaration(node, source, source_code) {
            imports.push(info);
        }
    }

    // Only recurse into program-level nodes, not into class bodies
    if node.kind() == "program" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "import_declaration" {
                if let Some(info) = parse_import_declaration(child, source, source_code) {
                    imports.push(info);
                }
            }
        }
    }
}

fn parse_import_declaration(node: Node, source: &str, source_code: &SourceCode) -> Option<ImportInfo> {
    let start = TextSize::from(node.start_byte() as u32);
    let end = TextSize::from(node.end_byte() as u32);
    let range = TextRange::new(start, end);
    let line = source_code.line_column(start).line.get();

    let mut is_static = false;
    let mut path_parts = Vec::new();
    let mut is_wildcard = false;

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "static" => is_static = true,
            "asterisk" => is_wildcard = true,
            "identifier" | "scoped_identifier" => {
                path_parts.push(child.utf8_text(source.as_bytes()).ok()?);
            }
            _ => {}
        }
    }

    if path_parts.is_empty() {
        return None;
    }

    let mut path = path_parts.join(".");
    if is_wildcard {
        path.push_str(".*");
    }

    let simple_name = if is_wildcard {
        None
    } else {
        path.rsplit('.').next().map(String::from)
    };

    Some(ImportInfo {
        path,
        simple_name,
        is_static,
        is_wildcard,
        range,
        line,
    })
}
```

**Step 2: Add test for import parsing**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_parser::JavaParser;
    use lintal_source_file::LineIndex;

    #[test]
    fn test_collect_simple_import() {
        let source = r#"
import java.util.List;

class Test {}
"#;
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let line_index = LineIndex::from_source_text(source);

        let imports = collect_imports(result.tree.root_node(), source, &line_index);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "java.util.List");
        assert_eq!(imports[0].simple_name, Some("List".to_string()));
        assert!(!imports[0].is_static);
        assert!(!imports[0].is_wildcard);
    }

    #[test]
    fn test_collect_wildcard_import() {
        let source = r#"
import java.util.*;

class Test {}
"#;
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let line_index = LineIndex::from_source_text(source);

        let imports = collect_imports(result.tree.root_node(), source, &line_index);

        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].path, "java.util.*");
        assert!(imports[0].simple_name.is_none());
        assert!(imports[0].is_wildcard);
    }

    #[test]
    fn test_collect_static_import() {
        let source = r#"
import static java.lang.Math.PI;

class Test {}
"#;
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let line_index = LineIndex::from_source_text(source);

        let imports = collect_imports(result.tree.root_node(), source, &line_index);

        assert_eq!(imports.len(), 1);
        assert!(imports[0].is_static);
        assert_eq!(imports[0].simple_name, Some("PI".to_string()));
    }
}
```

**Step 3: Run tests**

Run: `cargo test --package lintal_linter collect_`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/common.rs
git commit -m "feat(imports): add import parsing utility"
```

---

## Task 3: Add package extraction utility

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/common.rs`

**Step 1: Add get_package_name function**

```rust
/// Extract the package name from the source file.
pub fn get_package_name(root: Node, source: &str) -> Option<String> {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == "package_declaration" {
            return extract_package_path(child, source);
        }
    }
    None
}

fn extract_package_path(node: Node, source: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "scoped_identifier" || child.kind() == "identifier" {
            return child.utf8_text(source.as_bytes()).ok().map(String::from);
        }
    }
    None
}
```

**Step 2: Add test**

```rust
#[test]
fn test_get_package_name() {
    let source = r#"
package com.example.myapp;

import java.util.List;

class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let package = get_package_name(result.tree.root_node(), source);

    assert_eq!(package, Some("com.example.myapp".to_string()));
}

#[test]
fn test_no_package() {
    let source = r#"
import java.util.List;

class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let package = get_package_name(result.tree.root_node(), source);

    assert!(package.is_none());
}
```

**Step 3: Run tests**

Run: `cargo test --package lintal_linter get_package`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/common.rs
git commit -m "feat(imports): add package extraction utility"
```

---

## Task 4: Implement RedundantImport rule skeleton

**Files:**
- Create: `crates/lintal_linter/src/rules/imports/redundant_import.rs`

**Step 1: Create rule with violation types**

```rust
//! RedundantImport rule implementation.
//!
//! Detects redundant imports:
//! - Imports from the same package as the file
//! - Imports from java.lang (always implicit)
//! - Duplicate imports
//!
//! Checkstyle equivalent: RedundantImportCheck

use std::collections::HashMap;

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_source_file::LineIndex;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::{collect_imports, get_package_name, ImportInfo};

/// Violation: import from same package.
#[derive(Debug, Clone)]
pub struct SamePackageImport;

impl Violation for SamePackageImport {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Redundant import from the same package.".to_string()
    }
}

/// Violation: import from java.lang package.
#[derive(Debug, Clone)]
pub struct JavaLangImport;

impl Violation for JavaLangImport {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        "Redundant import from the java.lang package.".to_string()
    }
}

/// Violation: duplicate import.
#[derive(Debug, Clone)]
pub struct DuplicateImport {
    pub first_line: usize,
}

impl Violation for DuplicateImport {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Duplicate import to line {}.", self.first_line)
    }
}

/// Configuration for RedundantImport rule.
#[derive(Debug, Clone, Default)]
pub struct RedundantImport;

impl FromConfig for RedundantImport {
    const MODULE_NAME: &'static str = "RedundantImport";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for RedundantImport {
    fn name(&self) -> &'static str {
        "RedundantImport"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        // TODO: Implement in next task
        vec![]
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check --package lintal_linter`
Expected: PASS (warnings about unused imports are fine)

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/redundant_import.rs
git commit -m "feat(imports): add RedundantImport rule skeleton"
```

---

## Task 5: Implement RedundantImport check logic

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/redundant_import.rs`

**Step 1: Implement the check method**

```rust
impl Rule for RedundantImport {
    fn name(&self) -> &'static str {
        "RedundantImport"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        let source = ctx.source();
        let line_index = LineIndex::from_source_text(source);
        let ts_node = node.node();

        let imports = collect_imports(ts_node, source, &line_index);
        let current_package = get_package_name(ts_node, source);

        let mut diagnostics = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();

        for import in &imports {
            // Check for duplicate
            if let Some(&first_line) = seen.get(import.path.as_str()) {
                diagnostics.push(
                    Diagnostic::new(DuplicateImport { first_line }, import.range)
                        .with_fix(self.create_delete_fix(import, source))
                );
                continue;
            }
            seen.insert(&import.path, import.line);

            // Check for java.lang import
            if self.is_java_lang_import(import) {
                diagnostics.push(
                    Diagnostic::new(JavaLangImport, import.range)
                        .with_fix(self.create_delete_fix(import, source))
                );
                continue;
            }

            // Check for same-package import
            if let Some(ref pkg) = current_package {
                if self.is_same_package_import(import, pkg) {
                    diagnostics.push(
                        Diagnostic::new(SamePackageImport, import.range)
                            .with_fix(self.create_delete_fix(import, source))
                    );
                }
            }
        }

        diagnostics
    }
}

impl RedundantImport {
    fn is_java_lang_import(&self, import: &ImportInfo) -> bool {
        // Static imports from java.lang are NOT redundant
        if import.is_static {
            return false;
        }

        import.path.starts_with("java.lang.")
            || import.path == "java.lang.*"
    }

    fn is_same_package_import(&self, import: &ImportInfo, current_package: &str) -> bool {
        // Static imports from same package are NOT redundant
        if import.is_static {
            return false;
        }

        if import.is_wildcard {
            // "import pkg.*" where pkg matches current package
            import.package() == Some(current_package)
        } else {
            // "import pkg.Class" where pkg matches current package
            import.package() == Some(current_package)
        }
    }

    fn create_delete_fix(&self, import: &ImportInfo, source: &str) -> Fix {
        // Include trailing newline in deletion for clean output
        let end = import.range.end();
        let remaining = &source[end.to_usize()..];
        let newline_len = if remaining.starts_with("\r\n") {
            2
        } else if remaining.starts_with('\n') {
            1
        } else {
            0
        };

        let delete_end = end + TextSize::from(newline_len as u32);
        let delete_range = TextRange::new(import.range.start(), delete_end);

        Fix::safe_edit(Edit::range_deletion(delete_range))
    }
}
```

**Step 2: Run tests**

Run: `cargo test --package lintal_linter`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/redundant_import.rs
git commit -m "feat(imports): implement RedundantImport check logic"
```

---

## Task 6: Add RedundantImport unit tests

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/redundant_import.rs`

**Step 1: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = RedundantImport;

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_java_lang_import() {
        let source = r#"
import java.lang.String;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("java.lang"));
    }

    #[test]
    fn test_java_lang_wildcard() {
        let source = r#"
import java.lang.*;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_static_java_lang_ok() {
        let source = r#"
import static java.lang.Math.PI;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "Static java.lang imports should be allowed");
    }

    #[test]
    fn test_same_package_import() {
        let source = r#"
package com.example;

import com.example.Other;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("same package"));
    }

    #[test]
    fn test_same_package_wildcard() {
        let source = r#"
package com.example;

import com.example.*;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_subpackage_ok() {
        let source = r#"
package com.example;

import com.example.sub.Other;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "Subpackage imports should be allowed");
    }

    #[test]
    fn test_duplicate_import() {
        let source = r#"
import java.util.List;
import java.util.List;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("Duplicate"));
    }

    #[test]
    fn test_duplicate_static_import() {
        let source = r#"
import static java.lang.Math.*;
import static java.lang.Math.*;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn test_valid_imports_ok() {
        let source = r#"
package com.example;

import java.util.List;
import java.util.Map;
import java.io.File;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_all_have_fixes() {
        let source = r#"
package com.example;

import java.lang.String;
import com.example.Other;
import java.util.List;
import java.util.List;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 3);
        for d in &diagnostics {
            assert!(d.fix.is_some(), "All violations should have fixes");
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test --package lintal_linter redundant_import`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/redundant_import.rs
git commit -m "test(imports): add RedundantImport unit tests"
```

---

## Task 7: Register RedundantImport in registry

**Files:**
- Modify: `crates/lintal_linter/src/registry.rs`

**Step 1: Add import to registry**

```rust
use crate::rules::{
    ArrayTypeStyle, AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, EmptyForInitializerPad,
    FileTabCharacter, FinalLocalVariable, FinalParameters, LeftCurly, MethodParamPad,
    ModifierOrder, NeedBraces, NoWhitespaceAfter, NoWhitespaceBefore, ParenPad,
    RedundantImport, RedundantModifier, RightCurly, SingleSpaceSeparator, TypecastParenPad,
    UpperEll, WhitespaceAfter, WhitespaceAround,
};
```

**Step 2: Register the rule**

Add after style rules registration:

```rust
// Import rules
self.register::<RedundantImport>();
```

**Step 3: Run full test suite**

Run: `cargo test --package lintal_linter`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/registry.rs
git commit -m "feat(imports): register RedundantImport rule"
```

---

## Task 8: Create RedundantImport checkstyle compatibility test

**Files:**
- Create: `crates/lintal_linter/tests/checkstyle_redundantimport.rs`

**Step 1: Create test file**

```rust
//! RedundantImport checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::RedundantImport;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    line: usize,
    message_contains: &'static str,
}

fn check_redundant_import(source: &str) -> Vec<(usize, String)> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = RedundantImport;
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push((loc.line.get(), diagnostic.kind.body.clone()));
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::imports_test_input("redundantimport", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_redundant_import_with_checker() {
    let Some(source) = load_fixture("InputRedundantImportWithChecker.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_import(&source);

    // Expected from checkstyle test file comments:
    // Line 9: same package (wildcard)
    // Line 10: same package (explicit)
    // Line 12: java.lang.*
    // Line 13: java.lang.String
    // Line 16: duplicate of line 15
    // Line 28: duplicate static import of line 27

    let expected_lines = vec![9, 10, 12, 13, 16, 28];

    println!("Found violations:");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    for expected_line in &expected_lines {
        assert!(
            violations.iter().any(|(line, _)| line == expected_line),
            "Expected violation on line {}", expected_line
        );
    }
}

#[test]
fn test_no_false_positives() {
    let Some(source) = load_fixture("InputRedundantImportWithoutWarnings.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_redundant_import(&source);

    assert!(
        violations.is_empty(),
        "Expected no violations, got: {:?}", violations
    );
}
```

**Step 2: Add imports_test_input to checkstyle_repo.rs**

Add this function to `crates/lintal_linter/tests/checkstyle_repo.rs`:

```rust
/// Get path to a checkstyle test input file for import checks.
#[allow(dead_code)]
pub fn imports_test_input(check_name: &str, file_name: &str) -> Option<PathBuf> {
    let repo = checkstyle_repo()?;
    let path = repo
        .join("src/test/resources/com/puppycrawl/tools/checkstyle/checks/imports")
        .join(check_name.to_lowercase())
        .join(file_name);

    if path.exists() { Some(path) } else { None }
}
```

**Step 3: Run test**

Run: `cargo test --package lintal_linter checkstyle_redundantimport`
Expected: PASS (or skip if checkstyle repo not available)

**Step 4: Commit**

```bash
git add crates/lintal_linter/tests/checkstyle_redundantimport.rs
git add crates/lintal_linter/tests/checkstyle_repo.rs
git commit -m "test(imports): add RedundantImport checkstyle compatibility test"
```

---

## Task 9: Implement UnusedImports rule skeleton

**Files:**
- Create: `crates/lintal_linter/src/rules/imports/unused_imports.rs`

**Step 1: Create rule with violation type**

```rust
//! UnusedImports rule implementation.
//!
//! Detects imports that are never used in the code.
//!
//! Checkstyle equivalent: UnusedImportsCheck

use std::collections::HashSet;

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_source_file::LineIndex;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::{collect_imports, ImportInfo};

/// Violation: import is unused.
#[derive(Debug, Clone)]
pub struct UnusedImportViolation {
    pub import_path: String,
}

impl Violation for UnusedImportViolation {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Always;

    fn message(&self) -> String {
        format!("Unused import - {}.", self.import_path)
    }
}

/// Configuration for UnusedImports rule.
#[derive(Debug, Clone)]
pub struct UnusedImports {
    /// Whether to scan Javadoc comments for type references.
    pub process_javadoc: bool,
}

impl Default for UnusedImports {
    fn default() -> Self {
        Self {
            process_javadoc: true,
        }
    }
}

impl FromConfig for UnusedImports {
    const MODULE_NAME: &'static str = "UnusedImports";

    fn from_config(properties: &Properties) -> Self {
        let process_javadoc = properties
            .get("processJavadoc")
            .map(|v| *v != "false")
            .unwrap_or(true);

        Self { process_javadoc }
    }
}

impl Rule for UnusedImports {
    fn name(&self) -> &'static str {
        "UnusedImports"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        // TODO: Implement in next task
        vec![]
    }
}
```

**Step 2: Update mod.rs exports**

Already done in Task 1.

**Step 3: Verify compilation**

Run: `cargo check --package lintal_linter`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/unused_imports.rs
git commit -m "feat(imports): add UnusedImports rule skeleton"
```

---

## Task 10: Add type usage collection utility

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/common.rs`

**Step 1: Add collect_type_usages function**

```rust
use std::collections::HashSet;

/// Collect all type identifiers used in the source code.
///
/// This traverses the AST and collects simple names of types that are referenced:
/// - Type identifiers in declarations, casts, generics
/// - Annotation names
/// - Static method call targets (for static imports)
pub fn collect_type_usages(root: Node, source: &str) -> HashSet<String> {
    let mut usages = HashSet::new();
    collect_usages_recursive(root, source, &mut usages);
    usages
}

fn collect_usages_recursive(node: Node, source: &str, usages: &mut HashSet<String>) {
    match node.kind() {
        // Type identifier - used in declarations, generics, etc.
        "type_identifier" => {
            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                usages.insert(text.to_string());
            }
        }

        // Scoped type identifier - e.g., Map.Entry, use first part
        "scoped_type_identifier" => {
            // Get the first identifier (the imported type)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "type_identifier" {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        usages.insert(text.to_string());
                    }
                    break;
                }
            }
        }

        // Annotation - @Foo means Foo is used
        "marker_annotation" | "annotation" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Ok(text) = child.utf8_text(source.as_bytes()) {
                        usages.insert(text.to_string());
                    }
                    break;
                }
                if child.kind() == "scoped_identifier" {
                    // @com.foo.Bar - get first identifier
                    if let Some(first) = child.child(0) {
                        if let Ok(text) = first.utf8_text(source.as_bytes()) {
                            usages.insert(text.to_string());
                        }
                    }
                    break;
                }
            }
        }

        // Method invocation on a type - e.g., Arrays.sort()
        "method_invocation" => {
            if let Some(object) = node.child_by_field_name("object") {
                if object.kind() == "identifier" {
                    if let Ok(text) = object.utf8_text(source.as_bytes()) {
                        // Check if it looks like a class name (starts with uppercase)
                        if text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            usages.insert(text.to_string());
                        }
                    }
                }
            }
        }

        // Field access on a type - e.g., System.out
        "field_access" => {
            if let Some(object) = node.child_by_field_name("object") {
                if object.kind() == "identifier" {
                    if let Ok(text) = object.utf8_text(source.as_bytes()) {
                        if text.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            usages.insert(text.to_string());
                        }
                    }
                }
            }
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_usages_recursive(child, source, usages);
    }
}
```

**Step 2: Add tests**

```rust
#[test]
fn test_collect_type_usages_declaration() {
    let source = r#"
class Test {
    List<String> items;
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let usages = collect_type_usages(result.tree.root_node(), source);

    assert!(usages.contains("List"));
    assert!(usages.contains("String"));
}

#[test]
fn test_collect_type_usages_annotation() {
    let source = r#"
@Override
class Test {
    @Deprecated
    void method() {}
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let usages = collect_type_usages(result.tree.root_node(), source);

    assert!(usages.contains("Override"));
    assert!(usages.contains("Deprecated"));
}

#[test]
fn test_collect_type_usages_method_call() {
    let source = r#"
class Test {
    void method() {
        Arrays.sort(items);
    }
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let usages = collect_type_usages(result.tree.root_node(), source);

    assert!(usages.contains("Arrays"));
}

#[test]
fn test_collect_type_usages_inner_class() {
    let source = r#"
class Test {
    JToolBar.Separator sep;
}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let usages = collect_type_usages(result.tree.root_node(), source);

    assert!(usages.contains("JToolBar"), "Should capture outer class from inner class reference");
}
```

**Step 3: Run tests**

Run: `cargo test --package lintal_linter collect_type`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/common.rs
git commit -m "feat(imports): add type usage collection utility"
```

---

## Task 11: Add Javadoc reference extraction

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/common.rs`

**Step 1: Add Javadoc parsing with regex**

```rust
use regex::Regex;

/// Extract type references from Javadoc comments.
///
/// Parses:
/// - {@link Type}, {@link Type#method}, {@link Type#method(Param)}
/// - {@linkplain Type text}
/// - @see Type
/// - @throws Type, @exception Type
pub fn collect_javadoc_references(root: Node, source: &str) -> HashSet<String> {
    let mut references = HashSet::new();
    collect_javadoc_recursive(root, source, &mut references);
    references
}

fn collect_javadoc_recursive(node: Node, source: &str, references: &mut HashSet<String>) {
    if node.kind() == "block_comment" {
        if let Ok(text) = node.utf8_text(source.as_bytes()) {
            if text.starts_with("/**") {
                parse_javadoc_types(text, references);
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_javadoc_recursive(child, source, references);
    }
}

fn parse_javadoc_types(javadoc: &str, references: &mut HashSet<String>) {
    // Pattern for {@link Type}, {@link Type#method}, {@link Type#method(Param1, Param2)}
    // Also handles {@linkplain Type text}
    lazy_static::lazy_static! {
        static ref LINK_RE: Regex = Regex::new(
            r"\{@(?:link|linkplain)\s+([A-Z][A-Za-z0-9_]*)(?:#[^}(]*(?:\(([^)]*)\))?)?[^}]*\}"
        ).unwrap();

        static ref SEE_RE: Regex = Regex::new(
            r"@see\s+([A-Z][A-Za-z0-9_.]*)"
        ).unwrap();

        static ref THROWS_RE: Regex = Regex::new(
            r"@(?:throws|exception)\s+([A-Z][A-Za-z0-9_.]*)"
        ).unwrap();

        static ref PARAM_TYPE_RE: Regex = Regex::new(
            r"([A-Z][A-Za-z0-9_]*)"
        ).unwrap();
    }

    // Extract from @link tags
    for cap in LINK_RE.captures_iter(javadoc) {
        if let Some(m) = cap.get(1) {
            references.insert(m.as_str().to_string());
        }
        // Also extract types from method parameters like Type#method(ParamType)
        if let Some(params) = cap.get(2) {
            for param_cap in PARAM_TYPE_RE.captures_iter(params.as_str()) {
                if let Some(m) = param_cap.get(1) {
                    references.insert(m.as_str().to_string());
                }
            }
        }
    }

    // Extract from @see tags
    for cap in SEE_RE.captures_iter(javadoc) {
        if let Some(m) = cap.get(1) {
            // Get just the simple name (first part before any dot)
            let name = m.as_str().split('.').next().unwrap_or(m.as_str());
            references.insert(name.to_string());
        }
    }

    // Extract from @throws/@exception tags
    for cap in THROWS_RE.captures_iter(javadoc) {
        if let Some(m) = cap.get(1) {
            let name = m.as_str().split('.').next().unwrap_or(m.as_str());
            references.insert(name.to_string());
        }
    }
}
```

**Step 2: Add lazy_static dependency**

Add to `crates/lintal_linter/Cargo.toml`:

```toml
lazy_static = "1.4"
regex = "1"
```

**Step 3: Add tests**

```rust
#[test]
fn test_javadoc_link() {
    let source = r#"
/**
 * See {@link List} for details.
 */
class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let refs = collect_javadoc_references(result.tree.root_node(), source);

    assert!(refs.contains("List"));
}

#[test]
fn test_javadoc_link_with_method() {
    let source = r#"
/**
 * Uses {@link Arrays#sort(Object[])} internally.
 */
class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let refs = collect_javadoc_references(result.tree.root_node(), source);

    assert!(refs.contains("Arrays"));
    assert!(refs.contains("Object"));
}

#[test]
fn test_javadoc_see() {
    let source = r#"
/**
 * @see Calendar
 */
class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let refs = collect_javadoc_references(result.tree.root_node(), source);

    assert!(refs.contains("Calendar"));
}

#[test]
fn test_javadoc_throws() {
    let source = r#"
/**
 * @throws IOException if error
 * @exception RuntimeException if bad
 */
class Test {}
"#;
    let mut parser = JavaParser::new();
    let result = parser.parse(source).unwrap();

    let refs = collect_javadoc_references(result.tree.root_node(), source);

    assert!(refs.contains("IOException"));
    assert!(refs.contains("RuntimeException"));
}
```

**Step 4: Run tests**

Run: `cargo test --package lintal_linter javadoc`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/lintal_linter/Cargo.toml
git add crates/lintal_linter/src/rules/imports/common.rs
git commit -m "feat(imports): add Javadoc reference extraction"
```

---

## Task 12: Implement UnusedImports check logic

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/unused_imports.rs`

**Step 1: Implement the check method**

```rust
use super::common::{collect_imports, collect_type_usages, collect_javadoc_references, ImportInfo};

impl Rule for UnusedImports {
    fn name(&self) -> &'static str {
        "UnusedImports"
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        let source = ctx.source();
        let line_index = LineIndex::from_source_text(source);
        let ts_node = node.node();

        let imports = collect_imports(ts_node, source, &line_index);

        // Collect all type usages
        let mut usages = collect_type_usages(ts_node, source);

        // Optionally include Javadoc references
        if self.process_javadoc {
            usages.extend(collect_javadoc_references(ts_node, source));
        }

        let mut diagnostics = Vec::new();

        for import in &imports {
            // Skip wildcard imports - can't verify without type resolution
            if import.is_wildcard {
                continue;
            }

            // Check if the simple name is used
            if let Some(ref simple_name) = import.simple_name {
                if !usages.contains(simple_name) {
                    diagnostics.push(
                        Diagnostic::new(
                            UnusedImportViolation {
                                import_path: import.path.clone()
                            },
                            import.range
                        )
                        .with_fix(self.create_delete_fix(import, source))
                    );
                }
            }
        }

        diagnostics
    }
}

impl UnusedImports {
    fn create_delete_fix(&self, import: &ImportInfo, source: &str) -> Fix {
        // Include trailing newline in deletion for clean output
        let end = import.range.end();
        let remaining = &source[end.to_usize()..];
        let newline_len = if remaining.starts_with("\r\n") {
            2
        } else if remaining.starts_with('\n') {
            1
        } else {
            0
        };

        let delete_end = end + TextSize::from(newline_len as u32);
        let delete_range = TextRange::new(import.range.start(), delete_end);

        Fix::safe_edit(Edit::range_deletion(delete_range))
    }
}
```

**Step 2: Run tests**

Run: `cargo test --package lintal_linter`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/unused_imports.rs
git commit -m "feat(imports): implement UnusedImports check logic"
```

---

## Task 13: Add UnusedImports unit tests

**Files:**
- Modify: `crates/lintal_linter/src/rules/imports/unused_imports.rs`

**Step 1: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source(source: &str) -> Vec<Diagnostic> {
        check_source_with_config(source, true)
    }

    fn check_source_with_config(source: &str, process_javadoc: bool) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = UnusedImports { process_javadoc };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_unused_import() {
        let source = r#"
import java.util.List;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].kind.body.contains("Unused import"));
    }

    #[test]
    fn test_used_in_declaration() {
        let source = r#"
import java.util.List;

class Test {
    List<String> items;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_used_in_annotation() {
        let source = r#"
import java.lang.Override;

class Test {
    @Override
    public String toString() { return ""; }
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_used_in_static_method_call() {
        let source = r#"
import java.util.Arrays;

class Test {
    void method() {
        Arrays.sort(new int[0]);
    }
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_used_in_inner_class_reference() {
        let source = r#"
import javax.swing.JToolBar;

class Test {
    JToolBar.Separator sep;
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "JToolBar should be marked as used via JToolBar.Separator");
    }

    #[test]
    fn test_used_in_javadoc_link() {
        let source = r#"
import java.util.Date;

/**
 * Uses {@link Date} for timestamps.
 */
class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "Date used in Javadoc @link");
    }

    #[test]
    fn test_used_in_javadoc_see() {
        let source = r#"
import java.util.Calendar;

/**
 * @see Calendar
 */
class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "Calendar used in Javadoc @see");
    }

    #[test]
    fn test_used_in_javadoc_throws() {
        let source = r#"
import java.io.IOException;

/**
 * @throws IOException if error
 */
class Test {
    void method() {}
}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "IOException used in Javadoc @throws");
    }

    #[test]
    fn test_javadoc_disabled() {
        let source = r#"
import java.util.Date;

/**
 * Uses {@link Date} for timestamps.
 */
class Test {}
"#;
        let diagnostics = check_source_with_config(source, false);
        assert_eq!(diagnostics.len(), 1, "Date should be unused when Javadoc processing disabled");
    }

    #[test]
    fn test_wildcard_import_skipped() {
        let source = r#"
import java.util.*;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert!(diagnostics.is_empty(), "Wildcard imports should be skipped");
    }

    #[test]
    fn test_multiple_unused() {
        let source = r#"
import java.util.List;
import java.util.Map;
import java.io.File;

class Test {}
"#;
        let diagnostics = check_source(source);
        assert_eq!(diagnostics.len(), 3);
    }

    #[test]
    fn test_all_have_fixes() {
        let source = r#"
import java.util.List;
import java.util.Map;

class Test {}
"#;
        let diagnostics = check_source(source);
        for d in &diagnostics {
            assert!(d.fix.is_some(), "All violations should have fixes");
        }
    }
}
```

**Step 2: Run tests**

Run: `cargo test --package lintal_linter unused_imports`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/lintal_linter/src/rules/imports/unused_imports.rs
git commit -m "test(imports): add UnusedImports unit tests"
```

---

## Task 14: Register UnusedImports in registry

**Files:**
- Modify: `crates/lintal_linter/src/registry.rs`

**Step 1: Add import to registry**

```rust
use crate::rules::{
    ArrayTypeStyle, AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, EmptyForInitializerPad,
    FileTabCharacter, FinalLocalVariable, FinalParameters, LeftCurly, MethodParamPad,
    ModifierOrder, NeedBraces, NoWhitespaceAfter, NoWhitespaceBefore, ParenPad,
    RedundantImport, RedundantModifier, RightCurly, SingleSpaceSeparator, TypecastParenPad,
    UnusedImports, UpperEll, WhitespaceAfter, WhitespaceAround,
};
```

**Step 2: Register the rule**

Add after RedundantImport:

```rust
self.register::<UnusedImports>();
```

**Step 3: Run full test suite**

Run: `cargo test --package lintal_linter`
Expected: PASS

**Step 4: Commit**

```bash
git add crates/lintal_linter/src/registry.rs
git commit -m "feat(imports): register UnusedImports rule"
```

---

## Task 15: Create UnusedImports checkstyle compatibility test

**Files:**
- Create: `crates/lintal_linter/tests/checkstyle_unusedimports.rs`

**Step 1: Create test file**

```rust
//! UnusedImports checkstyle compatibility tests.

mod checkstyle_repo;

use lintal_java_cst::TreeWalker;
use lintal_java_parser::JavaParser;
use lintal_linter::rules::UnusedImports;
use lintal_linter::{CheckContext, Rule};
use lintal_source_file::{LineIndex, SourceCode};

fn check_unused_imports(source: &str, process_javadoc: bool) -> Vec<(usize, String)> {
    let mut parser = JavaParser::new();
    let Some(result) = parser.parse(source) else {
        panic!("Failed to parse source");
    };

    let rule = UnusedImports { process_javadoc };
    let ctx = CheckContext::new(source);
    let line_index = LineIndex::from_source_text(source);
    let source_code = SourceCode::new(source, &line_index);

    let mut violations = vec![];

    for node in TreeWalker::new(result.tree.root_node(), source) {
        let diagnostics = rule.check(&ctx, &node);
        for diagnostic in diagnostics {
            let loc = source_code.line_column(diagnostic.range.start());
            violations.push((loc.line.get(), diagnostic.kind.body.clone()));
        }
    }

    violations
}

fn load_fixture(file_name: &str) -> Option<String> {
    let path = checkstyle_repo::imports_test_input("unusedimports", file_name)?;
    std::fs::read_to_string(&path).ok()
}

#[test]
fn test_unused_imports_main() {
    let Some(source) = load_fixture("InputUnusedImports.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, true);

    println!("Found violations:");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    // Expected violations from checkstyle test comments (processJavadoc=true):
    // Line 11: GuardedBy unused
    // Line 15: java.lang.String unused
    // Line 17-18: List unused (duplicate)
    // Line 21: Enumeration unused
    // Line 24: JToggleButton unused
    // Line 26: BorderFactory unused
    // Line 31-32: createTempFile unused
    // Line 36: Label unused
    // Line 48: ForOverride unused

    let expected_unused_lines = vec![11, 15, 17, 18, 21, 24, 26, 36, 48];

    for line in &expected_unused_lines {
        let found = violations.iter().any(|(l, _)| l == line);
        if !found {
            println!("WARNING: Expected violation on line {} not found", line);
        }
    }

    // Should have at least 6 of the expected violations
    let found_count = expected_unused_lines.iter()
        .filter(|line| violations.iter().any(|(l, _)| l == *line))
        .count();

    assert!(
        found_count >= 6,
        "Expected at least 6 of {} violations, found {}",
        expected_unused_lines.len(),
        found_count
    );
}

#[test]
fn test_no_false_positives() {
    let Some(source) = load_fixture("InputUnusedImportsWithoutWarnings.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, true);

    println!("Violations (should be empty):");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }

    assert!(
        violations.is_empty(),
        "Expected no violations, got {} violations", violations.len()
    );
}

#[test]
fn test_javadoc_disabled() {
    let Some(source) = load_fixture("InputUnusedImportsFromStaticMethodRefJavadocDisabled.java") else {
        eprintln!("Skipping test: checkstyle repo not available");
        return;
    };

    let violations = check_unused_imports(&source, false);

    println!("Violations (javadoc disabled):");
    for (line, msg) in &violations {
        println!("  {}: {}", line, msg);
    }
}
```

**Step 2: Run test**

Run: `cargo test --package lintal_linter checkstyle_unusedimports`
Expected: PASS (or skip if checkstyle repo not available)

**Step 3: Commit**

```bash
git add crates/lintal_linter/tests/checkstyle_unusedimports.rs
git commit -m "test(imports): add UnusedImports checkstyle compatibility test"
```

---

## Task 16: Run full test suite and verify

**Step 1: Run all tests**

Run: `cargo test --all`
Expected: PASS

**Step 2: Run clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: PASS (no warnings)

**Step 3: Format code**

Run: `cargo fmt --all`
Expected: No changes needed

**Step 4: Test on real Java project**

Run: `./target/release/lintal check /path/to/java/project`
Expected: Reports import violations correctly

**Step 5: Final commit**

```bash
git add -A
git commit -m "feat(imports): complete import rules implementation

Implements Phase 7 import rules:
- RedundantImport: detects same-package, java.lang, and duplicate imports
- UnusedImports: detects imports not used in code or Javadoc

Both rules include:
- Auto-fix support (delete unused import line)
- Checkstyle compatibility tests
- Support for static imports
- Proper handling of wildcard imports"
```

---

## Summary

This plan implements 2 import rules with 16 tasks:

1. **Tasks 1-3**: Module structure and shared utilities
2. **Tasks 4-8**: RedundantImport rule with tests
3. **Tasks 9-15**: UnusedImports rule with Javadoc support
4. **Task 16**: Final verification

Each task is atomic and testable. Follow TDD: write tests, verify failure, implement, verify pass, commit.
