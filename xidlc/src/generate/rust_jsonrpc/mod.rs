mod definition;
mod interface;
mod render;
mod spec;

use crate::error::IdlcResult;
use crate::generate::Artifact;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

pub use render::{JsonRpcRender, JsonRpcRenderOutput, JsonRpcRenderer};

pub fn generate(
    spec: hir::Specification,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let _base = crate::generate::to_snake_case(stem);

    let file_name = input_path.file_stem().unwrap().to_str().unwrap();
    let filename = format!("{file_name}.rs");

    let mut renderer = JsonRpcRenderer::new()?;
    for (k, v) in props {
        renderer
            .env()
            .add_global(k, minijinja::Value::from_serialize(v));
    }
    let output = spec.render(&renderer)?;

    let source = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": output.source,
        }),
    )?;

    let mut artifacts = vec![Artifact::File {
        path: filename,
        content: source,
    }];
    if let Some(non_interface) = strip_interfaces(spec) {
        let mut properties = ParserProperties::default();
        properties.insert("render_header".to_string(), serde_json::Value::Bool(false));
        artifacts.push(Artifact::Hir {
            lang: "rs".to_string(),
            hir: non_interface,
            properties,
        });
    }
    Ok(artifacts)
}

pub(crate) struct RustJsonRpcCodegen;

impl crate::jsonrpc::Codegen for RustJsonRpcCodegen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        let mut props = ParserProperties::default();
        props.insert("expand_interface".into(), false.into());
        Ok(props)
    }

    fn generate(
        &self,
        hir: Specification,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        generate(hir, Path::new(&path), props).map_err(map_codegen_error)
    }
}

fn strip_interfaces(spec: hir::Specification) -> Option<hir::Specification> {
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

    let defs = strip_defs(spec.0);
    if defs.is_empty() {
        None
    } else {
        Some(hir::Specification(defs))
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}
