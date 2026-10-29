//! Data passed to the adapter's wire decoder template.

use crate::generate::typescript::definition::contexts::{TsType, ZodSchema};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct WireDefinition {
    pub name: String,
    pub output: TsType,
    pub body: WireBody,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum WireBody {
    Value(ZodSchema),
    Object(WireObject),
    Union(Vec<WireObject>),
}

#[derive(Serialize)]
pub(super) struct WireObject {
    pub keys: Vec<String>,
    pub forbidden: Vec<String>,
    pub catch_all: bool,
    pub fields: Vec<WireField>,
}

#[derive(Serialize)]
pub(super) struct WireField {
    pub prop: String,
    pub optional: bool,
    pub schema: ZodSchema,
    pub input: WireInput,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "data")]
pub(super) enum WireInput {
    Named(String),
    Flatten {
        keys: Vec<String>,
        receives_rest: bool,
    },
}
