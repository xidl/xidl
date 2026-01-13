use serde::{Deserialize, Serialize};

use super::ScopedName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstExpr(pub OrExpr);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrExpr {
    XorExpr(XorExpr),
    OrExpr(Box<OrExpr>, XorExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum XorExpr {
    AndExpr(AndExpr),
    XorExpr(Box<XorExpr>, AndExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AndExpr {
    ShiftExpr(ShiftExpr),
    AndExpr(Box<AndExpr>, ShiftExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShiftExpr {
    AddExpr(AddExpr),
    LeftShiftExpr(Box<ShiftExpr>, AddExpr),
    RightShiftExpr(Box<ShiftExpr>, AddExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddExpr {
    MultExpr(MultExpr),
    AddExpr(Box<AddExpr>, MultExpr),
    SubExpr(Box<AddExpr>, MultExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultExpr {
    UnaryExpr(UnaryExpr),
    MultExpr(Box<MultExpr>, UnaryExpr),
    DivExpr(Box<MultExpr>, UnaryExpr),
    ModExpr(Box<MultExpr>, UnaryExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryExpr {
    UnaryExpr(UnaryOperator, PrimaryExpr),
    PrimaryExpr(PrimaryExpr),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimaryExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    ConstExpr(Box<ConstExpr>),
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
    BooleanLiteral(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegerLiteral {
    BinNumber(String),
    OctNumber(String),
    DecNumber(String),
    HexNumber(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatingPtLiteral {
    pub sign: Option<IntegerSign>,
    pub integer: DecNumber,
    pub fraction: DecNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerSign(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecNumber(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositiveIntConst(pub ConstExpr);

impl From<crate::typed_ast::ConstExpr> for ConstExpr {
    fn from(value: crate::typed_ast::ConstExpr) -> Self {
        Self(value.0.into())
    }
}

impl From<crate::typed_ast::OrExpr> for OrExpr {
    fn from(value: crate::typed_ast::OrExpr) -> Self {
        match value {
            crate::typed_ast::OrExpr::XorExpr(value) => Self::XorExpr(value.into()),
            crate::typed_ast::OrExpr::OrExpr(left, right) => {
                Self::OrExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::XorExpr> for XorExpr {
    fn from(value: crate::typed_ast::XorExpr) -> Self {
        match value {
            crate::typed_ast::XorExpr::AndExpr(value) => Self::AndExpr(value.into()),
            crate::typed_ast::XorExpr::XorExpr(left, right) => {
                Self::XorExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::AndExpr> for AndExpr {
    fn from(value: crate::typed_ast::AndExpr) -> Self {
        match value {
            crate::typed_ast::AndExpr::ShiftExpr(value) => Self::ShiftExpr(value.into()),
            crate::typed_ast::AndExpr::AndExpr(left, right) => {
                Self::AndExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::ShiftExpr> for ShiftExpr {
    fn from(value: crate::typed_ast::ShiftExpr) -> Self {
        match value {
            crate::typed_ast::ShiftExpr::AddExpr(value) => Self::AddExpr(value.into()),
            crate::typed_ast::ShiftExpr::LeftShiftExpr(left, right) => {
                Self::LeftShiftExpr(Box::new((*left).into()), right.into())
            }
            crate::typed_ast::ShiftExpr::RightShiftExpr(left, right) => {
                Self::RightShiftExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::AddExpr> for AddExpr {
    fn from(value: crate::typed_ast::AddExpr) -> Self {
        match value {
            crate::typed_ast::AddExpr::MultExpr(value) => Self::MultExpr(value.into()),
            crate::typed_ast::AddExpr::AddExpr(left, right) => {
                Self::AddExpr(Box::new((*left).into()), right.into())
            }
            crate::typed_ast::AddExpr::SubExpr(left, right) => {
                Self::SubExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::MultExpr> for MultExpr {
    fn from(value: crate::typed_ast::MultExpr) -> Self {
        match value {
            crate::typed_ast::MultExpr::UnaryExpr(value) => Self::UnaryExpr(value.into()),
            crate::typed_ast::MultExpr::MultExpr(left, right) => {
                Self::MultExpr(Box::new((*left).into()), right.into())
            }
            crate::typed_ast::MultExpr::DivExpr(left, right) => {
                Self::DivExpr(Box::new((*left).into()), right.into())
            }
            crate::typed_ast::MultExpr::ModExpr(left, right) => {
                Self::ModExpr(Box::new((*left).into()), right.into())
            }
        }
    }
}

impl From<crate::typed_ast::UnaryExpr> for UnaryExpr {
    fn from(value: crate::typed_ast::UnaryExpr) -> Self {
        match value {
            crate::typed_ast::UnaryExpr::PrimaryExpr(value) => Self::PrimaryExpr(value.into()),
            crate::typed_ast::UnaryExpr::UnaryExpr(op, value) => {
                Self::UnaryExpr(op.into(), value.into())
            }
        }
    }
}

impl From<crate::typed_ast::PrimaryExpr> for PrimaryExpr {
    fn from(value: crate::typed_ast::PrimaryExpr) -> Self {
        match value {
            crate::typed_ast::PrimaryExpr::ScopedName(value) => Self::ScopedName(value.into()),
            crate::typed_ast::PrimaryExpr::Literal(value) => Self::Literal(value.into()),
            crate::typed_ast::PrimaryExpr::ConstExpr(value) => {
                Self::ConstExpr(Box::new((*value).into()))
            }
        }
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
            crate::typed_ast::Literal::BooleanLiteral(value) => Self::BooleanLiteral(value),
        }
    }
}

impl From<crate::typed_ast::IntegerLiteral> for IntegerLiteral {
    fn from(value: crate::typed_ast::IntegerLiteral) -> Self {
        match value {
            crate::typed_ast::IntegerLiteral::BinNumber(value) => Self::BinNumber(value),
            crate::typed_ast::IntegerLiteral::OctNumber(value) => Self::OctNumber(value),
            crate::typed_ast::IntegerLiteral::DecNumber(value) => Self::DecNumber(value),
            crate::typed_ast::IntegerLiteral::HexNumber(value) => Self::HexNumber(value),
        }
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
        Self(value.0)
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
