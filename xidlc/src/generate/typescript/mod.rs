mod definition;
mod render;
mod renderer;
mod spec;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::macros::hashmap;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render::{TsMode, TypescriptRender, TypescriptRenderOutput, TypescriptRenderer};

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();

    let mut renderer = TypescriptRenderer::new()?;
    renderer.extend(&props);

    let mode = props
        .get("typescript_mode")
        .map(|v| {
            if let Some(s) = v.as_str() {
                match s {
                    "all" => TsMode::All,
                    "interface_only" => TsMode::InterfaceOnly,
                    "types_only" => TsMode::TypesOnly,
                    _ => TsMode::All,
                }
            } else {
                TsMode::All
            }
        })
        .unwrap_or_default();

    let ts = spec.render(file_name, &renderer, mode)?;

    let mut artifacts = Vec::new();
    artifacts.push(Artifact::new_file(ArtifactFile {
        path: format!("{file_name}.iface.d.ts"),
        content: ts.types,
    }));
    artifacts.push(Artifact::new_file(ArtifactFile {
        path: format!("{file_name}.iface.zod.ts"),
        content: ts.zod,
    }));
    artifacts.push(Artifact::new_file(ArtifactFile {
        path: format!("{file_name}.client.ts"),
        content: ts.client,
    }));

    Ok(artifacts)
}

pub(crate) struct TypescriptCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for TypescriptCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(hashmap! {
            "expand_interface" => false,
            "format_typescript" => true
        })
    }

    async fn generate(
        &self,
        hir: Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate(hir, Path::new(&path), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })
    }
}
