use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumDcl {
    pub ident: String,
    pub member: Vec<String>,
}

impl From<crate::typed_ast::EnumDcl> for EnumDcl {
    fn from(typed_ast: crate::typed_ast::EnumDcl) -> Self {
        Self {
            ident: typed_ast.ident.0,
            member: typed_ast.member.into_iter().map(|x| x.ident.0).collect(),
        }
    }
}
