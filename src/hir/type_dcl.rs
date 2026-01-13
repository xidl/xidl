use super::*;

pub struct TypeDcl(pub Vec<TypeDclInner>);

pub enum TypeDclInner {
    ConstrTypeDcl(ConstrTypeDcl),
    TypedefDcl(TypedefDcl),
    NativeDcl(NativeDcl),
}

pub struct TypedefDcl {
    pub ty: TypedefType,
    pub decl: Vec<Declarator>,
}

pub enum TypedefType {
    TypeSpec(TypeSpec),
    ConstrTypeDcl(ConstrTypeDcl),
}

pub struct NativeDcl {
    pub decl: SimpleDeclarator,
}

impl From<crate::typed_ast::TypeDcl> for TypeDcl {
    fn from(value: crate::typed_ast::TypeDcl) -> Self {
        Self(value.0.into_iter().map(Into::into).collect())
    }
}

impl From<crate::typed_ast::TypeDclInner> for TypeDclInner {
    fn from(value: crate::typed_ast::TypeDclInner) -> Self {
        match value {
            crate::typed_ast::TypeDclInner::ConstrTypeDcl(constr) => {
                Self::ConstrTypeDcl(constr.into())
            }
            crate::typed_ast::TypeDclInner::TypedefDcl(typedef) => {
                Self::TypedefDcl(typedef.into())
            }
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
        Self { ty, decl }
    }
}

impl From<crate::typed_ast::NativeDcl> for NativeDcl {
    fn from(value: crate::typed_ast::NativeDcl) -> Self {
        Self {
            decl: value.decl.into(),
        }
    }
}
