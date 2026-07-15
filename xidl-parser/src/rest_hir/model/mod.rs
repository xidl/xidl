use crate::error::{ParseError, ParserResult};
use crate::hir;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpParamKind {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpOperationSource {
    Method,
    AttributeGet,
    AttributeSet,
    AttributeWatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpRoute {
    pub path: String,
    pub path_params: Vec<String>,
    pub query_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpParam {
    pub name: String,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
    pub kind: HttpParamKind,
    pub optional: bool,
    pub flatten: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpDocumentServer {
    pub base_url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HttpDocumentMetadata {
    pub package: Option<String>,
    pub version: Option<String>,
    pub servers: Vec<HttpDocumentServer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperation {
    pub meta: HttpOperationMeta,
    pub signature: HttpOperationSignature,
    pub http: HttpOperationHttpMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperationMeta {
    pub name: String,
    pub operation_id: String,
    pub source: HttpOperationSource,
    pub method: HttpMethod,
    pub routes: Vec<HttpRoute>,
    pub stream: super::semantics::HttpStreamConfig,
    pub cors: Option<super::semantics::HttpCorsProfile>,
    pub security: Option<super::semantics::HttpSecurityProfile>,
    pub basic_auth_realm: Option<String>,
    pub deprecated: Option<super::semantics::DeprecatedInfo>,
    pub upgrade_protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperationSignature {
    pub params: Vec<HttpSignatureParam>,
    pub return_type: Option<hir::TypeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSignatureParam {
    pub name: String,
    pub ty: hir::TypeSpec,
    pub direction: HttpSignatureParamDirection,
    pub is_optional: bool,
    pub is_flatten: bool,
    pub annotations: Vec<HttpSignatureParamAnnotation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpSignatureParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpSignatureParamAnnotation {
    Optional,
    Flatten,
    Path { name: String },
    Query { name: String },
    Header { name: String },
    Cookie { name: String },
    Body,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperationHttpMapping {
    pub request: HttpRequestMapping,
    pub response: HttpResponseMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestMapping {
    pub path: Vec<HttpInputBinding>,
    pub query: Vec<HttpInputBinding>,
    pub header: Vec<HttpInputBinding>,
    pub cookie: Vec<HttpInputBinding>,
    pub body: HttpRequestBodyMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInputBinding {
    pub source_param: String,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestBodyMapping {
    pub content_type: Option<String>,
    pub content_type_explicit: bool,
    pub codec: Option<HttpBodyCodec>,
    pub shape: HttpRequestBodyShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpRequestBodyShape {
    Empty,
    SingleValue {
        source_param: String,
        flatten: bool,
        ty: hir::TypeSpec,
    },
    Object {
        fields: Vec<HttpRequestBodyField>,
    },
    Stream {
        source_param: String,
        item_ty: hir::TypeSpec,
        codec: HttpStreamPayloadCodec,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRequestBodyField {
    pub source_param: String,
    pub field_name: String,
    pub ty: hir::TypeSpec,
    pub optional: bool,
    pub flatten: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseMapping {
    pub header: Vec<HttpOutputBinding>,
    pub cookie: Vec<HttpOutputBinding>,
    pub body: HttpResponseBodyMapping,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOutputBinding {
    pub source: HttpOutputSource,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpOutputSource {
    ReturnValue,
    Param { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseBodyMapping {
    pub content_type: Option<String>,
    pub content_type_explicit: bool,
    pub codec: Option<HttpBodyCodec>,
    pub shape: HttpResponseBodyShape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpResponseBodyShape {
    Empty,
    ReturnOnly {
        ty: hir::TypeSpec,
    },
    SingleValue {
        source: HttpOutputSource,
        ty: hir::TypeSpec,
    },
    Object {
        fields: Vec<HttpResponseBodyField>,
    },
    Stream {
        item_source: HttpOutputSource,
        item_ty: hir::TypeSpec,
        codec: HttpStreamPayloadCodec,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpResponseBodyField {
    pub source: HttpOutputSource,
    pub field_name: String,
    pub ty: hir::TypeSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpBodyCodec {
    Json,
    Text,
    FormUrlEncoded,
    Msgpack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpStreamPayloadCodec {
    Ndjson,
    Sse,
    Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInterface {
    pub name: String,
    pub module_path: Vec<String>,
    pub operations: Vec<HttpOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestHirDocument {
    pub spec: hir::Specification,
    pub document: HttpDocumentMetadata,
    pub interfaces: Vec<HttpInterface>,
}

impl RestHirDocument {
    pub fn from_props(props: &hir::ParserProperties) -> ParserResult<Self> {
        let value = props
            .get("rest_hir")
            .cloned()
            .ok_or_else(|| ParseError::Message("missing rest_hir properties".to_string()))?;
        serde_json::from_value(value).map_err(|err| ParseError::Message(err.to_string()))
    }

    pub fn find_interface(&self, module_path: &[String], name: &str) -> Option<&HttpInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.name == name && interface.module_path == module_path)
    }
}
