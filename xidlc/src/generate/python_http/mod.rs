mod render;
mod spec;

use crate::error::IdlcResult;
use crate::generate::http_hir::HttpHirDocument;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
use crate::macros::hashmap;
use convert_case::{Case, Casing};
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render::PythonHttpRenderer;

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let http_hir = HttpHirDocument::from_props(&props)?;
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let filename = format!("{}_http.py", stem.replace('-', "_"));
    let module_name = stem.to_case(Case::Snake).replace('-', "_");
    let content = spec::render_spec(&spec, &module_name, &http_hir)?;

    let mut artifacts = vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })];

    let non_interface = strip_interfaces(spec);
    if !non_interface.0.is_empty() {
        let props = hashmap! {
            "enable_metadata" => false
        };
        artifacts.push(Artifact::new_hir(ArtifactHir {
            lang: "python".into(),
            hir: non_interface,
            props,
        }));
    }

    Ok(artifacts)
}

pub(crate) struct PythonHttpCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for PythonHttpCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(hashmap! {
            "expand_interface" => false,
            "enable_client" => true,
            "enable_server" => true,
            "enable_metadata" => true
        })
    }

    async fn generate(
        &self,
        hir: Specification,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate(hir, Path::new(&input), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
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
