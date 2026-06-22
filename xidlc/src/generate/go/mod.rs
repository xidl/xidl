mod definition;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::generate::utils::go_package_name;
use crate::jsonrpc::{Artifact, ArtifactFile};
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
    let package = properties
        .get("package")
        .and_then(|v| v.as_str())
        .map(go_package_name)
        .unwrap_or_else(|| go_package_name(stem));
    let filename = format!("{}.go", package);
    let source = spec::render_spec(spec, &package, properties)?;
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
    renderer: GoRenderer,
    pub(crate) body: String,
}

impl GoRenderContext {
    pub(crate) fn new(package_name: String, properties: ParserProperties) -> IdlcResult<Self> {
        let renderer = GoRenderer::new(&properties)?;
        Ok(Self {
            package_name,
            properties,
            state: GoRenderState::default(),
            renderer,
            body: String::new(),
        })
    }

    pub(crate) fn push_template<S: Serialize>(&mut self, name: &str, ctx: &S) -> IdlcResult<()> {
        let rendered = self.render_template(name, ctx)?;
        self.body.push_str(&rendered);
        Ok(())
    }

    pub(crate) fn render_template<S: Serialize>(&self, name: &str, ctx: &S) -> IdlcResult<String> {
        self.renderer.render_template(name, ctx)
    }

    pub(crate) fn finish(self, blocks: Vec<String>) -> IdlcResult<String> {
        let body = blocks.concat();
        let output = GoRenderOutput {
            package_name: self.package_name,
            uses_context: self.state.uses_context,
            body,
        };
        self.renderer.render_spec(&output)
    }
}

#[derive(Serialize)]
pub struct GoRenderOutput {
    package_name: String,
    uses_context: bool,
    body: String,
}
pub(crate) enum ParamDirection {
    In,
    Out,
    InOut,
}
