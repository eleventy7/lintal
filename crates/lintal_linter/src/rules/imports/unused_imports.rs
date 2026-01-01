//! UnusedImports rule implementation.
//!
//! Detects imports that are never used in the code.
//!
//! Checkstyle equivalent: UnusedImportsCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_source_file::LineIndex;
use lintal_text_size::{TextRange, TextSize};

use crate::{CheckContext, FromConfig, Properties, Rule};

use super::common::{ImportInfo, collect_imports, collect_javadoc_references, collect_type_usages};

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

const RELEVANT_KINDS: &[&str] = &["program"];

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
            if let Some(ref simple_name) = import.simple_name
                && !usages.contains(simple_name)
            {
                diagnostics.push(
                    Diagnostic::new(
                        UnusedImportViolation {
                            import_path: import.path.clone(),
                        },
                        import.range,
                    )
                    .with_fix(self.create_delete_fix(import, source)),
                );
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
        assert!(
            diagnostics.is_empty(),
            "JToolBar should be marked as used via JToolBar.Separator"
        );
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
        assert!(
            diagnostics.is_empty(),
            "IOException used in Javadoc @throws"
        );
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
        assert_eq!(
            diagnostics.len(),
            1,
            "Date should be unused when Javadoc processing disabled"
        );
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
