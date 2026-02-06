mod render_typescript;
mod renderer;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::macros::hashmap;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render_typescript::{TsMode, render_typescript};
pub use renderer::TypescriptRenderer;

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();

    let mut renderer = TypescriptRenderer::new()?;
    renderer.extend(&props);

    let ts = render_typescript(&spec, file_name, &renderer, TsMode::TypesOnly)?;

    let mut artifacts = Vec::new();
    artifacts.push(Artifact::new_file(ArtifactFile {
        path: format!("{file_name}.d.ts"),
        content: ts.types,
    }));
    artifacts.push(Artifact::new_file(ArtifactFile {
        path: format!("{file_name}.zod.ts"),
        content: ts.zod,
    }));

    Ok(artifacts)
}

pub(crate) struct TypescriptCodegen;

impl crate::jsonrpc::Codegen for TypescriptCodegen {
    fn get_engine_version<'a>(
        &'a self,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<String, xidl_jsonrpc::Error>> {
        Box::pin(async move { Ok(crate::generate::compatible_xidlc_version()) })
    }

    fn get_properties<'a>(
        &'a self,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<ParserProperties, xidl_jsonrpc::Error>> {
        Box::pin(async move {
            Ok(hashmap! {
                "expand_interface" => false
            })
        })
    }

    fn generate<'a>(
        &'a self,
        hir: Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<Vec<Artifact>, xidl_jsonrpc::Error>> {
        Box::pin(async move {
            generate(hir, Path::new(&path), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
                code: xidl_jsonrpc::ErrorCode::ServerError,
                message: err.to_string(),
                data: None,
            })
        })
    }
}
