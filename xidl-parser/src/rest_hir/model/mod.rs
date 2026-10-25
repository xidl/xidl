use crate::hir;
use serde::{Deserialize, Serialize};

mod document;
mod references;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

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

/// Inputs the HIR stage passes down that the specification itself does not carry.
#[derive(Debug, Default)]
pub struct ProjectOptions {
    /// The API version parameter declared by the source timeline.
    pub version_parameter: Option<hir::VersionParameter>,
    /// Version module idents and their wire values for this projection.
    pub version_values: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpDocumentMetadata {
    pub package: Option<String>,
    pub version: Option<String>,
    pub servers: Vec<HttpDocumentServer>,
    /// API version parameter declaration. Typed stage metadata, not a pragma.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_parameter: Option<hir::VersionParameter>,
    /// Version module ident to wire value, so merged-layout output can
    /// resolve the version of its enclosing module.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub version_values: BTreeMap<String, String>,
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
    /// The exception's own or enclosing module's `@deprecated`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<super::semantics::DeprecatedInfo>,
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
    /// The parameter's own `@deprecated`, when declared on it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<super::semantics::DeprecatedInfo>,
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

impl HttpInputBinding {
    /// Whether this binding requires at least one wire occurrence.
    ///
    /// Unbounded non-path sequences encode an empty list with zero occurrences.
    /// This does not make the logical signature parameter optional. Other input
    /// shapes retain their existing presence declaration.
    pub fn requires_presence(&self, location: HttpParamKind) -> bool {
        location == HttpParamKind::Path
            || (!self.optional
                && !matches!(&self.ty, hir::TypeSpec::SequenceType(sequence) if sequence.len.is_none()))
    }
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
        #[serde(skip)]
        wire_name: String,
        flatten: bool,
        ty: hir::TypeSpec,
    },
    Object {
        fields: Vec<HttpRequestBodyField>,
    },
    Stream {
        source_param: String,
        #[serde(skip)]
        wire_name: String,
        item_ty: hir::TypeSpec,
        codec: HttpStreamPayloadCodec,
    },
}

impl HttpRequestBodyShape {
    /// Returns the source parameter carried as a single raw byte body.
    pub fn byte_parameter(&self) -> Option<&str> {
        match self {
            Self::SingleValue {
                source_param, ty, ..
            } if super::is_byte_sequence(ty) => Some(source_param),
            _ => None,
        }
    }
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
