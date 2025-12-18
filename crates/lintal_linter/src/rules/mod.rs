//! Lint rules organized by category.

pub mod blocks;
pub mod whitespace;

// Re-export all rules
pub use blocks::{
    AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, LeftCurly, NeedBraces, RightCurly,
};
pub use whitespace::*;
