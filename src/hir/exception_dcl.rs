use super::*;

#[derive(Debug)]
pub struct ExceptDcl {
    pub ident: String,
    pub member: Vec<Member>,
}

impl From<crate::typed_ast::ExceptDcl> for ExceptDcl {
    fn from(value: crate::typed_ast::ExceptDcl) -> Self {
        Self {
            ident: value.ident.0,
            member: value.member.into_iter().map(Into::into).collect(),
        }
    }
}
