pub struct ConstExpr(pub OrExpr);

pub enum OrExpr {
    XorExpr(XorExpr),
    OrExpr(Box<OrExpr>, XorExpr),
}

pub enum XorExpr {
    AndExpr(AndExpr),
    XorExpr(Box<XorExpr>, AndExpr),
}

pub enum AndExpr {
    ShiftExpr(ShiftExpr),
    AndExpr(Box<AndExpr>, ShiftExpr),
}

pub enum ShiftExpr {
    AdditiveExpr(AddExpr),
    LeftShiftExpr(Box<ShiftExpr>, AddExpr),
    RightShiftExpr(Box<ShiftExpr>, AddExpr),
}

pub enum AddExpr {
    MultExpr(MultExpr),
    AddExpr(Box<AddExpr>, MultExpr),
    SubExpr(Box<AddExpr>, MultExpr),
}

pub enum MultExpr {
    UnaryExpr(UnaryExpr),
    MultExpr(Box<MultExpr>, UnaryExpr),
    DivExpr(Box<MultExpr>, UnaryExpr),
    ModExpr(Box<MultExpr>, UnaryExpr),
}

pub enum UnaryExpr {
    UnaryExpr(UnaryOperator, PrimaryExpr),
    PrimaryExpr(PrimaryExpr),
}

pub enum PrimaryExpr {
    ScopedName(ScopedName),
    Literal(Literal),
    ConstExpr(Box<ConstExpr>),
}

pub enum UnaryOperator {
    Add,
    Sub,
    Not,
}

pub struct ScopedName(pub String);

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

pub enum IntegerLiteral {
    BinNumber(String),
    OctNumber(String),
    DecNumber(String),
    HexNumber(String),
}
