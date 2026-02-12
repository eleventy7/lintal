//! Coding rules (OneStatementPerLine, MultipleVariableDeclarations, etc.)

mod covariant_equals;
mod declaration_order;
mod default_comes_last;
mod empty_statement;
mod equals_hashcode;
mod fall_through;
mod hidden_field;
mod illegal_type;
mod inner_assignment;
mod missing_switch_default;
mod multiple_variable_declarations;
mod nested_try_depth;
mod one_statement_per_line;
mod package_declaration;
mod simplify_boolean_expression;
mod simplify_boolean_return;
mod string_literal_equality;

pub use covariant_equals::CovariantEquals;
pub use declaration_order::DeclarationOrder;
pub use default_comes_last::DefaultComesLast;
pub use empty_statement::EmptyStatement;
pub use equals_hashcode::EqualsHashCode;
pub use fall_through::FallThrough;
pub use hidden_field::HiddenField;
pub use illegal_type::IllegalType;
pub use inner_assignment::InnerAssignment;
pub use missing_switch_default::MissingSwitchDefault;
pub use multiple_variable_declarations::MultipleVariableDeclarations;
pub use nested_try_depth::NestedTryDepth;
pub use one_statement_per_line::OneStatementPerLine;
pub use package_declaration::PackageDeclaration;
pub use simplify_boolean_expression::SimplifyBooleanExpression;
pub use simplify_boolean_return::SimplifyBooleanReturn;
pub use string_literal_equality::StringLiteralEquality;
