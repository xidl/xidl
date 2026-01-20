use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Annotation {
    Id {
        value: ConstExpr,
    },
    Key,
    Builtin {
        name: String,
        params: Option<AnnotationParams>,
    },
    ScopedName {
        name: ScopedName,
        params: Option<AnnotationParams>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnnotationParams {
    ConstExpr(ConstExpr),
    Params(Vec<AnnotationParam>),
    Raw(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnnotationParam {
    pub ident: String,
    pub value: Option<ConstExpr>,
}

pub fn annotation_id_value(annotations: &[Annotation]) -> Option<u32> {
    for annotation in annotations {
        if let Annotation::Id { value } = annotation {
            if let Some(value) = super::expr::const_expr_to_i64(value) {
                if value >= 0 && value <= u32::MAX as i64 {
                    return Some(value as u32);
                }
            }
        }
    }
    None
}

pub fn expand_annotations(values: Vec<crate::typed_ast::AnnotationAppl>) -> Vec<Annotation> {
    let mut out = Vec::new();
    for value in values {
        push_annotation(&mut out, value);
    }
    out
}

fn push_annotation(out: &mut Vec<Annotation>, mut value: crate::typed_ast::AnnotationAppl) {
    let extra = std::mem::take(&mut value.extra);
    out.push(Annotation::from(value));
    for item in extra {
        push_annotation(out, item);
    }
}

impl From<crate::typed_ast::AnnotationAppl> for Annotation {
    fn from(value: crate::typed_ast::AnnotationAppl) -> Self {
        let params = value.params.map(Into::into);
        match value.name {
            crate::typed_ast::AnnotationName::ScopedName(name) => Self::ScopedName {
                name: name.into(),
                params,
            },
            crate::typed_ast::AnnotationName::Builtin(name) => {
                if name.eq_ignore_ascii_case("id") {
                    if let Some(AnnotationParams::ConstExpr(expr)) = &params {
                        return Self::Id {
                            value: expr.clone(),
                        };
                    }
                } else if name.eq_ignore_ascii_case("key") {
                    if params.is_none() {
                        return Self::Key;
                    }
                }
                Self::Builtin { name, params }
            }
        }
    }
}

impl From<crate::typed_ast::AnnotationParams> for AnnotationParams {
    fn from(value: crate::typed_ast::AnnotationParams) -> Self {
        match value {
            crate::typed_ast::AnnotationParams::ConstExpr(expr) => Self::ConstExpr(expr.into()),
            crate::typed_ast::AnnotationParams::Params(params) => {
                Self::Params(params.into_iter().map(Into::into).collect())
            }
            crate::typed_ast::AnnotationParams::Raw(value) => Self::Raw(value),
        }
    }
}

impl From<crate::typed_ast::AnnotationApplParam> for AnnotationParam {
    fn from(value: crate::typed_ast::AnnotationApplParam) -> Self {
        Self {
            ident: value.ident.0,
            value: value.value.map(Into::into),
        }
    }
}
