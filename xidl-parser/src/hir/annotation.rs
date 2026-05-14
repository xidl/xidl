use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Annotation {
    Id {
        value: String,
    },
    Key {
        value: Option<String>,
    },
    AutoId {
        value: Option<String>,
    },
    Optional {
        value: Option<String>,
    },
    Position {
        value: String,
    },
    Value {
        value: String,
    },
    Extensibility {
        kind: String,
    },
    Final,
    Appendable,
    Mutable,
    MustUnderstand {
        value: Option<String>,
    },
    Default {
        value: String,
    },
    Range {
        min: String,
        max: String,
    },
    Min {
        value: String,
    },
    Max {
        value: String,
    },
    Unit {
        value: String,
    },
    BitBound {
        value: String,
    },
    External {
        value: Option<String>,
    },
    Nested {
        value: Option<String>,
    },
    Verbatim {
        language: Option<String>,
        placement: Option<String>,
        text: String,
    },
    Service {
        platform: Option<String>,
    },
    Oneway {
        value: Option<String>,
    },
    Ami {
        value: Option<String>,
    },
    HashId {
        value: Option<String>,
    },
    DefaultNested {
        value: Option<String>,
    },
    IgnoreLiteralNames {
        value: Option<String>,
    },
    TryConstruct {
        value: Option<String>,
    },
    NonSerialized {
        value: Option<String>,
    },
    DataRepresentation {
        kinds: Vec<String>,
    },
    Topic {
        name: Option<String>,
        platform: Option<String>,
    },
    Choice,
    Empty,
    DdsService,
    DdsRequestTopic {
        name: String,
    },
    DdsReplyTopic {
        name: String,
    },
    Builtin {
        name: String,
        params: Option<AnnotationParams>,
    },
    ScopedName {
        name: ScopedName,
        params: Option<AnnotationParams>,
    },
    DefaultLiteral,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AnnotationParams {
    ConstExpr(ConstExpr),
    Positional(Vec<ConstExpr>),
    Params(Vec<AnnotationParam>),
    Raw(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnnotationParam {
    pub ident: String,
    pub value: Option<ConstExpr>,
}

pub fn annotation_id_value(annotations: &[Annotation]) -> Option<u32> {
    for annotation in annotations {
        if let Annotation::Id { value } = annotation {
            if let Ok(value) = value.parse::<u32>() {
                return Some(value);
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
            crate::typed_ast::AnnotationName::Builtin(name) => match value.builtin {
                Some(builtin) => super::annotation_builtin::from_builtin_annotation(builtin)
                    .unwrap_or(Self::Builtin { name, params }),
                None => Self::Builtin { name, params },
            },
        }
    }
}

impl From<crate::typed_ast::AnnotationParams> for AnnotationParams {
    fn from(value: crate::typed_ast::AnnotationParams) -> Self {
        match value {
            crate::typed_ast::AnnotationParams::Params(params) => {
                let mut positional = Vec::new();
                let mut named = Vec::new();
                for param in params {
                    match param {
                        crate::typed_ast::AnnotationApplParam::Positional(expr) => {
                            positional.push(expr.into());
                        }
                        crate::typed_ast::AnnotationApplParam::Named { ident, value } => {
                            named.push(AnnotationParam {
                                ident: ident.0,
                                value: Some(value.into()),
                            });
                        }
                    }
                }
                if !positional.is_empty() && named.is_empty() {
                    if positional.len() == 1 {
                        Self::ConstExpr(positional.remove(0))
                    } else {
                        Self::Positional(positional)
                    }
                } else {
                    Self::Params(named)
                }
            }
            crate::typed_ast::AnnotationParams::Raw(value) => Self::Raw(value),
        }
    }
}
