#[derive(Debug)]
pub struct ConstExpr(pub OrExpr);

#[derive(Debug)]
pub enum OrExpr {
    XorExpr(XorExpr),
    OrExpr(Box<OrExpr>, XorExpr),
}

#[derive(Debug)]
pub enum XorExpr {
    AndExpr(AndExpr),
    XorExpr(Box<XorExpr>, AndExpr),
}

#[derive(Debug)]
pub enum AndExpr {
    ShiftExpr(ShiftExpr),
    AndExpr(Box<AndExpr>, ShiftExpr),
}

#[derive(Debug)]
pub enum ShiftExpr {
    AdditiveExpr(AddExpr),
    LeftShiftExpr(Box<ShiftExpr>, AddExpr),
    RightShiftExpr(Box<ShiftExpr>, AddExpr),
}

#[derive(Debug)]
pub enum AddExpr {
    MultExpr(MultExpr),
    AddExpr(Box<AddExpr>, MultExpr),
    SubExpr(Box<AddExpr>, MultExpr),
}

#[derive(Debug)]
pub enum MultExpr {
    UnaryExpr(UnaryExpr),
    MultExpr(Box<MultExpr>, UnaryExpr),
    DivExpr(Box<MultExpr>, UnaryExpr),
    ModExpr(Box<MultExpr>, UnaryExpr),
}

#[derive(Debug)]
pub enum UnaryExpr {
    UnaryExpr(UnaryOperator, PrimaryExpr),
    PrimaryExpr(PrimaryExpr),
}

#[derive(Debug)]
pub enum PrimaryExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    ConstExpr(Box<ConstExpr>),
}

#[derive(Debug)]
pub enum UnaryOperator {
    Add,
    Sub,
    Not,
}

#[derive(Debug)]
pub struct ScopedName(pub String);

#[derive(Debug)]
pub enum Literal {
    IntegerLiteral(IntegerLiteral),
    FloatingPtLiteral,
    FixedPtLiteral,
    CharLiteral(char),
    WideCharacterLiteral,
    StringLiteral(String),
    WideStringLiteral,
    BooleanLiteral(bool),
}

#[derive(Debug)]
pub enum IntegerLiteral {
    BinNumber(String),
    OctNumber(String),
    DecNumber(String),
    HexNumber(String),
}
