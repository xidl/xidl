use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum TypeDcl {
    ConstrTypeDcl(ConstrTypeDcl),
    TypedefDcl(TypedefDcl),
    NativeDcl(NativeDcl),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TypedefDcl {
    pub annotations: Vec<Annotation>,
    pub ty: TypedefType,
    pub decl: Vec<Declarator>,
    /// The aliased type names a `@deprecated` declaration. Populated by
    /// semantic analysis so generated references can silence rustc's
    /// `deprecated` lint.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub references_deprecated: bool,
}

impl std::fmt::Debug for TypedefDcl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("TypedefDcl");
        debug
            .field("annotations", &self.annotations)
            .field("ty", &self.ty)
            .field("decl", &self.decl);
        // Aliases that reach no deprecated declaration keep the historical
        // debug shape so stored snapshots describe only flagged aliases.
        if self.references_deprecated {
            debug.field("references_deprecated", &true);
        }
        debug.finish()
    }
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
        match value.decl {
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
                TypeDcl::ConstrTypeDcl(decl)
            }
            crate::typed_ast::TypeDclInner::TypedefDcl(typedef) => {
                let mut decl: TypedefDcl = typedef.into();
                decl.annotations.extend(annotations);
                TypeDcl::TypedefDcl(decl)
            }
            crate::typed_ast::TypeDclInner::NativeDcl(native) => {
                let mut decl: NativeDcl = native.into();
                decl.annotations.extend(annotations);
                TypeDcl::NativeDcl(decl)
            }
        }
    }
}

impl From<crate::typed_ast::TypeDclInner> for TypeDcl {
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
            references_deprecated: false,
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
