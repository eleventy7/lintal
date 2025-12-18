//! Diagnostic types for reporting violations.

use lintal_text_size::TextRange;

use crate::Fix;

/// Indicates whether a fix is available for a violation.
#[derive(Copy, Clone, Debug, Default, Hash, PartialEq, Eq)]
pub enum FixAvailability {
    /// A fix is always available.
    Always,
    /// A fix is sometimes available.
    Sometimes,
    /// A fix is never available.
    #[default]
    None,
}

/// A trait for violations that can be reported as diagnostics.
pub trait Violation: std::fmt::Debug + Clone + Send + Sync {
    /// The availability of a fix for this violation.
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::None;

    /// Returns the message describing the violation.
    fn message(&self) -> String;

    /// Returns the title for the fix, if available.
    fn fix_title(&self) -> Option<String> {
        None
    }
}

/// The kind of diagnostic (rule code and message).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticKind {
    /// The rule code (e.g., "WS001").
    pub code: String,
    /// The message body.
    pub body: String,
}

/// A diagnostic representing a violation found in source code.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// The kind of diagnostic.
    pub kind: DiagnosticKind,
    /// The range in the source where the violation occurs.
    pub range: TextRange,
    /// The optional fix for the violation.
    pub fix: Option<Fix>,
}

impl Diagnostic {
    /// Create a new diagnostic from a violation.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new<V: Violation>(violation: V, range: TextRange) -> Self {
        Self {
            kind: DiagnosticKind {
                code: std::any::type_name::<V>()
                    .split("::")
                    .last()
                    .unwrap_or("Unknown")
                    .to_string(),
                body: violation.message(),
            },
            range,
            fix: None,
        }
    }

    /// Add a fix to this diagnostic.
    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fix = Some(fix);
        self
    }

    /// Set the fix for this diagnostic.
    pub fn set_fix(&mut self, fix: Fix) {
        self.fix = Some(fix);
    }

    /// Returns true if this diagnostic has a fix.
    pub fn fixable(&self) -> bool {
        self.fix.is_some()
    }
}
