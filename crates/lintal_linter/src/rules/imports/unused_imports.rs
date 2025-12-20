//! UnusedImports rule implementation - placeholder.

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Configuration for UnusedImports rule.
#[derive(Debug, Clone, Default)]
pub struct UnusedImports;

impl FromConfig for UnusedImports {
    const MODULE_NAME: &'static str = "UnusedImports";

    fn from_config(_properties: &Properties) -> Self {
        Self
    }
}

impl Rule for UnusedImports {
    fn name(&self) -> &'static str {
        "UnusedImports"
    }

    fn check(&self, _ctx: &CheckContext, _node: &CstNode) -> Vec<Diagnostic> {
        vec![]
    }
}
