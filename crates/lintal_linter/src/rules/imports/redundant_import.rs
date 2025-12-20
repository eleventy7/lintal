//! RedundantImport rule implementation - placeholder.

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

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

    fn check(&self, _ctx: &CheckContext, _node: &CstNode) -> Vec<Diagnostic> {
        vec![]
    }
}
