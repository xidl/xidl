mod definition;
mod interface;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::generate::utils::go_package_name;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
use crate::macros::hashmap;
pub use render::GoRestRenderer;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir::ParserProperties;

pub fn generate(
    rest_hir: xidl_parser::rest_hir::RestHirDocument,
    input_path: &Path,
    _props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let spec = rest_hir.spec.clone();
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let package = rest_hir
        .document
        .package
        .as_deref()
        .map(go_package_name)
        .unwrap_or_else(|| go_package_name(stem));
    let filename = format!("{}_http.go", package);
    let content = spec::render_spec(&spec, &package, &rest_hir)?;

    let mut artifacts = vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })];

    let non_interface = definition::strip_interfaces(spec);
    if !non_interface.0.is_empty() {
        let props = hashmap! {
            "enable_interfaces" => false,
            "enable_render_header" => false,
            "enable_metadata" => false,
            "package" => serde_json::Value::from(package)
        };
        artifacts.push(Artifact::new_hir(ArtifactHir {
            lang: "go".into(),
            hir: non_interface,
            props,
        }));
    }

    Ok(artifacts)
}

pub(crate) struct GoRestCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for GoRestCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(hashmap! {
            "expand_interface" => false,
            "hir_kind" => "http",
            "enable_client" => true,
            "enable_server" => true,
            "enable_render_header" => true,
            "enable_metadata" => true
        })
    }

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let rest_hir = input_hir.into_rest_hir();
        generate(rest_hir, Path::new(&input), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamSource {
    Path,
    Query,
    Header,
    Cookie,
    Body,
}

#[derive(Clone)]
pub(crate) struct ParamMeta {
    pub(crate) field_name: String,
    pub(crate) raw_name: String,
    pub(crate) wire_name: String,
    pub(crate) ty: String,
    pub(crate) optional: bool,
    pub(crate) source: ParamSource,
}

pub(crate) struct MethodMeta {
    pub(crate) method_name: String,
    pub(crate) struct_prefix: String,
    pub(crate) http_method: HttpMethod,
    pub(crate) paths: Vec<String>,
    pub(crate) request_struct: String,
    pub(crate) request_body_struct: Option<String>,
    pub(crate) request_body_direct_field: Option<String>,
    pub(crate) request_body_direct_ty: Option<String>,
    pub(crate) response_struct: String,
    pub(crate) response_body_struct: Option<String>,
    pub(crate) response_body_direct_field: Option<String>,
    pub(crate) response_body_direct_ty: Option<String>,
    pub(crate) request_content_type: String,
    pub(crate) response_content_type: String,
    pub(crate) request_params: Vec<ParamMeta>,
    pub(crate) path_params: Vec<ParamMeta>,
    pub(crate) query_params: Vec<ParamMeta>,
    pub(crate) header_params: Vec<ParamMeta>,
    pub(crate) cookie_params: Vec<ParamMeta>,
    pub(crate) body_params: Vec<ParamMeta>,
    pub(crate) response_body_params: Vec<ParamMeta>,
    pub(crate) response_header_params: Vec<ParamMeta>,
    pub(crate) response_cookie_params: Vec<ParamMeta>,
    pub(crate) return_ty: Option<String>,
    pub(crate) stream_kind: Option<xidl_parser::rest_hir::semantics::HttpStreamKind>,
    pub(crate) stream_codec: xidl_parser::rest_hir::semantics::HttpStreamCodec,
    pub(crate) security: Vec<xidl_parser::rest_hir::semantics::HttpSecurityRequirement>,
    pub(crate) basic_realm: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) deprecated_since: Option<String>,
    pub(crate) deprecated_after: Option<String>,
    pub(crate) deprecated_note: Option<String>,
}

#[derive(Serialize)]
pub struct GoRestRenderOutput {
    package_name: String,
    blocks: Vec<String>,
}
