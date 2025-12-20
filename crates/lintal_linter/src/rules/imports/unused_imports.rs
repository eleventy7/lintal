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

use super::common::{collect_imports, collect_type_usages, collect_javadoc_references, ImportInfo};

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
