use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub enum TypeSpec {
    SimpleTypeSpec(SimpleTypeSpec),
    // TemplateTypeSpec(TemplateTypeSpec),
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
            crate::typed_ast::TypeSpec::TemplateTypeSpec(_) => todo!(),
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
