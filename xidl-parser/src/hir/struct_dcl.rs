use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct StructForwardDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Member {
    pub annotations: Vec<Annotation>,
    pub ty: TypeSpec,
    pub ident: Vec<Declarator>,
    pub default: Option<Default>,
    pub field_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Default(pub ConstExpr);

impl StructDcl {
    pub fn serialize_kind(&self, config: &SerializeConfig) -> SerializeKind {
        config.resolve_for_annotations(&self.annotations)
    }
}

impl From<crate::typed_ast::StructDef> for StructDcl {
    fn from(value: crate::typed_ast::StructDef) -> Self {
        let mut members = value
            .member
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Member>>();
        for (index, member) in members.iter_mut().enumerate() {
            if member.field_id.is_none() {
                member.field_id = Some((index + 1) as u32);
            }
        }
        Self {
            annotations: vec![],
            ident: value.ident.0,
            parent: value.parent.into_iter().map(Into::into).collect(),
            member: members,
        }
    }
}

impl From<crate::typed_ast::Member> for Member {
    fn from(value: crate::typed_ast::Member) -> Self {
        let annotations = expand_annotations(value.annotations);
        let field_id = annotation_id_value(&annotations);
        Self {
            annotations,
            ty: value.ty.into(),
            ident: value.ident.0.into_iter().map(Into::into).collect(),
            default: value.default.map(Into::into),
            field_id,
        }
    }
}

impl From<crate::typed_ast::StructForwardDcl> for StructForwardDcl {
    fn from(typed_ast: crate::typed_ast::StructForwardDcl) -> Self {
        Self {
            annotations: vec![],
            ident: typed_ast.ident.0,
        }
    }
}

impl From<crate::typed_ast::Default> for Default {
    fn from(value: crate::typed_ast::Default) -> Self {
        Self(value.0.into())
    }
}
