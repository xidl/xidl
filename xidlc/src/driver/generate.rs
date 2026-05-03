use super::File;
use crate::diagnostic::DiagnosticRunner;
use crate::driver::generate_session::CodegenSession;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Artifact, ArtifactKind, Codegen, CodegenInput};
use crate::macros::hashmap;
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Generator {
    lang: String,
}

impl Generator {
    pub fn new(lang: String) -> Self {
        Self { lang }
    }

    pub async fn generate_from_idl(
        &mut self,
        source: &str,
        path: &Path,
        props: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        tracing::info!("generate for idl");
        DiagnosticRunner::new_idl().run(source, path.to_string_lossy().as_ref())?;

        let ts = if cfg!(test) {
            0
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };
        let mut target_props = self.get_properties_for_lang().await?;
        target_props.extend(self.metadata(source, props, ts));

        let empty = xidl_parser::hir::Specification(vec![]);
        self.generate_for_lang("hir", CodegenInput::new_rpc_hir(empty), path, target_props)
            .await
    }

    fn metadata(
        &self,
        source: &str,
        props: HashMap<String, serde_json::Value>,
        ts: u64,
    ) -> HashMap<String, serde_json::Value> {
        let mut metadata = hashmap! {
            "idl" => source,
            "target_lang" => self.lang.clone(),
            "xidlc_version" => env!("CARGO_PKG_VERSION"),
            "xidlc_timestamp" => ts
        };
        metadata.extend(props);
        metadata
    }

    async fn generate_for_lang(
        &mut self,
        lang: &str,
        input_hir: CodegenInput,
        input: &Path,
        base: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        tracing::info!("generate for lang: {lang}");
        let input_str = input.to_string_lossy();
        let session = CodegenSession::spawn(lang).await?;
        let properties = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let properties = self.merge_properties(properties, base);

        let artifacts: Vec<Artifact> = session
            .client
            .generate(input_hir, input_str.to_string(), properties.clone())
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret = Vec::new();
        for artifact in artifacts {
            ret.extend(Box::pin(self.expand_artifact(artifact, input, &properties)).await?);
        }
        session.finish().await;
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

    async fn expand_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        match artifact.tag() {
            ArtifactKind::Hir => self.expand_hir_artifact(artifact, input, properties).await,
            ArtifactKind::HttpHir => {
                self.expand_http_hir_artifact(artifact, input, properties)
                    .await
            }
            ArtifactKind::JsonRpcHir => {
                self.expand_jsonrpc_hir_artifact(artifact, input, properties)
                    .await
            }
            ArtifactKind::File => Ok(vec![Self::artifact_to_file(artifact)]),
        }
    }

    async fn expand_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        Box::pin(self.generate_for_lang(
            data.lang.as_str(),
            CodegenInput::new_rpc_hir(data.hir),
            input,
            props,
        ))
        .await
    }

    async fn expand_http_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_http_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        Box::pin(self.generate_for_lang(
            &data.lang,
            CodegenInput::new_http_hir(data.http_hir),
            input,
            props,
        ))
        .await
    }

    async fn expand_jsonrpc_hir_artifact(
        &mut self,
        artifact: Artifact,
        input: &Path,
        properties: &HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        let data = artifact.into_jsonrpc_hir();
        let mut props = properties.clone();
        props.extend(data.props);
        Box::pin(self.generate_for_lang(
            &data.lang,
            CodegenInput::new_jsonrpc_hir(data.jsonrpc_hir),
            input,
            props,
        ))
        .await
    }

    fn artifact_to_file(artifact: Artifact) -> File {
        let data = artifact.into_file();
        File {
            path: data.path,
            content: data.content,
        }
    }

    async fn get_properties_for_lang(&mut self) -> IdlcResult<HashMap<String, serde_json::Value>> {
        tracing::info!("get properties for {}", self.lang);
        let session = CodegenSession::spawn(&self.lang).await?;
        let props = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;
        session.finish().await;
        Ok(props)
    }
}
