use super::File;
use super::session_pool::SessionPool;
use crate::diagnostic::DiagnosticRunner;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{
    Artifact, ArtifactFile, ArtifactHir, ArtifactJsonRpcHir, ArtifactRestHir, CodegenInput,
};
use crate::macros::{hashmap, log_info};
use crate::units::UnitGraph;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xidl_parser::hir::ParserProperties;

pub struct Generator {
    lang: String,
    sessions: SessionPool,
}

impl Generator {
    pub fn new(lang: String) -> Self {
        Self {
            lang,
            sessions: SessionPool::new(),
        }
    }

    #[cfg(test)]
    pub(super) fn with_session_factory<F>(lang: String, factory: F) -> Self
    where
        F: FnMut(&str) -> IdlcResult<super::generate_session::CodegenSession>
            + Send
            + Sync
            + 'static,
    {
        Self {
            lang,
            sessions: SessionPool::with_factory(factory),
        }
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
        let session_properties = self.sessions.properties(lang)?;
        let properties = self.merge_properties(session_properties, base);

        let mut lease = self.sessions.checkout(lang)?;
        let generated =
            lease
                .session_mut()?
                .generate(input_hir, input_str.to_string(), properties.clone());
        self.sessions.reclaim(lease);
        let artifacts: Vec<Artifact> = generated.map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret = Vec::new();
        for artifact in artifacts {
            ret.extend(self.expand_artifact(artifact, input, &properties)?);
        }
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
        match artifact {
            Artifact::Hir(data) => self.expand_hir_artifact(data, input, properties),
            Artifact::RestHir(data) => self.expand_rest_hir_artifact(data, input, properties),
            Artifact::JsonRpcHir(data) => self.expand_jsonrpc_hir_artifact(data, input, properties),
            Artifact::File(data) => Ok(vec![Self::artifact_to_file(data)]),
        }
    }

    fn expand_hir_artifact(
        &mut self,
        data: ArtifactHir,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
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
        data: ArtifactRestHir,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
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
        data: ArtifactJsonRpcHir,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let mut props = properties.clone();
        props.extend(data.props);
        self.generate_for_lang(
            &data.lang,
            CodegenInput::new_jsonrpc_hir(data.jsonrpc_hir),
            input,
            props,
        )
    }

    fn artifact_to_file(data: ArtifactFile) -> File {
        File {
            path: data.path,
            content: data.content,
        }
    }

    pub(super) fn get_properties_for_lang(
        &mut self,
    ) -> IdlcResult<HashMap<String, serde_json::Value>> {
        log_info!("get properties for {}", self.lang);
        self.sessions.properties(&self.lang)
    }

    pub(super) fn generate_compilation(
        &mut self,
        graph: &UnitGraph,
        props: ParserProperties,
        roots: &[PathBuf],
    ) -> IdlcResult<Vec<File>> {
        super::compilation::CompilationGeneration::new(&self.lang, roots)?.generate(
            graph,
            props,
            &mut self.sessions,
        )
    }
}
