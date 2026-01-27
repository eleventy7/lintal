//! Coding rules (OneStatementPerLine, MultipleVariableDeclarations, etc.)

mod default_comes_last;
mod empty_statement;
mod multiple_variable_declarations;
mod one_statement_per_line;
mod simplify_boolean_expression;
mod simplify_boolean_return;
mod string_literal_equality;

pub use default_comes_last::DefaultComesLast;
pub use empty_statement::EmptyStatement;
pub use multiple_variable_declarations::MultipleVariableDeclarations;
pub use one_statement_per_line::OneStatementPerLine;
pub use simplify_boolean_expression::SimplifyBooleanExpression;
pub use simplify_boolean_return::SimplifyBooleanReturn;
pub use string_literal_equality::StringLiteralEquality;
