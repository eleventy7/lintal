//! OperatorWrap rule implementation.
//!
//! Checks that operators are on the correct line when expressions span multiple lines.
//!
//! Checkstyle equivalent: OperatorWrapCheck

use lintal_diagnostics::{Diagnostic, Edit, Fix, FixAvailability, Violation};
use lintal_java_cst::CstNode;
use lintal_text_size::{TextRange, TextSize};
use std::collections::HashSet;

use crate::{CheckContext, FromConfig, Properties, Rule};

/// Violation: operator should be on a new line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnNewLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnNewLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on a new line.", self.operator)
    }
}

/// Violation: operator should be on the previous line.
#[derive(Debug, Clone)]
pub struct OperatorShouldBeOnPrevLine {
    pub operator: String,
}

impl Violation for OperatorShouldBeOnPrevLine {
    const FIX_AVAILABILITY: FixAvailability = FixAvailability::Sometimes;

    fn message(&self) -> String {
        format!("'{}' should be on the previous line.", self.operator)
    }
}

/// Option for where operators should be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WrapOption {
    /// Operator should be on a new line (default).
    #[default]
    Nl,
    /// Operator should be at end of line.
    Eol,
}

/// Tokens that OperatorWrap can check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorWrapToken {
    // Arithmetic
    Plus,
    Minus,
    Star,
    Div,
    Mod,
    // Comparison
    Equal,
    NotEqual,
    Gt,
    Ge,
    Lt,
    Le,
    // Logical
    Land,
    Lor,
    // Bitwise
    Band,
    Bor,
    Bxor,
    Sl,
    Sr,
    Bsr,
    // Ternary
    Question,
    Colon,
    // Assignment
    Assign,
    PlusAssign,
    MinusAssign,
    StarAssign,
    DivAssign,
    ModAssign,
    SlAssign,
    SrAssign,
    BsrAssign,
    BxorAssign,
    BorAssign,
    BandAssign,
    // Special
    TypeExtensionAnd,
    LiteralInstanceof,
    MethodRef,
}

impl OperatorWrapToken {
    /// Convert from checkstyle token name to our enum.
    pub fn from_checkstyle_name(name: &str) -> Option<Self> {
        match name.trim() {
            "PLUS" => Some(Self::Plus),
            "MINUS" => Some(Self::Minus),
            "STAR" => Some(Self::Star),
            "DIV" => Some(Self::Div),
            "MOD" => Some(Self::Mod),
            "EQUAL" => Some(Self::Equal),
            "NOT_EQUAL" => Some(Self::NotEqual),
            "GT" => Some(Self::Gt),
            "GE" => Some(Self::Ge),
            "LT" => Some(Self::Lt),
            "LE" => Some(Self::Le),
            "LAND" => Some(Self::Land),
            "LOR" => Some(Self::Lor),
            "BAND" => Some(Self::Band),
            "BOR" => Some(Self::Bor),
            "BXOR" => Some(Self::Bxor),
            "SL" => Some(Self::Sl),
            "SR" => Some(Self::Sr),
            "BSR" => Some(Self::Bsr),
            "QUESTION" => Some(Self::Question),
            "COLON" => Some(Self::Colon),
            "ASSIGN" => Some(Self::Assign),
            "PLUS_ASSIGN" => Some(Self::PlusAssign),
            "MINUS_ASSIGN" => Some(Self::MinusAssign),
            "STAR_ASSIGN" => Some(Self::StarAssign),
            "DIV_ASSIGN" => Some(Self::DivAssign),
            "MOD_ASSIGN" => Some(Self::ModAssign),
            "SL_ASSIGN" => Some(Self::SlAssign),
            "SR_ASSIGN" => Some(Self::SrAssign),
            "BSR_ASSIGN" => Some(Self::BsrAssign),
            "BXOR_ASSIGN" => Some(Self::BxorAssign),
            "BOR_ASSIGN" => Some(Self::BorAssign),
            "BAND_ASSIGN" => Some(Self::BandAssign),
            "TYPE_EXTENSION_AND" => Some(Self::TypeExtensionAnd),
            "LITERAL_INSTANCEOF" => Some(Self::LiteralInstanceof),
            "METHOD_REF" => Some(Self::MethodRef),
            _ => None,
        }
    }

    /// Get the token for a given operator string.
    pub fn from_operator(op: &str) -> Option<Self> {
        match op {
            "+" => Some(Self::Plus),
            "-" => Some(Self::Minus),
            "*" => Some(Self::Star),
            "/" => Some(Self::Div),
            "%" => Some(Self::Mod),
            "==" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Ge),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Le),
            "&&" => Some(Self::Land),
            "||" => Some(Self::Lor),
            "&" => Some(Self::Band),
            "|" => Some(Self::Bor),
            "^" => Some(Self::Bxor),
            "<<" => Some(Self::Sl),
            ">>" => Some(Self::Sr),
            ">>>" => Some(Self::Bsr),
            "?" => Some(Self::Question),
            ":" => Some(Self::Colon),
            "=" => Some(Self::Assign),
            "+=" => Some(Self::PlusAssign),
            "-=" => Some(Self::MinusAssign),
            "*=" => Some(Self::StarAssign),
            "/=" => Some(Self::DivAssign),
            "%=" => Some(Self::ModAssign),
            "<<=" => Some(Self::SlAssign),
            ">>=" => Some(Self::SrAssign),
            ">>>=" => Some(Self::BsrAssign),
            "^=" => Some(Self::BxorAssign),
            "|=" => Some(Self::BorAssign),
            "&=" => Some(Self::BandAssign),
            "instanceof" => Some(Self::LiteralInstanceof),
            "::" => Some(Self::MethodRef),
            _ => None,
        }
    }

    /// Default tokens as per checkstyle documentation.
    pub fn default_tokens() -> HashSet<Self> {
        [
            Self::Question,
            Self::Colon,
            Self::Equal,
            Self::NotEqual,
            Self::Div,
            Self::Plus,
            Self::Minus,
            Self::Star,
            Self::Mod,
            Self::Sr,
            Self::Bsr,
            Self::Ge,
            Self::Gt,
            Self::Sl,
            Self::Le,
            Self::Lt,
            Self::Bxor,
            Self::Bor,
            Self::Lor,
            Self::Band,
            Self::Land,
            Self::TypeExtensionAnd,
            Self::LiteralInstanceof,
        ]
        .into_iter()
        .collect()
    }
}

/// Configuration for OperatorWrap rule.
#[derive(Debug, Clone)]
pub struct OperatorWrap {
    pub option: WrapOption,
    pub tokens: HashSet<OperatorWrapToken>,
}

impl Default for OperatorWrap {
    fn default() -> Self {
        Self {
            option: WrapOption::Nl,
            tokens: OperatorWrapToken::default_tokens(),
        }
    }
}

impl FromConfig for OperatorWrap {
    const MODULE_NAME: &'static str = "OperatorWrap";

    fn from_config(properties: &Properties) -> Self {
        let option = properties
            .get("option")
            .map(|v| match *v {
                "eol" | "EOL" => WrapOption::Eol,
                _ => WrapOption::Nl,
            })
            .unwrap_or_default();

        let tokens = if let Some(tokens_str) = properties.get("tokens") {
            let mut tokens = HashSet::new();
            for token in tokens_str.split(',') {
                if let Some(t) = OperatorWrapToken::from_checkstyle_name(token.trim()) {
                    tokens.insert(t);
                }
            }
            if tokens.is_empty() {
                OperatorWrapToken::default_tokens()
            } else {
                tokens
            }
        } else {
            OperatorWrapToken::default_tokens()
        };

        Self { option, tokens }
    }
}

/// Node kinds that OperatorWrap cares about.
const RELEVANT_KINDS: &[&str] = &[
    "binary_expression",
    "ternary_expression",
    "assignment_expression",
    "variable_declarator",
    "instanceof_expression",
    "type_bound",
    "method_reference",
    "enhanced_for_statement",
    "resource",
    "element_value_pair",
];

impl Rule for OperatorWrap {
    fn name(&self) -> &'static str {
        "OperatorWrap"
    }

    fn relevant_kinds(&self) -> &'static [&'static str] {
        RELEVANT_KINDS
    }

    fn check(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        match node.kind() {
            "binary_expression" => self.check_binary_expression(ctx, node),
            "ternary_expression" => self.check_ternary_expression(ctx, node),
            "assignment_expression" => self.check_assignment_expression(ctx, node),
            "variable_declarator" => self.check_variable_declarator(ctx, node),
            "instanceof_expression" => self.check_instanceof_expression(ctx, node),
            "type_bound" => self.check_type_bound(ctx, node),
            "method_reference" => self.check_method_reference(ctx, node),
            "enhanced_for_statement" => self.check_enhanced_for(ctx, node),
            "resource" => self.check_resource(ctx, node),
            "element_value_pair" => self.check_element_value_pair(ctx, node),
            _ => vec![],
        }
    }
}

impl OperatorWrap {
    /// Check binary expression operators (+, -, *, /, %, ==, !=, etc.).
    fn check_binary_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        if children.len() < 3 {
            return vec![];
        }

        let left = children[0];
        let operator = children[1];
        let right = children[2];

        let op_text = operator.utf8_text(source.as_bytes()).unwrap_or("");

        // Check if this operator token is configured
        // For & in binary expression, it's BAND not TYPE_EXTENSION_AND
        let token = OperatorWrapToken::from_operator(op_text);
        if let Some(t) = token {
            if !self.tokens.contains(&t) {
                return vec![];
            }
        } else {
            return vec![];
        }

        self.check_wrap(ctx, &left, &operator, &right, op_text)
    }

    /// Check ternary expression (? and :).
    fn check_ternary_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // ternary: condition ? then_expr : else_expr
        // Children: condition, ?, then, :, else
        if children.len() < 5 {
            return vec![];
        }

        let mut diagnostics = vec![];

        // Check ? operator
        let condition = &children[0];
        let question = &children[1];
        let then_expr = &children[2];

        let q_text = question.utf8_text(source.as_bytes()).unwrap_or("");
        if q_text == "?" && self.tokens.contains(&OperatorWrapToken::Question) {
            diagnostics.extend(self.check_wrap(ctx, condition, question, then_expr, "?"));
        }

        // Check : operator
        let colon = &children[3];
        let else_expr = &children[4];

        let c_text = colon.utf8_text(source.as_bytes()).unwrap_or("");
        if c_text == ":" && self.tokens.contains(&OperatorWrapToken::Colon) {
            diagnostics.extend(self.check_wrap(ctx, then_expr, colon, else_expr, ":"));
        }

        diagnostics
    }

    /// Check assignment expression (=, +=, -=, etc.).
    fn check_assignment_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        if children.len() < 3 {
            return vec![];
        }

        let left = &children[0];
        let operator = &children[1];
        let right = &children[2];

        let op_text = operator.utf8_text(source.as_bytes()).unwrap_or("");

        let token = OperatorWrapToken::from_operator(op_text);
        if let Some(t) = token {
            if !self.tokens.contains(&t) {
                return vec![];
            }
        } else {
            return vec![];
        }

        self.check_wrap(ctx, left, operator, right, op_text)
    }

    /// Check variable declarator initialization (=).
    fn check_variable_declarator(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        // Only check if ASSIGN token is configured
        if !self.tokens.contains(&OperatorWrapToken::Assign) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // variable_declarator: identifier = value
        // Children: identifier, =, value
        if children.len() < 3 {
            return vec![];
        }

        // Find the = operator
        let mut eq_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "=" {
                eq_idx = Some(i);
                break;
            }
        }

        let Some(i) = eq_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, "=")
    }

    /// Check instanceof expression.
    fn check_instanceof_expression(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::LiteralInstanceof) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // instanceof_expression: expr instanceof Type
        if children.len() < 3 {
            return vec![];
        }

        // Find instanceof keyword
        let mut inst_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "instanceof" {
                inst_idx = Some(i);
                break;
            }
        }

        let Some(i) = inst_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, "instanceof")
    }

    /// Check type bound (& in generics).
    fn check_type_bound(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::TypeExtensionAnd) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        let mut diagnostics = vec![];

        // type_bound: extends Type1 & Type2 & Type3
        // Find all & operators
        for i in 0..children.len() {
            let child = &children[i];
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "&" && i > 0 && i + 1 < children.len() {
                let left = &children[i - 1];
                let right = &children[i + 1];
                diagnostics.extend(self.check_wrap(ctx, left, child, right, "&"));
            }
        }

        diagnostics
    }

    /// Check method reference (::).
    fn check_method_reference(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::MethodRef) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // method_reference: Type::method or expr::method
        // Find :: operator
        let mut ref_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "::" {
                ref_idx = Some(i);
                break;
            }
        }

        let Some(i) = ref_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, "::")
    }

    /// Check enhanced for statement (:).
    fn check_enhanced_for(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::Colon) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // enhanced_for_statement: for ( Type var : iterable ) stmt
        // Find : operator
        let mut colon_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == ":" {
                colon_idx = Some(i);
                break;
            }
        }

        let Some(i) = colon_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, ":")
    }

    /// Check resource in try-with-resources (=).
    fn check_resource(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::Assign) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // resource: Type var = expr
        // Find = operator
        let mut eq_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "=" {
                eq_idx = Some(i);
                break;
            }
        }

        let Some(i) = eq_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, "=")
    }

    /// Check element value pair in annotations (=).
    fn check_element_value_pair(&self, ctx: &CheckContext, node: &CstNode) -> Vec<Diagnostic> {
        if !self.tokens.contains(&OperatorWrapToken::Assign) {
            return vec![];
        }

        let ts_node = node.inner();
        let source = ctx.source();

        let mut cursor = ts_node.walk();
        let children: Vec<_> = ts_node
            .children(&mut cursor)
            .filter(|n| !n.is_extra())
            .collect();

        // element_value_pair: identifier = value
        // Find = operator
        let mut eq_idx = None;
        for (i, child) in children.iter().enumerate() {
            let text = child.utf8_text(source.as_bytes()).unwrap_or("");
            if text == "=" {
                eq_idx = Some(i);
                break;
            }
        }

        let Some(i) = eq_idx else {
            return vec![];
        };

        if i == 0 || i + 1 >= children.len() {
            return vec![];
        }

        let left = &children[i - 1];
        let operator = &children[i];
        let right = &children[i + 1];

        self.check_wrap(ctx, left, operator, right, "=")
    }

    /// Core wrap checking logic used by all operator types.
    fn check_wrap(
        &self,
        ctx: &CheckContext,
        left: &tree_sitter::Node,
        operator: &tree_sitter::Node,
        right: &tree_sitter::Node,
        op_text: &str,
    ) -> Vec<Diagnostic> {
        let source_code = ctx.source_code();

        let left_end = TextSize::from(left.end_byte() as u32);
        let right_start = TextSize::from(right.start_byte() as u32);
        let op_start = TextSize::from(operator.start_byte() as u32);
        let op_end = TextSize::from(operator.end_byte() as u32);

        let left_end_line = source_code.line_column(left_end).line.get();
        let right_start_line = source_code.line_column(right_start).line.get();
        let op_line = source_code.line_column(op_start).line.get();

        // Only check if expression spans multiple lines
        if left_end_line == right_start_line {
            return vec![];
        }

        let op_range = TextRange::new(op_start, op_end);

        match self.option {
            WrapOption::Nl => {
                // Operator should be on new line (same line as right operand)
                // Violation if operator is on same line as left operand
                if op_line == left_end_line && op_line != right_start_line {
                    let fix = self.create_fix_nl(ctx, left, operator, right, op_text);
                    let mut diagnostic = Diagnostic::new(
                        OperatorShouldBeOnNewLine {
                            operator: op_text.to_string(),
                        },
                        op_range,
                    );
                    if let Some(f) = fix {
                        diagnostic = diagnostic.with_fix(f);
                    }
                    return vec![diagnostic];
                }
            }
            WrapOption::Eol => {
                // Operator should be at end of line (same line as left operand)
                // Violation if operator is NOT on same line as left operand
                if op_line != left_end_line {
                    let fix = self.create_fix_eol(ctx, left, operator, right, op_text);
                    let mut diagnostic = Diagnostic::new(
                        OperatorShouldBeOnPrevLine {
                            operator: op_text.to_string(),
                        },
                        op_range,
                    );
                    if let Some(f) = fix {
                        diagnostic = diagnostic.with_fix(f);
                    }
                    return vec![diagnostic];
                }
            }
        }

        vec![]
    }

    /// Create fix for NL option: move operator from end of line to start of next line.
    fn create_fix_nl(
        &self,
        ctx: &CheckContext,
        left: &tree_sitter::Node,
        operator: &tree_sitter::Node,
        right: &tree_sitter::Node,
        op_text: &str,
    ) -> Option<Fix> {
        let source = ctx.source();

        // Check for comments between operator and right operand
        let op_end = operator.end_byte();
        let right_start = right.start_byte();
        let between = &source[op_end..right_start];
        if between.contains("//") || between.contains("/*") {
            return None; // Don't fix if there are comments
        }

        let left_end = left.end_byte();

        // Find the indentation of the right operand's line
        let right_line_start = source[..right_start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let indent: String = source[right_line_start..right_start]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        // Replace from after left operand to before right operand
        // Original: "1 +\n            2" -> "1\n            + 2"
        let range_start = TextSize::from(left_end as u32);
        let range_end = TextSize::from(right_start as u32);
        // New content: newline + indent + operator + space
        let replacement = format!("\n{}{} ", indent, op_text);

        let edit = Edit::replacement(replacement, range_start, range_end);

        Some(Fix::safe_edit(edit))
    }

    /// Create fix for EOL option: move operator from start of line to end of previous line.
    fn create_fix_eol(
        &self,
        ctx: &CheckContext,
        left: &tree_sitter::Node,
        operator: &tree_sitter::Node,
        right: &tree_sitter::Node,
        op_text: &str,
    ) -> Option<Fix> {
        let source = ctx.source();

        // Check for comments between left operand and operator
        let left_end = left.end_byte();
        let op_start = operator.start_byte();
        let between = &source[left_end..op_start];
        if between.contains("//") || between.contains("/*") {
            return None; // Don't fix if there are comments
        }

        let right_start = right.start_byte();

        // Find the indentation of the right operand's line
        let right_line_start = source[..right_start]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let indent: String = source[right_line_start..right_start]
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();

        // Replace from after left operand to before right operand
        // Original: "1\n            + 2" -> "1 +\n            2"
        let range_start = TextSize::from(left_end as u32);
        let range_end = TextSize::from(right_start as u32);
        // New content: space + operator + newline + indent
        let replacement = format!(" {}\n{}", op_text, indent);

        let edit = Edit::replacement(replacement, range_start, range_end);

        Some(Fix::safe_edit(edit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lintal_java_cst::TreeWalker;
    use lintal_java_parser::JavaParser;

    fn check_source_nl(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap::default();

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    fn check_source_eol(source: &str) -> Vec<Diagnostic> {
        let mut parser = JavaParser::new();
        let result = parser.parse(source).unwrap();
        let ctx = CheckContext::new(source);
        let rule = OperatorWrap {
            option: WrapOption::Eol,
            tokens: OperatorWrapToken::default_tokens(),
        };

        let mut diagnostics = vec![];
        for node in TreeWalker::new(result.tree.root_node(), source) {
            diagnostics.extend(rule.check(&ctx, &node));
        }
        diagnostics
    }

    #[test]
    fn test_nl_operator_at_end_of_line_violation() {
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Operator at end of line should be violation with nl option"
        );
    }

    #[test]
    fn test_nl_operator_on_new_line_ok() {
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics.is_empty(),
            "Operator on new line should be OK with nl option"
        );
    }

    #[test]
    fn test_eol_operator_on_new_line_violation() {
        let source = r#"
class Test {
    void method() {
        int x = 1
            + 2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert_eq!(
            diagnostics.len(),
            1,
            "Operator on new line should be violation with eol option"
        );
    }

    #[test]
    fn test_eol_operator_at_end_of_line_ok() {
        let source = r#"
class Test {
    void method() {
        int x = 1 +
            2;
    }
}
"#;
        let diagnostics = check_source_eol(source);
        assert!(
            diagnostics.is_empty(),
            "Operator at end of line should be OK with eol option"
        );
    }

    #[test]
    fn test_same_line_no_violation() {
        let source = r#"
class Test {
    void method() {
        int x = 1 + 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics.is_empty(),
            "Same line expression should not cause violation"
        );
    }

    #[test]
    fn test_ternary_question_violation() {
        let source = r#"
class Test {
    void method() {
        int x = true ?
            1 : 2;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics.iter().any(|d| d.kind.body.contains("?")),
            "Should detect ternary ? at end of line"
        );
    }

    #[test]
    fn test_instanceof_violation() {
        let source = r#"
class Test {
    void method(Object o) {
        boolean b = o instanceof
            String;
    }
}
"#;
        let diagnostics = check_source_nl(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.kind.body.contains("instanceof")),
            "Should detect instanceof at end of line"
        );
    }
}
