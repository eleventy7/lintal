//! Coding rules (OneStatementPerLine, MultipleVariableDeclarations, etc.)

mod empty_statement;
mod multiple_variable_declarations;
mod one_statement_per_line;
mod simplify_boolean_expression;
mod simplify_boolean_return;

pub use empty_statement::EmptyStatement;
pub use multiple_variable_declarations::MultipleVariableDeclarations;
pub use one_statement_per_line::OneStatementPerLine;
pub use simplify_boolean_expression::SimplifyBooleanExpression;
pub use simplify_boolean_return::SimplifyBooleanReturn;
