use serde::{Deserialize, Serialize};

use crate::hir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcHirDocument {
    pub spec: hir::Specification,
    pub interfaces: Vec<JsonRpcInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcInterface {
    pub ident: String,
    pub module_path: Vec<String>,
    pub annotations: Vec<hir::Annotation>,
    pub methods: Vec<JsonRpcMethod>,
    pub watch_methods: Vec<JsonRpcWatchMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcMethod {
    pub source: JsonRpcMethodSource,
    pub kind: JsonRpcMethodKind,
    pub name: String,
    pub rpc_name: String,
    pub annotations: Vec<hir::Annotation>,
    pub request_fields: Vec<JsonRpcField>,
    pub response_fields: Vec<JsonRpcField>,
    pub response_kind: JsonRpcResponseKind,
    pub stream_item: Option<hir::TypeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcWatchMethod {
    pub getter_name: String,
    pub item_ty: hir::TypeSpec,
    pub stream_rpc_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonRpcMethodSource {
    Operation,
    AttributeGet,
    AttributeSet,
    AttributeStreamSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonRpcMethodKind {
    Unary,
    ServerStream,
    ClientStream,
    BidiStream,
    StreamSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonRpcResponseKind {
    Empty,
    SingleReturn,
    SingleOutput,
    MultiOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcField {
    pub name: String,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
    pub annotations: Vec<hir::Annotation>,
    pub required: bool,
    pub source: JsonRpcFieldSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JsonRpcFieldSource {
    Return,
    Param,
}
