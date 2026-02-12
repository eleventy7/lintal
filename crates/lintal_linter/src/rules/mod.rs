//! Lint rules organized by category.

pub mod blocks;
pub mod coding;
pub mod design;
pub mod imports;
pub mod misc;
pub mod modifier;
pub mod naming;
pub mod regexp;
pub mod sizes;
pub mod style;
pub mod whitespace;

// Re-export all rules
pub use blocks::{
    AvoidNestedBlocks, EmptyBlock, EmptyCatchBlock, LeftCurly, NeedBraces, RightCurly,
};
pub use coding::{
    CovariantEquals, DeclarationOrder, DefaultComesLast, EmptyStatement, EqualsHashCode,
    FallThrough, HiddenField, IllegalType, InnerAssignment, MissingSwitchDefault,
    MultipleVariableDeclarations, NestedTryDepth, OneStatementPerLine, PackageDeclaration,
    SimplifyBooleanExpression, SimplifyBooleanReturn, StringLiteralEquality,
};
pub use design::{HideUtilityClassConstructor, MutableException};
pub use imports::{RedundantImport, UnusedImports};
pub use misc::DescendantToken;
pub use modifier::{
    FinalClass, FinalLocalVariable, FinalParameters, ModifierOrder, RedundantModifier,
};
pub use naming::{
    ConstantName, LocalFinalVariableName, LocalVariableName, MemberName, MethodName, PackageName,
    ParameterName, StaticVariableName, TypeName,
};
pub use regexp::RegexpSinglelineJava;
pub use sizes::{LineLength, MethodLength};
pub use style::{ArrayTypeStyle, UpperEll};
pub use whitespace::*;
