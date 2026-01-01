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

use super::common::{ImportInfo, collect_imports, get_package_name};

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

const RELEVANT_KINDS: &[&str] = &["program"];

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

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        let source = ctx.source();
        let line_index = LineIndex::from_source_text(source);
        let ts_node = node.inner();

        let imports = collect_imports(ts_node, source, &line_index);
        let current_package = get_package_name(ts_node, source);

        let mut diagnostics = Vec::new();
        let mut seen: HashMap<&str, usize> = HashMap::new();

        for import in &imports {
            // Check for duplicate
            if let Some(&first_line) = seen.get(import.path.as_str()) {
                diagnostics.push(
                    Diagnostic::new(DuplicateImport { first_line }, import.range)
                        .with_fix(self.create_delete_fix(import, source)),
                );
                continue;
            }
            seen.insert(&import.path, import.line);

            // Check for java.lang import
            if self.is_java_lang_import(import) {
                diagnostics.push(
                    Diagnostic::new(JavaLangImport, import.range)
                        .with_fix(self.create_delete_fix(import, source)),
                );
                continue;
            }

            // Check for same-package import
            if let Some(ref pkg) = current_package
                && self.is_same_package_import(import, pkg)
            {
                diagnostics.push(
                    Diagnostic::new(SamePackageImport, import.range)
                        .with_fix(self.create_delete_fix(import, source)),
                );
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

        // Check for wildcard: java.lang.*
        if import.path == "java.lang.*" {
            return true;
        }

        // Check for direct java.lang import (not a subpackage)
        // java.lang.String -> redundant
        // java.lang.instrument.Instrumentation -> NOT redundant (subpackage)
        if let Some(rest) = import.path.strip_prefix("java.lang.") {
            // If there's no more dots, it's directly in java.lang
            !rest.contains('.')
        } else {
            false
        }
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
        assert!(
            diagnostics.is_empty(),
            "Static java.lang imports should be allowed"
        );
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
        assert!(
            diagnostics.is_empty(),
            "Subpackage imports should be allowed"
        );
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
