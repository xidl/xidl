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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpDocumentMetadata {
    pub package: Option<String>,
    pub version: Option<String>,
    pub servers: Vec<HttpDocumentServer>,
    /// Document-level `exception` declarations projected for HTTP use.
    #[serde(default)]
    pub exceptions: Vec<HttpException>,
    /// Document-level `@http` unions: content-negotiated representations.
    #[serde(default)]
    pub http_unions: Vec<HttpUnion>,
}

/// An `exception` declaration with its HTTP error-channel semantics.
///
/// Members annotated `@header`/`@cookie` become the error response's
/// headers/cookies; unannotated members form the JSON error body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpException {
    pub module_path: Vec<String>,
    pub ident: String,
    /// HTTP status code bound via `@http(<status>)` / `@http(status = <status>)`.
    pub status: u16,
    pub headers: Vec<HttpExceptionMember>,
    pub cookies: Vec<HttpExceptionMember>,
    pub body: Vec<HttpExceptionField>,
}

/// A `@header`/`@cookie` annotated member of an exception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExceptionMember {
    pub field: String,
    pub wire_name: String,
    pub ty: hir::TypeSpec,
    /// `sequence<...>` members repeat the header (append) instead of replacing it.
    pub is_multi: bool,
    /// `@optional` members may be absent: the field is `Option<_>` and the
    /// header is only written when a value exists.
    #[serde(default)]
    pub optional: bool,
}

/// An unannotated member of an exception: part of the JSON error body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExceptionField {
    pub field: String,
    /// JSON property name after applying the existing naming annotations.
    pub wire_name: String,
    /// Whether the JSON property may be absent.
    pub optional: bool,
    pub ty: hir::TypeSpec,
}

/// A `raises(...)` entry on an operation, resolved to a declared exception.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpExceptionRef {
    pub module_path: Vec<String>,
    pub ident: String,
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
    pub upgrade_mode: Option<super::semantics::UpgradeMode>,
    pub websocket: Option<super::semantics::WebSocketConfig>,
    /// Exceptions declared via `raises(...)` on this operation.
    #[serde(default)]
    pub raises: Vec<HttpExceptionRef>,
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
    /// Non-empty when the return type is an `@http` union: the success
    /// response is content-negotiated across these representations.
    #[serde(default)]
    pub representations: Vec<HttpRepresentation>,
}

/// One content-negotiated representation of an `@http` union response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRepresentation {
    /// Shared protocol value; both the case name and media type derive from it.
    pub content_type: xidl_http::ContentType,
    /// Case payload type.
    pub ty: hir::TypeSpec,
    /// `sequence<octet>` cases respond with raw bytes.
    pub is_byte: bool,
}

/// An `@http` union declaration: HTTP representation switching (#296).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpUnion {
    pub module_path: Vec<String>,
    pub ident: String,
    pub cases: Vec<HttpRepresentation>,
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
