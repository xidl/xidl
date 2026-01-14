use serde::{Deserialize, Serialize};

use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TypeSpec {
    SimpleTypeSpec(SimpleTypeSpec),
    TemplateTypeSpec(TemplateTypeSpec),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SimpleTypeSpec {
    IntegerType(IntegerType),
    FloatingPtType,
    CharType,
    WideCharType,
    Boolean,
    AnyType,
    ObjectType,
    ValueBaseType,
    ScopedName(ScopedName),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TemplateTypeSpec {
    SequenceType(SequenceType),
    StringType(StringType),
    WideStringType(WideStringType),
    FixedPtType(FixedPtType),
    MapType(MapType),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SequenceType {
    pub ty: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapType {
    pub key: Box<TypeSpec>,
    pub value: Box<TypeSpec>,
    pub len: Option<PositiveIntConst>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WideStringType {
    pub bound: Option<PositiveIntConst>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixedPtType {
    pub integer: PositiveIntConst,
    pub fraction: PositiveIntConst,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IntegerType {
    Char,
    UChar,
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    I32,
    I64,
}

impl From<crate::typed_ast::TypeSpec> for TypeSpec {
    fn from(value: crate::typed_ast::TypeSpec) -> Self {
        match value {
            crate::typed_ast::TypeSpec::SimpleTypeSpec(simple_type_spec) => {
                Self::SimpleTypeSpec(simple_type_spec.into())
            }
            crate::typed_ast::TypeSpec::TemplateTypeSpec(template_type_spec) => {
                Self::TemplateTypeSpec(template_type_spec.into())
            }
        }
    }
}

impl From<crate::typed_ast::SimpleTypeSpec> for SimpleTypeSpec {
    fn from(ty: crate::typed_ast::SimpleTypeSpec) -> Self {
        match ty {
            crate::typed_ast::SimpleTypeSpec::BaseTypeSpec(base_type_spec) => {
                match base_type_spec {
                    crate::typed_ast::BaseTypeSpec::IntegerType(integer_type) => {
                        Self::IntegerType(integer_type.into())
                    }
                    crate::typed_ast::BaseTypeSpec::FloatingPtType(_) => Self::FloatingPtType,
                    crate::typed_ast::BaseTypeSpec::CharType(_) => Self::CharType,
                    crate::typed_ast::BaseTypeSpec::WideCharType(_) => Self::WideCharType,
                    crate::typed_ast::BaseTypeSpec::BooleanType(_) => Self::Boolean,
                    crate::typed_ast::BaseTypeSpec::OctetType(_) => {
                        Self::IntegerType(IntegerType::U8)
                    }
                    crate::typed_ast::BaseTypeSpec::AnyType(_) => Self::AnyType,
                    crate::typed_ast::BaseTypeSpec::ObjectType(_) => Self::ObjectType,
                    crate::typed_ast::BaseTypeSpec::ValueBaseType(_) => Self::ValueBaseType,
                }
            }
            crate::typed_ast::SimpleTypeSpec::ScopedName(scoped_name) => {
                Self::ScopedName(scoped_name.into())
            }
        }
    }
}

impl From<crate::typed_ast::IntegerType> for IntegerType {
    fn from(value: crate::typed_ast::IntegerType) -> Self {
        match value {
            crate::typed_ast::IntegerType::SignedInt(signed_int) => match signed_int {
                crate::typed_ast::SignedInt::SignedShortInt(_) => Self::I16,
                crate::typed_ast::SignedInt::SignedLongInt(_) => Self::I32,
                crate::typed_ast::SignedInt::SignedLongLongInt(_) => Self::I64,
                crate::typed_ast::SignedInt::SignedTinyInt(_) => Self::I8,
            },
            crate::typed_ast::IntegerType::UnsignedInt(unsigned_int) => match unsigned_int {
                crate::typed_ast::UnsignedInt::UnsignedShortInt(_) => Self::U16,
                crate::typed_ast::UnsignedInt::UnsignedLongInt(_) => Self::U32,
                crate::typed_ast::UnsignedInt::UnsignedLongLongInt(_) => Self::U64,
                crate::typed_ast::UnsignedInt::UnsignedTinyInt(_) => Self::U8,
            },
        }
    }
}

impl From<crate::typed_ast::TemplateTypeSpec> for TemplateTypeSpec {
    fn from(value: crate::typed_ast::TemplateTypeSpec) -> Self {
        match value {
            crate::typed_ast::TemplateTypeSpec::SequenceType(sequence_type) => {
                Self::SequenceType(sequence_type.into())
            }
            crate::typed_ast::TemplateTypeSpec::StringType(string_type) => {
                Self::StringType(string_type.into())
            }
            crate::typed_ast::TemplateTypeSpec::WideStringType(wide_string_type) => {
                Self::WideStringType(wide_string_type.into())
            }
            crate::typed_ast::TemplateTypeSpec::FixedPtType(fixed_pt_type) => {
                Self::FixedPtType(fixed_pt_type.into())
            }
            crate::typed_ast::TemplateTypeSpec::MapType(map_type) => Self::MapType(map_type.into()),
        }
    }
}

impl From<crate::typed_ast::SequenceType> for SequenceType {
    fn from(value: crate::typed_ast::SequenceType) -> Self {
        Self {
            ty: Box::new((*value.ty).into()),
            len: value.len.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::MapType> for MapType {
    fn from(value: crate::typed_ast::MapType) -> Self {
        Self {
            key: Box::new((*value.key).into()),
            value: Box::new((*value.value).into()),
            len: value.len.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::StringType> for StringType {
    fn from(value: crate::typed_ast::StringType) -> Self {
        Self {
            bound: value.bound.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::WideStringType> for WideStringType {
    fn from(value: crate::typed_ast::WideStringType) -> Self {
        Self {
            bound: value.bound.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::FixedPtType> for FixedPtType {
    fn from(value: crate::typed_ast::FixedPtType) -> Self {
        Self {
            integer: value.integer.into(),
            fraction: value.fraction.into(),
        }
    }
}
