use derive::Parser;

use crate::ast::Identifier;

#[derive(Debug, Parser)]
pub struct ConstExpr(pub OrExpr);

#[derive(Debug, Parser)]
pub enum OrExpr {
    XorExpr(XorExpr),
    OrExpr(Box<OrExpr>, XorExpr),
}

#[derive(Debug, Parser)]
pub enum XorExpr {
    AndExpr(AndExpr),
    XorExpr(Box<XorExpr>, AndExpr),
}

#[derive(Debug, Parser)]
pub enum AndExpr {
    ShiftExpr(ShiftExpr),
    AndExpr(Box<AndExpr>, ShiftExpr),
}

#[derive(Debug, Parser)]
pub enum ShiftExpr {
    AdditiveExpr(AddExpr),
    LeftShiftExpr(Box<ShiftExpr>, AddExpr),
    RightShiftExpr(Box<ShiftExpr>, AddExpr),
}

#[derive(Debug, Parser)]
pub enum AddExpr {
    MultExpr(MultExpr),
    AddExpr(Box<AddExpr>, MultExpr),
    SubExpr(Box<AddExpr>, MultExpr),
}

#[derive(Debug, Parser)]
pub enum MultExpr {
    UnaryExpr(UnaryExpr),
    MultExpr(Box<MultExpr>, UnaryExpr),
    DivExpr(Box<MultExpr>, UnaryExpr),
    ModExpr(Box<MultExpr>, UnaryExpr),
}

#[derive(Debug, Parser)]
pub enum UnaryExpr {
    UnaryExpr(UnaryOperator, PrimaryExpr),
    PrimaryExpr(PrimaryExpr),
}

#[derive(Debug, Parser)]
pub enum PrimaryExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    ConstExpr(Box<ConstExpr>),
}

#[derive(Debug, Parser)]
pub enum UnaryOperator {
    Add,
    Sub,
    Not,
}

#[derive(Debug, Parser)]
pub struct ScopedName {
    pub identifier: Vec<Identifier>,
}

#[derive(Debug, Parser)]
pub enum Literal {
    IntegerLiteral(IntegerLiteral),
    FloatingPtLiteral,
    FixedPtLiteral,
    // CharLiteral(char),
    // WideCharacterLiteral,
    // StringLiteral(String),
    // WideStringLiteral,
    // BooleanLiteral(bool),
}

#[derive(Debug, Parser)]
pub enum IntegerLiteral {
    BinNumber(String),
    OctNumber(String),
    DecNumber(String),
    HexNumber(String),
}
