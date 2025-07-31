use derive::Parser;

use crate::parser::FromTreeSitter;

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
#[ts(text = "long long")]
pub struct LongLong;

#[derive(Parser)]
pub struct Int64;

pub struct UnsignedInt;
#[derive(Parser)]
#[ts(text = "uint8")]
pub struct UnsignedTinyInt;

#[derive(Parser)]
#[ts(text = "boolean")]
pub struct BooleanType;

#[derive(Parser)]
#[ts(text = "fixed")]
pub struct FixedPtConstType;

#[derive(Parser)]
#[ts(text = "octet")]
pub struct OctetType;
pub struct IntegerType;
pub enum SignedInt {
    SignedShortInt,
    SignedLongInt,
    SignedLongLongInt,
    SignedTinyInt,
}

#[derive(Parser)]
#[ts(text = "int8")]
pub struct SignedTinyInt;
pub enum UnsignedShortInt {
    UnsignedShort(UnsignedShort),
    UInt16(UInt16),
}

#[derive(Parser)]
#[ts(text = "unsigned short")]
pub struct UnsignedShort;

#[derive(Parser)]
pub struct UInt16;

pub enum UnsignedLongInt {
    UnsignedLong(UnsignedLong),
    UInt32(UInt32),
}

#[derive(Parser)]
#[ts(text = "unsigned long")]
pub struct UnsignedLong;

#[derive(Parser)]
pub struct UInt32;

pub enum UnsignedLongLongInt {
    UnsignedLongLong(UnsignedLongLong),
    UInt64(UInt64),
}

#[derive(Parser)]
#[ts(text = "unsigned long long")]
pub struct UnsignedLongLong;

#[derive(Parser)]
pub struct UInt64;

pub enum FloatingPtType {
    Float(Float),
    Double(Double),
    LongDouble(LongDouble),
}

#[derive(Parser)]
pub struct Float;
#[derive(Parser)]
pub struct Double;
#[derive(Parser)]
pub struct LongDouble;

#[derive(Parser)]
#[ts(text = "char")]
pub struct CharType;

#[derive(Parser)]
#[ts(text = "wchar")]
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

#[derive(Parser)]
#[ts(text = "any")]
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
