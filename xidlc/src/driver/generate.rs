use super::File;
use crate::diagnostic::DiagnosticRunner;
use crate::driver::generate_session::CodegenSession;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Artifact, ArtifactKind, CodegenInput};
use crate::macros::{hashmap, log_info};
use std::collections::HashMap;
use std::path::Path;

pub struct Generator {
    lang: String,
}

impl Generator {
    pub fn new(lang: String) -> Self {
        Self { lang }
    }

    pub fn generate_from_idl(
        &mut self,
        source: &str,
        path: &Path,
        props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        log_info!("generate for idl");
        DiagnosticRunner::new_idl().run(source, path.to_string_lossy().as_ref())?;

        let mut target_props = self.get_properties_for_lang()?;
        target_props.extend(self.metadata(source, props));

        let empty = xidl_parser::hir::Specification(vec![]);
        self.generate_for_lang("hir", CodegenInput::new_rpc_hir(empty), path, target_props)
    }

    fn metadata(
        &self,
        source: &str,
        props: HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut metadata = hashmap! {
            "idl" => source,
            "target_lang" => self.lang.clone(),
            "xidlc_version" => option_env!("XIDLC_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
            "xidlc_shorthash" => option_env!("XIDLC_GIT_HASH").unwrap_or("unknown")
        };
        metadata.extend(props);
        metadata
    }

    fn generate_for_lang(
        &mut self,
        lang: &str,
        input_hir: CodegenInput,
        input: &Path,
        base: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        log_info!("generate for lang: {lang}");
        let input_str = input.to_string_lossy();
        let mut session = CodegenSession::spawn(lang)?;
        let properties = session
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let properties = self.merge_properties(properties, base);

        let artifacts: Vec<Artifact> = session
            .generate(input_hir, input_str.to_string(), properties.clone())
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret = Vec::new();
        for artifact in artifacts {
            ret.extend(self.expand_artifact(artifact, input, &properties)?);
        }
        session.finish();
        Ok(ret)
    }

    fn merge_properties(
        &self,
        mut properties: HashMap<String, serde_json::Value>,
        extra: HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        properties.extend(extra);
        properties
    }

    fn expand_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        match artifact.tag() {
            ArtifactKind::Hir => self.expand_hir_artifact(artifact, input, properties),
            ArtifactKind::RestHir => self.expand_rest_hir_artifact(artifact, input, properties),
            ArtifactKind::JsonRpcHir => {
                self.expand_jsonrpc_hir_artifact(artifact, input, properties)
            }
            ArtifactKind::File => Ok(vec![Self::artifact_to_file(artifact)]),
        }
    }

    fn expand_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        self.generate_for_lang(
            data.lang.as_str(),
            CodegenInput::new_rpc_hir(data.hir),
            input,
            props,
        )
    }

    fn expand_rest_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_rest_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        self.generate_for_lang(
            &data.lang,
            CodegenInput::new_rest_hir(data.rest_hir),
            input,
            props,
        )
    }

    fn expand_jsonrpc_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_jsonrpc_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        self.generate_for_lang(
            &data.lang,
            CodegenInput::new_jsonrpc_hir(data.jsonrpc_hir),
            input,
            props,
        )
    }

    fn artifact_to_file(artifact: Artifact) -> File {
        let data = artifact.into_file();
        File {
            path: data.path,
            content: data.content,
        }
    }

    fn get_properties_for_lang(&mut self) -> IdlcResult<HashMap<String, serde_json::Value>> {
        log_info!("get properties for {}", self.lang);
        let mut session = CodegenSession::spawn(&self.lang)?;
        let props = session
            .get_properties()
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        session.finish();
        Ok(props)
    }
}
