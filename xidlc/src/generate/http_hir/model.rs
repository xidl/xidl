use crate::error::{IdlcError, IdlcResult};
use serde::{Deserialize, Serialize};
use xidl_parser::hir;

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
pub enum HttpParamDirection {
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpParamSource {
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
    pub direction: HttpParamDirection,
    pub source: HttpParamSource,
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
    pub name: String,
    pub operation_id: String,
    pub source: HttpOperationSource,
    pub method: HttpMethod,
    pub routes: Vec<HttpRoute>,
    pub stream: super::semantics::HttpStreamConfig,
    pub request_content_type: String,
    pub response_content_type: String,
    pub security: Option<super::semantics::HttpSecurityProfile>,
    pub basic_auth_realm: Option<String>,
    pub deprecated: Option<super::semantics::DeprecatedInfo>,
    pub request_params: Vec<HttpParam>,
    pub request_path_params: Vec<HttpParam>,
    pub request_query_params: Vec<HttpParam>,
    pub request_header_params: Vec<HttpParam>,
    pub request_cookie_params: Vec<HttpParam>,
    pub request_body_params: Vec<HttpParam>,
    pub response_params: Vec<HttpParam>,
    pub response_header_params: Vec<HttpParam>,
    pub response_cookie_params: Vec<HttpParam>,
    pub response_body_params: Vec<HttpParam>,
    pub return_type: Option<hir::TypeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpInterface {
    pub name: String,
    pub module_path: Vec<String>,
    pub operations: Vec<HttpOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HttpHirDocument {
    pub document: HttpDocumentMetadata,
    pub interfaces: Vec<HttpInterface>,
}

impl HttpHirDocument {
    pub fn from_props(props: &hir::ParserProperties) -> IdlcResult<Self> {
        let value = props
            .get("http_hir")
            .cloned()
            .ok_or_else(|| IdlcError::rpc("missing http_hir properties".to_string()))?;
        serde_json::from_value(value).map_err(|err| IdlcError::rpc(err.to_string()))
    }

    pub fn find_interface(&self, module_path: &[String], name: &str) -> Option<&HttpInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.name == name && interface.module_path == module_path)
    }
}
