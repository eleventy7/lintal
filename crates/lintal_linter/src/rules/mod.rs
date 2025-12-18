//! Lint rules organized by category.

pub mod blocks;
pub mod whitespace;

// Re-export all rules
pub use blocks::*;
pub use whitespace::*;
