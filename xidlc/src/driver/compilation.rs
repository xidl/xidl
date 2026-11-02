//! Opt-in whole-compilation generation, reusing the ordinary semantic stage.

use super::{File, generate_session::CodegenSession};
use crate::diagnostic::DiagnosticRunner;
use crate::error::{IdlcError, IdlcResult};
use crate::generate::hir_gen::HirGen;
use crate::jsonrpc::{
    Artifact, COMPILATION_FORMAT, Codegen, CodegenInput, CompilationInput, CompilationUnit,
    UnitImport, VersionInput,
};
use crate::units::{IMPORT_PROPERTY, UNIT_PATH_PROPERTY, Unit, UnitGraph};
use std::collections::BTreeSet;
use std::path::PathBuf;
use xidl_parser::hir::{ParserProperties, Specification};

pub(super) struct CompilationGeneration<'a> {
    lang: &'a str,
    roots: BTreeSet<PathBuf>,
}

impl<'a> CompilationGeneration<'a> {
    pub(super) fn new(lang: &'a str, roots: &[PathBuf]) -> IdlcResult<Self> {
        let roots = roots
            .iter()
            .map(std::fs::canonicalize)
            .collect::<Result<_, _>>()?;
        Ok(Self { lang, roots })
    }

    pub(super) fn generate(
        &self,
        graph: &UnitGraph,
        props: ParserProperties,
    ) -> IdlcResult<Vec<File>> {
        let units = graph
            .units()
            .iter()
            .map(|unit| self.compile_unit(graph, unit, props.clone()))
            .collect::<IdlcResult<Vec<_>>>()?;
        let compilation = CompilationInput {
            format: COMPILATION_FORMAT,
            units,
        };
        let mut session = CodegenSession::spawn(self.lang)?;
        let artifacts = session
            .generate(
                CodegenInput::Compilation(Box::new(compilation)),
                String::new(),
                props,
            )
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        session.finish();
        artifacts
            .into_iter()
            .map(|artifact| match artifact {
                Artifact::File(file) => Ok(File {
                    path: file.path,
                    content: file.content,
                }),
                _ => Err(IdlcError::rpc(
                    "compilation generator must return file artifacts",
                )),
            })
            .collect()
    }

    fn compile_unit(
        &self,
        graph: &UnitGraph,
        unit: &Unit,
        mut props: ParserProperties,
    ) -> IdlcResult<CompilationUnit> {
        let input = unit.generation_input();
        let path = input.to_string_lossy().into_owned();
        if !self.roots.contains(&std::fs::canonicalize(unit.file())?) {
            props.remove(xidl_parser::versioning::API_VERSION_PROPERTY);
            props.remove(xidl_parser::versioning::API_VERSIONS_PROPERTY);
        }
        DiagnosticRunner::new_idl().run(unit.source(), &path)?;
        props.insert("idl".into(), unit.source().into());
        props.insert("target_lang".into(), self.lang.into());
        props.insert(IMPORT_PROPERTY.into(), graph.import_property(unit));
        props.insert(UNIT_PATH_PROPERTY.into(), unit.path().into());
        props.insert(
            xidl_parser::semantic::INTEGER_IMPORTS_PROPERTY.into(),
            serde_json::to_value(graph.integer_imports(unit))?,
        );
        let artifacts = HirGen
            .generate(CodegenInput::RpcHir(Specification(vec![])), path, props)
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        let mut version_map = None;
        let mut inputs = Vec::new();
        for artifact in artifacts {
            let (input, properties) = match artifact {
                Artifact::Hir(data) => (CodegenInput::RpcHir(data.hir), data.props),
                Artifact::RestHir(data) => (CodegenInput::RestHir(data.rest_hir), data.props),
                Artifact::JsonRpcHir(data) => {
                    (CodegenInput::JsonRpcHir(data.jsonrpc_hir), data.props)
                }
                Artifact::File(_) => {
                    return Err(IdlcError::rpc("compilation HIR stage returned a file"));
                }
            };
            if let Some(map) = properties.get("version_map") {
                version_map = Some(serde_json::from_value(map.clone())?);
            }
            inputs.push(VersionInput {
                version: properties
                    .get(xidl_parser::versioning::API_VERSION_PROPERTY)
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                input,
            });
        }
        Ok(CompilationUnit {
            id: unit.prefix().to_owned(),
            is_root: self.roots.contains(&std::fs::canonicalize(unit.file())?),
            imports: serde_json::from_value::<Vec<UnitImport>>(graph.import_property(unit))?,
            version_map,
            inputs,
        })
    }
}
