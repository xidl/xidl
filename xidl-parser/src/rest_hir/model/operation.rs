use serde::{Deserialize, Serialize};

use crate::rest_hir::semantics;

use super::{
    HttpExceptionRef, HttpMethod, HttpOperationHttpMapping, HttpOperationPublication,
    HttpOperationSignature, HttpOperationSource, HttpRoute,
};

/// A projected operation with a shared signature and HTTP mapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperation {
    pub meta: HttpOperationMeta,
    pub signature: HttpOperationSignature,
    pub http: HttpOperationHttpMapping,
}

/// Operation identity, publication, routes and protocol metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOperationMeta {
    pub name: String,
    pub operation_id: String,
    /// Document publication only; request and response semantics are unchanged.
    #[serde(default, skip_serializing_if = "HttpOperationPublication::is_path")]
    pub publication: HttpOperationPublication,
    pub source: HttpOperationSource,
    pub method: HttpMethod,
    pub routes: Vec<HttpRoute>,
    pub stream: semantics::HttpStreamConfig,
    pub cors: Option<semantics::HttpCorsProfile>,
    pub security: Option<semantics::HttpSecurityProfile>,
    pub basic_auth_realm: Option<String>,
    pub deprecated: Option<semantics::DeprecatedInfo>,
    pub upgrade_protocol: Option<String>,
    pub upgrade_mode: Option<semantics::UpgradeMode>,
    pub websocket: Option<semantics::WebSocketConfig>,
    /// Exceptions declared via `raises(...)` on this operation.
    #[serde(default)]
    pub raises: Vec<HttpExceptionRef>,
}
