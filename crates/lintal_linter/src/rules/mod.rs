//! Lint rules organized by category.

pub mod blocks;
pub mod imports;
pub mod modifier;
pub mod style;
pub mod whitespace;

// Re-export all rules
pub use blocks::{
    AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, LeftCurly, NeedBraces, RightCurly,
};
pub use imports::{RedundantImport, UnusedImports};
pub use modifier::{FinalLocalVariable, FinalParameters, ModifierOrder, RedundantModifier};
pub use style::{ArrayTypeStyle, UpperEll};
pub use whitespace::*;
