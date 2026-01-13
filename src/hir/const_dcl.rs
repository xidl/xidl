use super::*;

#[derive(Debug)]
pub struct ConstDcl {
    pub ty: ConstType,
    pub ident: String,
    pub value: ConstExpr,
}

#[derive(Debug)]
pub enum ConstType {
    IntegerType(IntegerType),
    FloatingPtType,
    FixedPtConstType,
    CharType,
    WideCharType,
    BooleanType,
    OctetType,
    StringType(StringType),
    WideStringType(WideStringType),
    ScopedName(ScopedName),
    SequenceType(SequenceType),
}

impl From<crate::typed_ast::ConstDcl> for ConstDcl {
    fn from(value: crate::typed_ast::ConstDcl) -> Self {
        Self {
            ty: value.ty.into(),
            ident: value.ident.0,
            value: value.value,
        }
    }
}

impl From<crate::typed_ast::ConstType> for ConstType {
    fn from(value: crate::typed_ast::ConstType) -> Self {
        match value {
            crate::typed_ast::ConstType::IntegerType(integer_type) => {
                Self::IntegerType(integer_type.into())
            }
            crate::typed_ast::ConstType::FloatingPtType(_) => Self::FloatingPtType,
            crate::typed_ast::ConstType::FixedPtConstType(_) => Self::FixedPtConstType,
            crate::typed_ast::ConstType::CharType(_) => Self::CharType,
            crate::typed_ast::ConstType::WideCharType(_) => Self::WideCharType,
            crate::typed_ast::ConstType::BooleanType(_) => Self::BooleanType,
            crate::typed_ast::ConstType::OctetType(_) => Self::OctetType,
            crate::typed_ast::ConstType::StringType(string_type) => {
                Self::StringType(string_type.into())
            }
            crate::typed_ast::ConstType::WideStringType(wide_string_type) => {
                Self::WideStringType(wide_string_type.into())
            }
            crate::typed_ast::ConstType::ScopedName(scoped_name) => {
                Self::ScopedName(scoped_name.into())
            }
            crate::typed_ast::ConstType::SequenceType(sequence_type) => {
                Self::SequenceType(sequence_type.into())
            }
        }
    }
}
