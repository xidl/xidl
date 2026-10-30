mod definition;
mod interface;
mod render;
mod spec;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::go::{GoImport, GoPackages};
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
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    GoRestRenderMode::from_properties(&props)?;
    let spec = rest_hir.spec.clone();
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let unit_path = props
        .get(crate::units::UNIT_PATH_PROPERTY)
        .and_then(serde_json::Value::as_str);
    let package = match unit_path.and_then(|path| path.rsplit_once('/')) {
        Some((dir, _)) => {
            let name = dir.rsplit('/').next().unwrap_or(dir);
            go_package_name(if name.is_empty() { stem } else { name })
        }
        None => rest_hir
            .document
            .package
            .as_deref()
            .map(go_package_name)
            .unwrap_or_else(|| go_package_name(stem)),
    };
    let filename = match unit_path {
        Some(path) => format!("{}_http.go", path.trim_end_matches(".idl")),
        None => format!("{}_http.go", package),
    };
    let content = spec::render_spec(&spec, &package, &rest_hir, &props)?;

    let mut artifacts = vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })];

    let enable_metadata = props
        .get("enable_metadata")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::from(true));

    let non_interface = definition::strip_interfaces(spec);
    if !non_interface.0.is_empty() {
        let props = hashmap! {
            "enable_interfaces" => false,
            "enable_render_header" => false,
            "enable_metadata" => enable_metadata,
            "allow_versioned_modules" => serde_json::Value::Bool(true),
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

impl crate::jsonrpc::Codegen for GoRestCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(hashmap! {
            "hir_kind" => "http",
            "enable_client" => true,
            "enable_server" => true,
            "enable_render_header" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let rest_hir = input_hir.into_rest_hir();
        generate(rest_hir, Path::new(&input), props)
            .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct GoRestRenderMode {
    pub(crate) enable_client: bool,
    pub(crate) enable_server: bool,
}

impl GoRestRenderMode {
    pub(crate) fn from_properties(props: &HashMap<String, serde_json::Value>) -> IdlcResult<Self> {
        let enable_client = props
            .get("enable_client")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        let enable_server = props
            .get("enable_server")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);
        if !enable_client && !enable_server {
            return Err(IdlcError::rpc(
                "go-rest requires enable_client or enable_server",
            ));
        }
        Ok(Self {
            enable_client,
            enable_server,
        })
    }
}

pub(crate) struct GoRestRenderContext<'a> {
    pub(crate) renderer: &'a GoRestRenderer,
    pub(crate) rest_hir: &'a xidl_parser::rest_hir::RestHirDocument,
    pub(crate) mode: GoRestRenderMode,
    pub(crate) packages: GoPackages,
    pub(crate) versioned_modules: bool,
}

impl GoRestRenderContext<'_> {
    pub(crate) fn version_parameter(&self, module_path: &[String]) -> Option<VersionParameterMeta> {
        let parameter = self.rest_hir.document.version_parameter.as_ref()?;
        let value = self.rest_hir.document.api_version(module_path)?.to_string();
        Some(VersionParameterMeta {
            location: parameter.location,
            name: parameter.name.clone(),
            value,
        })
    }

    pub(crate) fn versioned_modules(&self) -> bool {
        self.versioned_modules
    }
}

#[derive(Clone)]
pub(crate) struct VersionParameterMeta {
    pub(crate) location: xidl_parser::hir::HttpParameterLocation,
    pub(crate) name: String,
    pub(crate) value: String,
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
    pub(crate) requires_presence: bool,
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
    pub(crate) request_content_type_explicit: bool,
    pub(crate) response_content_type: String,
    pub(crate) response_content_type_explicit: bool,
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
    pub(crate) exceptions: Vec<interface::ExceptionContext>,
    pub(crate) stream_kind: Option<xidl_parser::rest_hir::semantics::HttpStreamKind>,
    pub(crate) is_websocket: bool,
    pub(crate) websocket_subprotocol: Option<String>,
    pub(crate) stream_in_ty: String,
    pub(crate) stream_out_ty: String,
    pub(crate) handshake_params: Vec<ParamMeta>,
    pub(crate) version_parameter: Option<VersionParameterMeta>,
    pub(crate) cors: Option<xidl_parser::rest_hir::semantics::HttpCorsProfile>,
    pub(crate) security: Vec<xidl_parser::rest_hir::semantics::HttpSecurityRequirement>,
    pub(crate) basic_realm: Option<String>,
    pub(crate) deprecated: bool,
    pub(crate) deprecated_since: Option<String>,
    pub(crate) deprecated_after: Option<String>,
    pub(crate) deprecated_note: Option<String>,
}

#[derive(Default, Serialize)]
pub(crate) struct GoRestRenderBlocks {
    server: Vec<String>,
    client: Vec<String>,
    shared: Vec<String>,
    has_exceptions: bool,
    has_exception_body: bool,
}

impl GoRestRenderBlocks {
    pub(crate) fn extend(&mut self, other: Self) {
        self.server.extend(other.server);
        self.client.extend(other.client);
        self.shared.extend(other.shared);
        self.has_exceptions |= other.has_exceptions;
        self.has_exception_body |= other.has_exception_body;
    }

    pub(crate) fn push_server(&mut self, block: String) {
        if !block.is_empty() {
            self.server.push(block);
        }
    }

    pub(crate) fn push_client(&mut self, block: String) {
        if !block.is_empty() {
            self.client.push(block);
        }
    }

    pub(crate) fn push_shared(&mut self, block: String) {
        if !block.is_empty() {
            self.shared.push(block);
        }
    }
}

#[derive(Serialize)]
pub(crate) struct GoRestRenderOutput {
    package_name: String,
    blocks: GoRestRenderBlocks,
    opt: ParserProperties,
    imports: Vec<GoImport>,
}
