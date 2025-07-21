use super::*;

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
