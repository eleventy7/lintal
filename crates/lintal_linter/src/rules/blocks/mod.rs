//! Blocks rules for checking brace placement and block structure.

pub mod common;
pub mod empty_block;
pub mod left_curly;
pub mod need_braces;
pub mod right_curly;

pub use empty_block::EmptyBlock;
pub use left_curly::LeftCurly;
pub use need_braces::NeedBraces;
pub use right_curly::RightCurly;

// Additional rules will be added as they're implemented
