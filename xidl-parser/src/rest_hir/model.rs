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
pub enum HttpBodyShape {
    None,
    /// Only one body parameter, and it's flattened (the body itself is the parameter).
    SingleFlattened,
    /// Only one body parameter, not flattened.
    Single,
    /// Multiple body parameters, wrapped in an object.
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpRequestShape {
    None,
    /// One or more parameters (body, query, etc) that require a wrapper struct/object.
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpResponseShape {
    None,
    /// Only the return value, no out parameters.
    ReturnOnly,
    /// Return value and out parameters, or multiple out parameters, or single out parameter with no return value.
    Object,
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
    pub name: String,
    pub operation_id: String,
    pub source: HttpOperationSource,
    pub method: HttpMethod,
    pub routes: Vec<HttpRoute>,
    pub stream: super::semantics::HttpStreamConfig,
    pub request_content_type: String,
    pub response_content_type: String,
    pub request_shape: HttpRequestShape,
    pub response_shape: HttpResponseShape,
    pub request_body_shape: HttpBodyShape,
    pub response_body_shape: HttpBodyShape,
    pub security: Option<super::semantics::HttpSecurityProfile>,
    pub basic_auth_realm: Option<String>,
    pub deprecated: Option<super::semantics::DeprecatedInfo>,
    pub request_params: Vec<HttpParam>,
    pub response_params: Vec<HttpParam>,
    pub return_type: Option<hir::TypeSpec>,
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
