#[derive(Debug, Clone, PartialEq)]
pub struct Identifier(String);

pub struct PositiveIntConst(pub ConstExpr);

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

pub struct SignedShortInt;
pub struct SignedLongInt;
pub struct SignedLongLongInt;
pub struct UnsignedInt;
pub struct UnsignedTinyInt;
pub struct BooleanType;
pub struct FixedPtConstType;
pub struct OctetType;
pub struct IntegerType;
pub enum SignedInt {
    SignedShortInt,
    SignedLongInt,
    SignedLongLongInt,
    SignedTinyInt,
}

pub struct SignedTinyInt;
pub struct UnsignedShortInt;
pub struct UnsignedLongInt;
pub struct UnsignedLongLongInt;

pub struct FloatingPtType;
pub struct CharType;
pub struct WideCharType;
pub struct StringType {
    pub bound: Option<PositiveIntConst>,
}

pub struct WideStringType {
    pub bound: Option<PositiveIntConst>,
}

pub enum TypeSpec {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
}

pub enum SimpleTypeSpec {
    BaseTypeSpec(BaseTypeSpec),
    ScopedName(ScopedName),
}

pub enum BaseTypeSpec {
    IntegerType(IntegerType),
    FloatingPtType(FloatingPtType),
    CharType(CharType),
    WideCharType(WideCharType),
    BooleanType(BooleanType),
    OctetType(OctetType),
    AnyType(AnyType),
    ObjectType(ObjectType),
    ValueBaseType(ValueBaseType),
}

pub struct AnyType;

pub struct FixedPtType {
    pub integer: PositiveIntConst,
    pub fraction: PositiveIntConst,
}

pub enum TemplateTypeSpec {
    SequenceType(SequenceType),
    StringType(StringType),
    WideStringType(WideStringType),
    FixedPtType(FixedPtType),
    MapType(MapType),
}

pub struct SequenceType {
    pub ty: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

pub struct MapType {
    pub key: Box<TypeSpec>,
    pub value: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

pub struct ObjectType;
pub struct ValueBaseType;
