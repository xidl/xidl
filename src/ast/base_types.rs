use derive::Parser;

use super::*;

pub enum SignedShortInt {
    Short(Short),
    Int16(Int16),
}
#[derive(Parser)]
pub struct Short;

#[derive(Parser)]
pub struct Int16;

pub enum SignedLongInt {
    Long(Long),
    Int32(Int32),
}

#[derive(Parser)]
pub struct Long;

#[derive(Parser)]
pub struct Int32;

pub enum SignedLongLongInt {
    LongLong(LongLong),
    Int64(Int64),
}

#[derive(Parser)]
#[ts(name = "long long")]
pub struct LongLong;

#[derive(Parser)]
pub struct Int64;

pub struct UnsignedInt;
#[derive(Parser)]
#[ts(name = "uint8")]
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
}

#[derive(Debug, Parser)]
pub enum SignedInt {
    SignedShortInt,
    SignedLongInt,
    SignedLongLongInt,
    SignedTinyInt,
}

#[derive(Parser)]
#[ts(name = "int8")]
pub struct SignedTinyInt;
pub enum UnsignedShortInt {
    UnsignedShort(UnsignedShort),
    UInt16(UInt16),
}

#[derive(Parser)]
#[ts(name = "unsigned short")]
pub struct UnsignedShort;

#[derive(Parser)]
pub struct UInt16;

pub enum UnsignedLongInt {
    UnsignedLong(UnsignedLong),
    UInt32(UInt32),
}

#[derive(Parser)]
#[ts(name = "unsigned long")]
pub struct UnsignedLong;

#[derive(Parser)]
pub struct UInt32;

pub enum UnsignedLongLongInt {
    UnsignedLongLong(UnsignedLongLong),
    UInt64(UInt64),
}

#[derive(Parser)]
#[ts(name = "unsigned long long")]
pub struct UnsignedLongLong;

#[derive(Parser)]
pub struct UInt64;

#[derive(Debug, Parser)]
pub enum FloatingPtType {
    Float(Float),
    Double(Double),
    LongDouble(LongDouble),
}

#[derive(Debug, Parser)]
pub struct Float;

#[derive(Debug, Parser)]
pub struct Double;

#[derive(Debug, Parser)]
pub struct LongDouble;

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
