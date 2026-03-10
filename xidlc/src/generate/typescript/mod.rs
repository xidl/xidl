mod definition;
mod render;
mod renderer;
mod spec;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};
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

    let mut artifacts = Vec::new();

    match mode {
        TsMode::InterfaceOnly => {
            let ts = spec.render(file_name, &renderer, TsMode::InterfaceOnly)?;
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
        }
        TsMode::TypesOnly => {
            let ts = spec.render(file_name, &renderer, TsMode::TypesOnly)?;
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.d.ts"),
                content: ts.types,
            }));
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.zod.ts"),
                content: ts.zod,
            }));
        }
        TsMode::All => {
            let ts_iface = spec.render(file_name, &renderer, TsMode::InterfaceOnly)?;
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.iface.d.ts"),
                content: ts_iface.types,
            }));
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.iface.zod.ts"),
                content: ts_iface.zod,
            }));
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.client.ts"),
                content: ts_iface.client,
            }));

            let non_interface = strip_interfaces(spec);
            let ts_types = non_interface.render(file_name, &renderer, TsMode::TypesOnly)?;
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.d.ts"),
                content: ts_types.types,
            }));
            artifacts.push(Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.zod.ts"),
                content: ts_types.zod,
            }));

            if !non_interface.0.is_empty() {
                let props = hashmap! {
                    "enable_render_header" => false,
                    "enable_metadata" => false,
                    "enable_serialize" => false,
                    "enable_deserialize" => false
                };

                artifacts.push(Artifact::new_hir(ArtifactHir {
                    lang: "rs".into(),
                    hir: non_interface,
                    props,
                }));
            }
        }
    }

    Ok(artifacts)
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
