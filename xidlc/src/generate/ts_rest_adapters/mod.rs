//! Independent TypeScript REST adapter target.

mod context;
mod endpoints;
mod error;
mod names;
mod render;
mod renderer;
mod symbols;
mod type_endpoints;
mod wire;
mod wire_context;

use crate::generate::adapter::AdapterPlanner;
use crate::jsonrpc::{Artifact, ArtifactFile, Codegen, CodegenInput, RpcError};
use crate::macros::hashmap;
use std::path::Path;
use xidl_parser::hir::ParserProperties;

pub(crate) struct TypescriptRestAdaptersCodegen;

impl Codegen for TypescriptRestAdaptersCodegen {
    fn get_engine_version(&self) -> Result<String, RpcError> {
        Ok("*".into())
    }

    fn get_properties(&self) -> Result<ParserProperties, RpcError> {
        Ok(
            hashmap! { "expand_interface" => false, "hir_kind" => "http", "enable_metadata" => true },
        )
    }

    fn generate(
        &self,
        input: CodegenInput,
        path: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError> {
        let http = input.into_rest_hir();
        let plan = AdapterPlanner::from_props(&http, &props)
            .and_then(|planner| planner.plan())
            .map_err(|error| RpcError::new(error.to_string()))?;
        let stem = Path::new(&path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                RpcError::new(format!(
                    "typescript-rest-adapters: invalid contract path {path}"
                ))
            })?;
        let mut renderer = renderer::TsRestAdaptersRenderer::new()
            .map_err(|error| RpcError::new(error.to_string()))?;
        renderer.extend(&props);
        let context = render::AdapterRenderer::new(&http, &plan)
            .context(stem)
            .map_err(|error| RpcError::new(error.to_string()))?;
        let content = renderer
            .render_template("adapters.ts.j2", &context)
            .map_err(|error| RpcError::new(error.to_string()))?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: format!("{stem}.adapters.ts"),
            content,
        })])
    }
}
