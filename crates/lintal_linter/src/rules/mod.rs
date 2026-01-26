//! Lint rules organized by category.

pub mod blocks;
pub mod coding;
pub mod imports;
pub mod modifier;
pub mod naming;
pub mod style;
pub mod whitespace;

// Re-export all rules
pub use blocks::{
    AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, LeftCurly, NeedBraces, RightCurly,
};
pub use coding::{
    EmptyStatement, MultipleVariableDeclarations, OneStatementPerLine, SimplifyBooleanExpression,
    SimplifyBooleanReturn,
};
pub use imports::{RedundantImport, UnusedImports};
pub use modifier::{
    FinalClass, FinalLocalVariable, FinalParameters, ModifierOrder, RedundantModifier,
};
pub use naming::{
    ConstantName, LocalFinalVariableName, LocalVariableName, MemberName, MethodName, PackageName,
    ParameterName, StaticVariableName, TypeName,
};
pub use style::{ArrayTypeStyle, UpperEll};
pub use whitespace::*;
