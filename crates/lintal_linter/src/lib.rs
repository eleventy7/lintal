//! Java linter with auto-fix support.

pub mod registry;
pub mod rules;
pub mod suppression;

pub use registry::{FromConfig, Properties, RuleRegistry};
pub use suppression::{FileSuppressionsConfig, PlainTextCommentFilterConfig, SuppressionContext};

use lintal_diagnostics::Diagnostic;
use lintal_java_cst::CstNode;
use lintal_source_file::{LineIndex, SourceCode};
use lintal_text_size::TextRange;

/// Context provided to rules during checking.
pub struct CheckContext<'a> {
    source: &'a str,
    line_index: LineIndex,
}

impl<'a> CheckContext<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            line_index: LineIndex::from_source_text(source),
        }
    }

    /// Get the source text.
    pub fn source(&self) -> &'a str {
        self.source
    }

    /// Get the cached line index.
    pub fn line_index(&self) -> &LineIndex {
        &self.line_index
    }

    /// Get the source code helper for line/column info.
    pub fn source_code(&self) -> SourceCode<'a, '_> {
        SourceCode::new(self.source, &self.line_index)
    }

    /// Get text at a given range.
    pub fn text_at(&self, range: TextRange) -> &'a str {
        &self.source[range]
    }

    /// Get text before a position.
    pub fn text_before(&self, pos: lintal_text_size::TextSize) -> &'a str {
        &self.source[..usize::from(pos)]
    }

    /// Get text after a position.
    pub fn text_after(&self, pos: lintal_text_size::TextSize) -> &'a str {
        &self.source[usize::from(pos)..]
    }
}

/// Trait for lint rules.
pub trait Rule: Send + Sync {
    /// The rule's name (matching checkstyle module name).
    fn name(&self) -> &'static str;

    /// Node kinds this rule cares about. Empty means run on all nodes.
    fn relevant_kinds(&self) -> &'static [&'static str] {
        &[]
    }

    /// Check a CST node for violations.
    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic>;
}

/// Result of linting a file.
#[derive(Debug, Default)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl LintResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diagnostic(mut self, diagnostic: Diagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }

    pub fn extend(&mut self, other: LintResult) {
        self.diagnostics.extend(other.diagnostics);
    }

    /// Get all fixable diagnostics.
    pub fn fixable(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter().filter(|d| d.fix.is_some())
    }
}
