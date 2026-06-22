use serde::{Deserialize, Serialize};

use super::ScopedName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    UnaryExpr(UnaryOperator, Box<ConstExpr>),
    BinaryExpr(BinaryOperator, Box<ConstExpr>, Box<ConstExpr>),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BinaryOperator {
    Or,
    Xor,
    And,
    LeftShift,
    RightShift,
    Add,
    Sub,
    Mult,
    Div,
    Mod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Add,
    Sub,
    Not,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Literal {
    IntegerLiteral(IntegerLiteral),
    FloatingPtLiteral(FloatingPtLiteral),
    CharLiteral(String),
    WideCharacterLiteral(String),
    StringLiteral(String),
    WideStringLiteral(String),
    BooleanLiteral(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerLiteral(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingPtLiteral {
    pub sign: Option<IntegerSign>,
    pub integer: DecNumber,
    pub fraction: DecNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegerSign {
    Plus,
    Minus,
}

impl IntegerSign {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plus => "+",
            Self::Minus => "-",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecNumber(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositiveIntConst(pub ConstExpr);

pub fn const_expr_to_i64(expr: &ConstExpr) -> Option<i64> {
    match expr {
        ConstExpr::Literal(Literal::IntegerLiteral(lit)) => parse_int_literal(lit),
        ConstExpr::UnaryExpr(UnaryOperator::Add, expr) => const_expr_to_i64(expr),
        ConstExpr::UnaryExpr(UnaryOperator::Sub, expr) => const_expr_to_i64(expr).map(|v| -v),
        ConstExpr::ScopedName(_)
        | ConstExpr::Literal(_)
        | ConstExpr::UnaryExpr(UnaryOperator::Not, _)
        | ConstExpr::BinaryExpr(_, _, _) => None,
    }
}

fn parse_int_literal(value: &IntegerLiteral) -> Option<i64> {
    parse_radix(&value.0, 10)
}

fn parse_radix(value: &str, radix: u32) -> Option<i64> {
    let cleaned = value.replace('_', "");
    let trimmed = cleaned.trim();
    let stripped = match radix {
        2 => trimmed
            .strip_prefix("0b")
            .or_else(|| trimmed.strip_prefix("0B")),
        8 => trimmed
            .strip_prefix("0o")
            .or_else(|| trimmed.strip_prefix("0O")),
        16 => trimmed
            .strip_prefix("0x")
            .or_else(|| trimmed.strip_prefix("0X")),
        _ => None,
    };
    i64::from_str_radix(stripped.unwrap_or(trimmed), radix).ok()
}

impl From<crate::typed_ast::ConstExpr> for ConstExpr {
    fn from(value: crate::typed_ast::ConstExpr) -> Self {
        from_or_expr(value.0)
    }
}

fn from_or_expr(value: crate::typed_ast::OrExpr) -> ConstExpr {
    match value {
        crate::typed_ast::OrExpr::XorExpr(value) => from_xor_expr(value),
        crate::typed_ast::OrExpr::OrExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Or,
            Box::new(from_or_expr(*left)),
            Box::new(from_xor_expr(right)),
        ),
    }
}

fn from_xor_expr(value: crate::typed_ast::XorExpr) -> ConstExpr {
    match value {
        crate::typed_ast::XorExpr::AndExpr(value) => from_and_expr(value),
        crate::typed_ast::XorExpr::XorExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Xor,
            Box::new(from_xor_expr(*left)),
            Box::new(from_and_expr(right)),
        ),
    }
}

fn from_and_expr(value: crate::typed_ast::AndExpr) -> ConstExpr {
    match value {
        crate::typed_ast::AndExpr::ShiftExpr(value) => from_shift_expr(value),
        crate::typed_ast::AndExpr::AndExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::And,
            Box::new(from_and_expr(*left)),
            Box::new(from_shift_expr(right)),
        ),
    }
}

fn from_shift_expr(value: crate::typed_ast::ShiftExpr) -> ConstExpr {
    match value {
        crate::typed_ast::ShiftExpr::AddExpr(value) => from_add_expr(value),
        crate::typed_ast::ShiftExpr::LeftShiftExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::LeftShift,
            Box::new(from_shift_expr(*left)),
            Box::new(from_add_expr(right)),
        ),
        crate::typed_ast::ShiftExpr::RightShiftExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::RightShift,
            Box::new(from_shift_expr(*left)),
            Box::new(from_add_expr(right)),
        ),
    }
}

fn from_add_expr(value: crate::typed_ast::AddExpr) -> ConstExpr {
    match value {
        crate::typed_ast::AddExpr::MultExpr(value) => from_mult_expr(value),
        crate::typed_ast::AddExpr::AddExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Add,
            Box::new(from_add_expr(*left)),
            Box::new(from_mult_expr(right)),
        ),
        crate::typed_ast::AddExpr::SubExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Sub,
            Box::new(from_add_expr(*left)),
            Box::new(from_mult_expr(right)),
        ),
    }
}

fn from_mult_expr(value: crate::typed_ast::MultExpr) -> ConstExpr {
    match value {
        crate::typed_ast::MultExpr::UnaryExpr(value) => from_unary_expr(value),
        crate::typed_ast::MultExpr::MultExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Mult,
            Box::new(from_mult_expr(*left)),
            Box::new(from_unary_expr(right)),
        ),
        crate::typed_ast::MultExpr::DivExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Div,
            Box::new(from_mult_expr(*left)),
            Box::new(from_unary_expr(right)),
        ),
        crate::typed_ast::MultExpr::ModExpr(left, right) => ConstExpr::BinaryExpr(
            BinaryOperator::Mod,
            Box::new(from_mult_expr(*left)),
            Box::new(from_unary_expr(right)),
        ),
    }
}

fn from_unary_expr(value: crate::typed_ast::UnaryExpr) -> ConstExpr {
    match value {
        crate::typed_ast::UnaryExpr::PrimaryExpr(value) => from_primary_expr(value),
        crate::typed_ast::UnaryExpr::UnaryExpr(op, value) => {
            ConstExpr::UnaryExpr(op.into(), Box::new(from_primary_expr(value)))
        }
    }
}

fn from_primary_expr(value: crate::typed_ast::PrimaryExpr) -> ConstExpr {
    match value {
        crate::typed_ast::PrimaryExpr::ScopedName(value) => ConstExpr::ScopedName(value.into()),
        crate::typed_ast::PrimaryExpr::Literal(value) => ConstExpr::Literal(value.into()),
        crate::typed_ast::PrimaryExpr::ConstExpr(value) => (*value).into(),
    }
}

impl From<crate::typed_ast::UnaryOperator> for UnaryOperator {
    fn from(value: crate::typed_ast::UnaryOperator) -> Self {
        match value {
            crate::typed_ast::UnaryOperator::Add => Self::Add,
            crate::typed_ast::UnaryOperator::Sub => Self::Sub,
            crate::typed_ast::UnaryOperator::Not => Self::Not,
        }
    }
}

impl From<crate::typed_ast::Literal> for Literal {
    fn from(value: crate::typed_ast::Literal) -> Self {
        match value {
            crate::typed_ast::Literal::IntegerLiteral(value) => Self::IntegerLiteral(value.into()),
            crate::typed_ast::Literal::FloatingPtLiteral(value) => {
                Self::FloatingPtLiteral(value.into())
            }
            crate::typed_ast::Literal::CharLiteral(value) => Self::CharLiteral(value),
            crate::typed_ast::Literal::WideCharacterLiteral(value) => {
                Self::WideCharacterLiteral(value)
            }
            crate::typed_ast::Literal::StringLiteral(value) => Self::StringLiteral(value),
            crate::typed_ast::Literal::WideStringLiteral(value) => Self::WideStringLiteral(value),
            crate::typed_ast::Literal::BooleanLiteral(value) => {
                Self::BooleanLiteral(value.as_bool())
            }
        }
    }
}

impl From<crate::typed_ast::IntegerLiteral> for IntegerLiteral {
    fn from(value: crate::typed_ast::IntegerLiteral) -> Self {
        let parsed = match value {
            crate::typed_ast::IntegerLiteral::BinNumber(value) => parse_radix(&value, 2),
            crate::typed_ast::IntegerLiteral::OctNumber(value) => parse_radix(&value, 8),
            crate::typed_ast::IntegerLiteral::DecNumber(value) => parse_radix(&value, 10),
            crate::typed_ast::IntegerLiteral::HexNumber(value) => parse_radix(&value, 16),
        }
        .expect("typed_ast integer literal should parse");
        Self(parsed.to_string())
    }
}

impl From<crate::typed_ast::FloatingPtLiteral> for FloatingPtLiteral {
    fn from(value: crate::typed_ast::FloatingPtLiteral) -> Self {
        Self {
            sign: value.sign.map(Into::into),
            integer: value.integer.into(),
            fraction: value.fraction.into(),
        }
    }
}

impl From<crate::typed_ast::IntegerSign> for IntegerSign {
    fn from(value: crate::typed_ast::IntegerSign) -> Self {
        match value {
            crate::typed_ast::IntegerSign::Plus => Self::Plus,
            crate::typed_ast::IntegerSign::Minus => Self::Minus,
        }
    }
}

impl From<crate::typed_ast::DecNumber> for DecNumber {
    fn from(value: crate::typed_ast::DecNumber) -> Self {
        Self(value.0)
    }
}

impl From<crate::typed_ast::PositiveIntConst> for PositiveIntConst {
    fn from(value: crate::typed_ast::PositiveIntConst) -> Self {
        Self(value.0.into())
    }
}

impl From<crate::typed_ast::FixedArraySize> for PositiveIntConst {
    fn from(value: crate::typed_ast::FixedArraySize) -> Self {
        value.0.into()
    }
}

#[cfg(test)]
mod tests;
