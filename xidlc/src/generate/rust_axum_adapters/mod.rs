//! Independent Rust Axum adapter artifact, consuming public HIR and map facts.

mod construction;
mod context;
mod errors;
mod layout;
mod names;
mod render;
mod renderer;

use crate::generate::adapter::AdapterPlanner;
use crate::jsonrpc::{Artifact, ArtifactFile, Codegen, CodegenInput, RpcError};
use crate::macros::hashmap;
use std::path::Path;
use xidl_parser::hir::ParserProperties;

pub(crate) struct RustAxumAdaptersCodegen;

impl Codegen for RustAxumAdaptersCodegen {
    fn get_engine_version(&self) -> Result<String, RpcError> {
        Ok("*".into())
    }

    fn get_properties(&self) -> Result<ParserProperties, RpcError> {
        Ok(hashmap! { "hir_kind" => "http", "enable_metadata" => true })
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
            .ok_or_else(|| RpcError::new(format!("rust-axum-adapters: invalid contract {path}")))?;
        let renderer = renderer::RustAxumAdaptersRenderer::new()
            .map_err(|error| RpcError::new(error.to_string()))?;
        let models = crate::generate::rust::RustRenderer::new(Default::default())
            .map_err(|error| RpcError::new(error.to_string()))?
            .with_runtime("::xidl_rust_axum");
        let context = render::AdapterRenderer::new(&http, &plan, &models)
            .context()
            .map_err(|error| RpcError::new(error.to_string()))?;
        let content = renderer
            .render_template("adapters.rs.j2", &context)
            .map_err(|error| RpcError::new(error.to_string()))?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: format!("{stem}.adapters.rs"),
            content,
        })])
    }
}
