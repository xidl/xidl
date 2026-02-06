use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub member: Vec<Enumerator>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enumerator {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

impl EnumDcl {
    pub fn serialize_kind(&self, config: &SerializeConfig) -> SerializeKind {
        config.resolve_for_annotations(&self.annotations)
    }
}

impl From<crate::typed_ast::EnumDcl> for EnumDcl {
    fn from(typed_ast: crate::typed_ast::EnumDcl) -> Self {
        Self {
            annotations: vec![],
            ident: typed_ast.ident.0,
            member: typed_ast.member.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::typed_ast::Enumerator> for Enumerator {
    fn from(value: crate::typed_ast::Enumerator) -> Self {
        Self {
            annotations: expand_annotations(value.annotations),
            ident: value.ident.0,
        }
    }
}
