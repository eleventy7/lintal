//! Diagnostic and fix infrastructure for linting.
//!
//! This crate is derived from [ruff_diagnostics](https://github.com/astral-sh/ruff)
//! by Astral Software Inc., licensed under MIT.

pub use diagnostic::{Diagnostic, DiagnosticKind, FixAvailability, Violation};
pub use edit::Edit;
pub use fix::{Applicability, Fix, IsolationLevel};
pub use source_map::{SourceMap, SourceMarker};

mod diagnostic;
mod edit;
mod fix;
mod source_map;
