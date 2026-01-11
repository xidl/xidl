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
