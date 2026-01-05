use derive::Parser;

use super::*;

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct SignedShortInt;

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct SignedLongInt;

#[derive(Debug, Parser)]
#[ts(id = "signed_longlong_int")]
pub struct SignedLongLongInt;

#[derive(Debug, Parser)]
pub enum UnsignedInt {
    UnsignedShortInt(UnsignedShortInt),
    UnsignedLongInt(UnsignedLongInt),
    #[ts(id = "unsigned_longlong_int")]
    UnsignedLongLongInt(UnsignedLongLongInt),
    UnsignedTinyInt(UnsignedTinyInt),
}

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct UnsignedTinyInt;

#[derive(Debug, Parser)]
#[ts(name = "boolean")]
pub struct BooleanType;

#[derive(Debug, Parser)]
#[ts(name = "fixed")]
pub struct FixedPtConstType;

#[derive(Debug, Parser)]
#[ts(name = "octet")]
pub struct OctetType;

#[derive(Debug, Parser)]
pub enum IntegerType {
    SignedInt(SignedInt),
    UnsignedInt(UnsignedInt),
}

#[derive(Debug, Parser)]
pub enum SignedInt {
    SignedShortInt(SignedShortInt),
    SignedLongInt(SignedLongInt),
    #[ts(id = "signed_longlong_int")]
    SignedLongLongInt(SignedLongLongInt),
    SignedTinyInt(SignedTinyInt),
}

#[derive(Debug, Parser)]
#[ts(name = "int8")]
pub struct SignedTinyInt;

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct UnsignedShortInt;

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct UnsignedLongInt;

#[derive(Debug, Parser)]
#[ts(mark)]
#[ts(id = "unsigned_longlong_int")]
pub struct UnsignedLongLongInt;

#[derive(Debug, Parser)]
#[ts(mark)]
pub struct FloatingPtType;

#[derive(Debug, Parser)]
#[ts(name = "char")]
pub struct CharType;

#[derive(Debug, Parser)]
#[ts(name = "wchar")]
pub struct WideCharType;

#[derive(Debug, Parser)]
pub struct StringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Debug, Parser)]
pub struct WideStringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Debug, Parser)]
pub enum TypeSpec {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
}

#[derive(Debug, Parser)]
pub enum SimpleTypeSpec {
    BaseTypeSpec(BaseTypeSpec),
    ScopedName(ScopedName),
}

#[derive(Debug, Parser)]
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

#[derive(Debug, Parser)]
#[ts(name = "any")]
pub struct AnyType;

#[derive(Debug, Parser)]
pub struct FixedPtType {
    pub integer: PositiveIntConst,
    pub fraction: PositiveIntConst,
}

#[derive(Debug, Parser)]
pub enum TemplateTypeSpec {
    SequenceType(SequenceType),
    StringType(StringType),
    WideStringType(WideStringType),
    FixedPtType(FixedPtType),
    MapType(MapType),
}

#[derive(Debug, Parser)]
pub struct SequenceType {
    pub ty: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Debug, Parser)]
pub struct MapType {
    pub key: Box<TypeSpec>,
    pub value: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Debug, Parser)]
pub struct ObjectType;

#[derive(Debug, Parser)]
pub struct ValueBaseType;
