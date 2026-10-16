use xidl_parser::hir::ParserProperties;
use xidl_parser::parser::IncludeResolver;
use xidl_parser::rest_hir::ProjectedHir;
use xidl_parser::versioning::{self, VersionedProjection};

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir, ArtifactJsonRpcHir};
use crate::library::MOUNT_PROPERTY;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct HirGen;

impl crate::jsonrpc::Codegen for HirGen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(crate::macros::hashmap! {
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        _input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let source: String = serde_json::from_value(props.get("idl").unwrap().clone()).unwrap();
        let target_lang: String =
            serde_json::from_value(props.get("target_lang").unwrap().clone()).unwrap();

        let mut resolver = include_resolver(&props);
        let typed =
            xidl_parser::parser::parser_text_with_resolver(&source, Some(&input), &mut *resolver)
                .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))?;
        let projections = xidl_parser::hir::Specification::project_typed_ast_versions(
            typed,
            props.clone(),
            std::path::Path::new(&input),
        )
        .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))?;

        let mut artifacts = Vec::new();
        for projection in projections {
            let props = projection_properties(props.clone(), &projection);
            artifacts.push(projection_artifact(
                projection,
                &target_lang,
                &input,
                props,
            )?);
        }
        Ok(artifacts)
    }
}

/// Picks the include resolver: dependency mounts when the driver supplied any,
/// and plain filesystem resolution otherwise.
fn include_resolver(props: &ParserProperties) -> Box<dyn IncludeResolver> {
    let mounts = props
        .get(MOUNT_PROPERTY)
        .and_then(|value| serde_json::from_value::<BTreeMap<String, PathBuf>>(value.clone()).ok())
        .unwrap_or_default();
    if mounts.is_empty() {
        Box::new(xidl_parser::hir::FsIncludeResolver)
    } else {
        Box::new(xidl_parser::hir::MountIncludeResolver::new(mounts))
    }
}

/// Adds the projected API versions to the properties seen by the target.
///
/// The timeline and the unprojected intervals travel along so the adapter
/// stage can pair versions from one channel; projections strip the lifecycle
/// annotations those facts are read from.
fn projection_properties(
    mut props: ParserProperties,
    projection: &VersionedProjection,
) -> ParserProperties {
    if let Some(version) = &projection.version {
        props.insert(
            versioning::API_VERSION_PROPERTY.to_string(),
            serde_json::Value::String(version.ident.clone()),
        );
    }
    if !projection.versions.is_empty() {
        let modules = projection
            .versions
            .iter()
            .map(|version| serde_json::Value::String(version.ident.clone()))
            .collect();
        props.insert(
            versioning::API_VERSION_MODULES_PROPERTY.to_string(),
            serde_json::Value::Array(modules),
        );
        let timeline = projection
            .versions
            .iter()
            .map(|version| serde_json::to_value(version).unwrap_or(serde_json::Value::Null))
            .collect();
        props.insert(
            versioning::API_VERSION_TIMELINE_PROPERTY.to_string(),
            serde_json::Value::Array(timeline),
        );
    }
    if !projection.intervals.is_empty() {
        let intervals =
            serde_json::to_value(&projection.intervals).unwrap_or(serde_json::Value::Null);
        props.insert(
            versioning::API_VERSION_INTERVALS_PROPERTY.to_string(),
            intervals,
        );
    }
    props
}

fn projection_artifact(
    projection: VersionedProjection,
    target_lang: &str,
    input: &str,
    props: ParserProperties,
) -> Result<Artifact, crate::jsonrpc::RpcError> {
    Ok(match projection.hir {
        ProjectedHir::Rpc(hir) => {
            if target_lang == "hir" {
                Artifact::new_file(ArtifactFile {
                    path: input.to_string(),
                    content: serde_json::to_string(&hir)?,
                })
            } else {
                Artifact::new_hir(ArtifactHir {
                    lang: target_lang.to_string(),
                    hir,
                    props,
                })
            }
        }
        ProjectedHir::Http(rest_hir) => {
            if target_lang == "rest-hir" {
                Artifact::new_file(ArtifactFile {
                    path: input.replace(".idl", ".rest_hir.json"),
                    content: serde_json::to_string_pretty(&rest_hir)?,
                })
            } else {
                Artifact::new_rest_hir(crate::jsonrpc::ArtifactRestHir {
                    lang: target_lang.to_string(),
                    rest_hir,
                    props,
                })
            }
        }
        ProjectedHir::JsonRpc(jsonrpc_hir) => {
            if target_lang == "jsonrpc-hir" {
                Artifact::new_file(ArtifactFile {
                    path: input.replace(".idl", ".jsonrpc_hir.json"),
                    content: serde_json::to_string_pretty(&jsonrpc_hir)?,
                })
            } else {
                Artifact::new_jsonrpc_hir(ArtifactJsonRpcHir {
                    lang: target_lang.to_string(),
                    jsonrpc_hir,
                    props,
                })
            }
        }
    })
}
