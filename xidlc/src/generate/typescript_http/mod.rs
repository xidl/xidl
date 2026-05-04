mod interface;
mod model;
mod server;
mod spec;

use crate::error::{IdlcError, IdlcResult};
use crate::generate::typescript::TypescriptRenderer;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
use crate::macros::hashmap;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub fn generate(
    http_hir: xidl_parser::http_hir::HttpHirDocument,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let enable_client = props
        .get("enable_client")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let enable_server = props
        .get("enable_server")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    if !enable_client && !enable_server {
        return Err(IdlcError::rpc(
            "typescript-http requires enable_client or enable_server",
        ));
    }

    let spec = http_hir.spec.clone();
    let file_name = input_path.file_stem().unwrap().to_str().unwrap();
    let mut renderer = TypescriptRenderer::new()?;
    renderer.extend(&props);

    let output = spec::render_spec(&spec, file_name, &renderer, &http_hir)?;
    let mut artifacts = vec![
        Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.iface.d.ts"),
            content: output.types,
        }),
        Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.iface.zod.ts"),
            content: output.zod,
        }),
    ];
    if enable_client {
        artifacts.push(Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.client.ts"),
            content: output.client,
        }));
    }
    if enable_server {
        artifacts.push(Artifact::new_file(ArtifactFile {
            path: format!("{file_name}.server.ts"),
            content: output.server,
        }));
    }

    let non_interface = strip_interfaces(spec);
    if !non_interface.0.is_empty() {
        artifacts.push(Artifact::new_hir(ArtifactHir {
            lang: "typescript".into(),
            hir: non_interface,
            props: hashmap! {
                "typescript_mode" => "types_only"
            },
        }));
    }

    Ok(artifacts)
}

pub(crate) struct TypescriptHttpCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for TypescriptHttpCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(hashmap! {
            "expand_interface" => false,
            "hir_kind" => "http",
            "enable_client" => true,
            "enable_server" => true
        })
    }

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let http_hir = input_hir.into_http_hir();
        generate(http_hir, Path::new(&path), props).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })
    }
}

fn strip_interfaces(spec: hir::Specification) -> hir::Specification {
    hir::Specification(strip_defs(spec.0))
}

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
