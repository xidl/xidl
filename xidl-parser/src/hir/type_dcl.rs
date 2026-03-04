use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeDcl {
    pub decl: TypeDclInner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
    TypedefDcl(TypedefDcl),
    NativeDcl(NativeDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypedefDcl {
    pub annotations: Vec<Annotation>,
    pub ty: TypedefType,
    pub decl: Vec<Declarator>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TypedefType {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NativeDcl {
    pub annotations: Vec<Annotation>,
    pub decl: SimpleDeclarator,
}

impl From<crate::typed_ast::TypeDcl> for TypeDcl {
    fn from(value: crate::typed_ast::TypeDcl) -> Self {
        let annotations = expand_annotations(value.annotations);
        let decl = match value.decl {
            crate::typed_ast::TypeDclInner::ConstrTypeDcl(constr) => {
                let mut decl: ConstrTypeDcl = constr.into();
                match &mut decl {
                    ConstrTypeDcl::StructForwardDcl(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::StructDcl(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::EnumDcl(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::UnionForwardDcl(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::UnionDef(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::BitsetDcl(value) => value.annotations.extend(annotations),
                    ConstrTypeDcl::BitmaskDcl(value) => value.annotations.extend(annotations),
                }
                TypeDclInner::ConstrTypeDcl(decl)
            }
            crate::typed_ast::TypeDclInner::TypedefDcl(typedef) => {
                let mut decl: TypedefDcl = typedef.into();
                decl.annotations.extend(annotations);
                TypeDclInner::TypedefDcl(decl)
            }
            crate::typed_ast::TypeDclInner::NativeDcl(native) => {
                let mut decl: NativeDcl = native.into();
                decl.annotations.extend(annotations);
                TypeDclInner::NativeDcl(decl)
            }
        };
        Self { decl }
    }
}

impl From<crate::typed_ast::TypeDclInner> for TypeDclInner {
    fn from(value: crate::typed_ast::TypeDclInner) -> Self {
        match value {
            crate::typed_ast::TypeDclInner::ConstrTypeDcl(constr) => {
                Self::ConstrTypeDcl(constr.into())
            }
            crate::typed_ast::TypeDclInner::TypedefDcl(typedef) => Self::TypedefDcl(typedef.into()),
            crate::typed_ast::TypeDclInner::NativeDcl(native_dcl) => {
                Self::NativeDcl(native_dcl.into())
            }
        }
    }
}

impl From<crate::typed_ast::TypedefDcl> for TypedefDcl {
    fn from(value: crate::typed_ast::TypedefDcl) -> Self {
        let ty = match value.decl.ty {
            crate::typed_ast::TypeDeclaratorInner::SimpleTypeSpec(simple) => {
                TypedefType::TypeSpec(crate::typed_ast::TypeSpec::SimpleTypeSpec(simple).into())
            }
            crate::typed_ast::TypeDeclaratorInner::TemplateTypeSpec(template) => {
                TypedefType::TypeSpec(crate::typed_ast::TypeSpec::TemplateTypeSpec(template).into())
            }
            crate::typed_ast::TypeDeclaratorInner::ConstrTypeDcl(constr) => {
                TypedefType::ConstrTypeDcl(constr.into())
            }
        };
        let decl = value
            .decl
            .decl
            .0
            .into_iter()
            .map(|decl| match decl {
                crate::typed_ast::AnyDeclarator::SimpleDeclarator(simple) => {
                    Declarator::SimpleDeclarator(simple.into())
                }
                crate::typed_ast::AnyDeclarator::ArrayDeclarator(array) => {
                    Declarator::ArrayDeclarator(array.into())
                }
            })
            .collect();
        Self {
            annotations: vec![],
            ty,
            decl,
        }
    }
}

impl From<crate::typed_ast::NativeDcl> for NativeDcl {
    fn from(value: crate::typed_ast::NativeDcl) -> Self {
        Self {
            annotations: vec![],
            decl: value.decl.into(),
        }
    }
}
