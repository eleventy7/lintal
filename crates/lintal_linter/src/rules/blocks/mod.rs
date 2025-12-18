//! Blocks rules for checking brace placement and block structure.

pub mod avoid_nested_blocks;
pub mod common;
pub mod empty_block;
pub mod empty_catch_block;
pub mod left_curly;
pub mod need_braces;
pub mod right_curly;

pub use avoid_nested_blocks::AvoidNestedBlocks;
pub use empty_block::EmptyBlock;
pub use empty_catch_block::EmptyCatchBlock;
pub use left_curly::LeftCurly;
pub use need_braces::NeedBraces;
pub use right_curly::RightCurly;
