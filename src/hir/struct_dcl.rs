use super::*;

pub struct StructForwardDcl {
    pub ident: String,
}

pub struct StructDcl {
    pub ident: String,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

pub struct Member {
    pub ty: TypeSpec,
    pub ident: Vec<Declarator>,
    pub default: Option<Default>,
}

pub struct Default(pub ConstExpr);

impl From<crate::typed_ast::StructDef> for StructDcl {
    fn from(value: crate::typed_ast::StructDef) -> Self {
        Self {
            ident: value.ident.0,
            parent: value.parent.into_iter().map(Into::into).collect(),
            member: value.member.into_iter().map(|x| x.into()).collect(),
        }
    }
}

impl From<crate::typed_ast::Member> for Member {
    fn from(value: crate::typed_ast::Member) -> Self {
        Self {
            ty: value.ty.into(),
            ident: value.ident.0.into_iter().map(Into::into).collect(),
            default: value.default.map(Into::into),
        }
    }
}

impl From<crate::typed_ast::StructForwardDcl> for StructForwardDcl {
    fn from(typed_ast: crate::typed_ast::StructForwardDcl) -> Self {
        Self {
            ident: typed_ast.ident.0,
        }
    }
}

impl From<crate::typed_ast::Default> for Default {
    fn from(value: crate::typed_ast::Default) -> Self {
        Self(value.0)
    }
}
