pub(crate) mod definition;
mod render;
mod renderer;
mod spec;

use crate::error::IdlcResult;
use crate::jsonrpc::Artifact;
use crate::jsonrpc::ArtifactFile;
use crate::macros::hashmap;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub use render::{TsMode, TypescriptRender, TypescriptRenderOutput, TypescriptRenderer};

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();

    let mut renderer = TypescriptRenderer::new()?;
    renderer.extend(&props);
    let ts = spec.render(file_name, &renderer, TsMode::TypesOnly)?;
    Ok(vec![
        Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.d.ts"),
            content: ts.types,
        }),
        Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.zod.ts"),
            content: ts.zod,
        }),
    ])
}

pub(crate) struct TypescriptCodegen;

impl crate::jsonrpc::Codegen for TypescriptCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(hashmap! {
            "expand_interface" => false,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let hir = input_hir.into_rpc_hir();
        generate(hir, Path::new(&path), props)
            .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))
    }
}
