use super::*;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StructForwardDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StructDcl {
    pub annotations: Vec<Annotation>,
    pub ident: String,
    pub parent: Vec<ScopedName>,
    pub member: Vec<Member>,
    /// Members inherited from the single base struct, with wire names and known
    /// references bound in their declaring scope. Populated by semantic analysis.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inherited: Vec<Member>,
}

impl fmt::Debug for StructDcl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("StructDcl");
        debug
            .field("annotations", &self.annotations)
            .field("ident", &self.ident)
            .field("parent", &self.parent)
            .field("member", &self.member);
        // Structs without a base keep their historical debug shape so stored
        // snapshots describe only declarations that carry inherited members.
        if !self.inherited.is_empty() {
            debug.field("inherited", &self.inherited);
        }
        debug.finish()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Member {
    pub annotations: Vec<Annotation>,
    pub ty: TypeSpec,
    pub ident: Vec<Declarator>,
    pub default: Option<Default>,
    pub field_id: Option<u32>,
    pub recursive: bool,
    /// The member's type names a `@deprecated` declaration. Populated by
    /// semantic analysis so generated references can silence rustc's
    /// `deprecated` lint.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub references_deprecated: bool,
}

impl fmt::Debug for Member {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("Member");
        debug
            .field("annotations", &self.annotations)
            .field("ty", &self.ty)
            .field("ident", &self.ident)
            .field("default", &self.default)
            .field("field_id", &self.field_id)
            .field("recursive", &self.recursive);
        // Members that reach no deprecated declaration keep the historical
        // debug shape so stored snapshots describe only flagged members.
        if self.references_deprecated {
            debug.field("references_deprecated", &true);
        }
        debug.finish()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Default(pub ConstExpr);

impl Member {
    /// The declared default value, from either the `@default` annotation or
    /// the grammar's member default, rendered as it appears in source.
    pub fn default_value(&self) -> Option<String> {
        self.annotations
            .iter()
            .find_map(|annotation| match annotation {
                Annotation::Default { value } => Some(value.clone()),
                _ => None,
            })
            .or_else(|| {
                self.default
                    .as_ref()
                    .map(|default| render_annotation_const_expr(&default.0))
            })
    }

    pub fn is_optional(&self) -> bool {
        self.annotations.iter().any(|annotation| match annotation {
            Annotation::Optional { .. } => true,
            Annotation::ScopedName { name, .. } => name
                .name
                .last()
                .map(|value| value.eq_ignore_ascii_case("optional"))
                .unwrap_or(false),
            _ => false,
        })
    }
}

impl StructDcl {
    /// Members of this struct in wire order: inherited members first, then its
    /// own members.
    ///
    /// Inherited members are the single base struct's fields, copied into this
    /// struct by semantic analysis. A target that renders a flattened JSON shape
    /// must use this iterator instead of [`Self::member`].
    pub fn members(&self) -> impl Iterator<Item = &Member> {
        self.inherited.iter().chain(self.member.iter())
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
            inherited: Vec::new(),
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
            recursive: false,
            references_deprecated: false,
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
