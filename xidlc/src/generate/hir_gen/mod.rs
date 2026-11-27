use xidl_parser::hir::ParserProperties;
use xidl_parser::parser::{IncludeResolver, parser_text_with_resolver};
use xidl_parser::pipeline::{CompileOptions, Pipeline, ProjectedHir};
use xidl_parser::typed_ast;
use xidl_parser::versioning::{self, VersionedProjection};

use crate::diagnostic::DiagnosticRunner;
use crate::driver::lang::Plugin;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir, ArtifactJsonRpcHir, RpcError};
use crate::library::MOUNT_PROPERTY;
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Built-in HIR stage compiling IDL source into projected IR artifacts.
pub struct HirGen;

impl crate::jsonrpc::Codegen for HirGen {
    fn get_engine_version(&self) -> Result<String, RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, RpcError> {
        Ok(crate::macros::hashmap! {
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        _input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError> {
        let source = Self::required_string_prop(&props, "idl")?;
        let typed = self
            .parse_source(&source, &input, &props)
            .map_err(|err| RpcError::new(err.to_string()))?;
        self.generate_typed_ast(typed, &source, input, props)
            .map_err(|err| RpcError::new(err.to_string()))
    }
}

impl HirGen {
    /// Compiles an already-parsed typed AST into target artifacts.
    pub(crate) fn generate_typed_ast(
        &self,
        typed: typed_ast::Specification,
        source: &str,
        input: String,
        mut props: ParserProperties,
    ) -> IdlcResult<Vec<Artifact>> {
        let target_lang = Self::required_string_prop(&props, "target_lang")
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        let mut resolver = include_resolver(&props);
        let diagnostics =
            |error| DiagnosticRunner::new_idl().from_parse_error(source, &input, error);

        if Self::is_version_map_target(&target_lang) {
            let options = CompileOptions::new(&input, props).with_version_map(true);
            let version_map = Pipeline::build_version_map_typed_ast(typed, options, &mut *resolver)
                .map_err(diagnostics)?;
            let Some(version_map) = version_map.as_ref() else {
                return Err(IdlcError::rpc("xidlc version-map requires a @version enum"));
            };
            let file_name = std::path::Path::new(&input)
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("api");
            let content = xidl_parser::versioning::map::to_json(version_map)
                .map_err(|err| IdlcError::rpc(err.to_string()))?;
            return Ok(vec![Artifact::new_file(ArtifactFile {
                path: format!("{file_name}.version-map.json"),
                content,
            })]);
        }

        let include_http_operations = matches!(
            props.get("hir_kind").and_then(serde_json::Value::as_str),
            Some(value) if value.eq_ignore_ascii_case("http")
        );
        let options =
            CompileOptions::new(&input, props.clone()).with_version_map(include_http_operations);
        let compiled =
            Pipeline::compile_typed_ast(typed, options, &mut *resolver).map_err(diagnostics)?;

        if let Some(version_map) = compiled.version_map {
            props.insert(
                "version_map".to_string(),
                serde_json::to_value(version_map)?,
            );
        }
        let mut artifacts = Vec::new();
        for projection in compiled.projections {
            let props = projection_properties(props.clone(), &projection);
            artifacts.push(
                projection_artifact(projection, &target_lang, &input, props)
                    .map_err(|err| IdlcError::rpc(err.to_string()))?,
            );
        }
        Ok(artifacts)
    }

    /// Parses IDL source with the include resolver selected by `props`.
    pub(crate) fn parse_source(
        &self,
        source: &str,
        input: &str,
        props: &ParserProperties,
    ) -> xidl_parser::error::ParserResult<typed_ast::Specification> {
        let mut resolver = include_resolver(props);
        parser_text_with_resolver(source, Some(input), &mut *resolver)
    }

    fn required_string_prop(props: &ParserProperties, key: &str) -> Result<String, RpcError> {
        let value = props.get(key).ok_or_else(|| {
            RpcError::invalid_params(format!("hir: missing required property '{key}'"))
        })?;
        serde_json::from_value(value.clone()).map_err(|err| {
            RpcError::invalid_params(format!("hir: invalid property '{key}': {err}"))
        })
    }

    fn is_version_map_target(target_lang: &str) -> bool {
        Plugin::from(target_lang) == Plugin::VersionMap
    }
}

/// Picks the include resolver: dependency mounts when the driver supplied any,
/// and plain filesystem resolution otherwise.
fn include_resolver(props: &ParserProperties) -> Box<dyn IncludeResolver> {
    let mounts = props
        .get(MOUNT_PROPERTY)
        .and_then(|value| serde_json::from_value::<BTreeMap<String, PathBuf>>(value.clone()).ok())
        .unwrap_or_default();
    crate::library::include_resolver(&mounts)
}

/// Adds the selected API version to the properties seen by the target.
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
    }
    props
}

fn projection_artifact(
    projection: VersionedProjection,
    target_lang: &str,
    input: &str,
    props: ParserProperties,
) -> Result<Artifact, RpcError> {
    let plugin = Plugin::from(target_lang);
    Ok(match projection.hir {
        ProjectedHir::Rpc(hir) => {
            if plugin == Plugin::Hir {
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
            if plugin == Plugin::RestHir {
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
            if plugin == Plugin::JsonRpcHir {
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
