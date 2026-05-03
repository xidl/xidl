mod definition;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use convert_case::{Case, Casing};
pub use render::GoRenderer;
use serde::Serialize;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub fn generate_with_properties(
    spec: &hir::Specification,
    input_path: &Path,
    properties: &ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let base = go_package_name(stem);
    let filename = format!("{base}.go");
    let source = spec::render_spec(spec, &base, properties)?;
    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content: source,
    })])
}

pub(crate) struct GoCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for GoCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(crate::macros::hashmap! {
            "expand_interface" => false,
            "enable_interfaces" => true,
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
        let hir = input_hir.into_rpc_hir();
        generate_with_properties(&hir, Path::new(&input), &props).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}

#[derive(Default)]
pub(crate) struct GoRenderState {
    pub(crate) uses_context: bool,
}

pub(crate) struct GoRenderContext {
    pub(crate) package_name: String,
    pub(crate) properties: ParserProperties,
    pub(crate) state: GoRenderState,
    pub(crate) body: String,
}

impl GoRenderContext {
    pub(crate) fn new(package_name: String, properties: ParserProperties) -> Self {
        Self {
            package_name,
            properties,
            state: GoRenderState::default(),
            body: String::new(),
        }
    }

    pub(crate) fn finish(self) -> GoRenderOutput {
        GoRenderOutput {
            package_name: self.package_name,
            uses_context: self.state.uses_context,
            body: self.body,
        }
    }
}

#[derive(Serialize)]
pub struct GoRenderOutput {
    package_name: String,
    uses_context: bool,
    body: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParamDirection {
    In,
    Out,
    InOut,
}

pub(crate) fn go_package_name(value: &str) -> String {
    let mut out = value.to_case(Case::Snake);
    out = out.replace('-', "_");
    if out.is_empty() {
        "xidl".to_string()
    } else {
        out
    }
}
