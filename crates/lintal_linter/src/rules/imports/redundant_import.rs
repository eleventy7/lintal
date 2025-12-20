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

    fn check(&self, _ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check at program level (once per file)
        if node.kind() != "program" {
            return vec![];
        }

        // TODO: Implement in next task
        vec![]
    }
}
