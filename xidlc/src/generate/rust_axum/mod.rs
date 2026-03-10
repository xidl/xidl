mod definition;
mod interface;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
use crate::macros::hashmap;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();
    let filename = format!("{file_name}.rs");

    let mut renderer = RustAxumRenderer::new()?;
    renderer.extend(&props);
    let output = spec.render(&renderer)?;

    let content = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": output.source,
        }),
    )?;

    let mut artifacts = vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })];

    let non_interface = strip_interfaces(spec);
    if !non_interface.0.is_empty() {
        let props = hashmap! {
            "enable_render_header" => false,
            "enable_metadata" => false,
            "enable_serialize" => false,
            "enable_deserialize" => false
        };

        artifacts.push(Artifact::new_hir(ArtifactHir {
            lang: "rust".into(),
            hir: non_interface,
            props,
        }));
    }

    Ok(artifacts)
}

pub(crate) struct RustAxumCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for RustAxumCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(hashmap! {
            "expand_interface" => false,
            "enable_client" => true,
            "enable_server" => true,
            "enable_render_header" => true,
            "enable_serialize" => true,
            "enable_deserialize" => true,
            "enable_metadata" => true
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

fn strip_interfaces(spec: hir::Specification) -> hir::Specification {
    fn strip_defs(defs: Vec<hir::Definition>) -> Vec<hir::Definition> {
        let mut out = Vec::new();
        for def in defs {
            match def {
                hir::Definition::InterfaceDcl(_) => {}
                hir::Definition::ModuleDcl(mut module) => {
                    module.definition = strip_defs(module.definition);
                    if !module.definition.is_empty() {
                        out.push(hir::Definition::ModuleDcl(module));
                    }
                }
                other => out.push(other),
            }
        }
        out
    }

    hir::Specification(strip_defs(spec.0))
}
