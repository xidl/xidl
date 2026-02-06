use super::File;
use crate::driver::generate_session::CodegenSession;
use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::Codegen;
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
        let ts = if cfg!(test) || cfg!(target_os = "emscripten") {
            0
        } else {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };
        let metadata = hashmap! {
            "idl" => source,
            "target_lang" => self.lang.clone(),
            "xidlc_version" => env!("CARGO_PKG_VERSION"),
            "xidlc_timestamp" => ts
        };
        let mut target_props = self.get_properties_for_lang().await?;
        target_props.extend(metadata);
        target_props.extend(props);

        let empty = xidl_parser::hir::Specification(vec![]);
        self.generate_for_lang("hir", empty, path, target_props)
            .await
    }

    async fn generate_for_lang(
        &mut self,
        lang: &str,
        hir: xidl_parser::hir::Specification,
        input: &Path,
        base: HashMap<String, serde_json::Value>,
    ) -> IdlcResult<Vec<File>> {
        tracing::info!("generate for lang: {lang}");
        let input_str = input.to_string_lossy();
        let session = CodegenSession::spawn(lang).await?;
        let mut properties = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        properties.extend(base);

        let artifacts: Vec<crate::jsonrpc::Artifact> = session
            .client
            .generate(hir, input_str.to_string(), properties.clone())
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))?;

        let mut ret: Vec<File> = vec![];
        for file in artifacts {
            match file.tag() {
                crate::jsonrpc::ArtifactKind::Hir => {
                    let data = file.into_hir();
                    let mut props = properties.clone();
                    props.extend(data.props);
                    ret.extend(
                        Box::pin(self.generate_for_lang(&data.lang, data.hir, input, props))
                            .await?,
                    );
                }
                crate::jsonrpc::ArtifactKind::File => {
                    let data = file.into_file();
                    ret.push(File {
                        path: data.path.clone(),
                        content: data.content.clone(),
                    })
                }
            }
        }
        session.finish().await;
        Ok(ret)
    }

    async fn get_properties_for_lang(&mut self) -> IdlcResult<HashMap<String, serde_json::Value>> {
        tracing::info!("get properties for {}", self.lang);
        let session = CodegenSession::spawn(&self.lang).await?;
        let props = session
            .client
            .get_properties()
            .await
            .map_err(|err| IdlcError::rpc(err.to_string()))
            .unwrap();
        session.finish().await;
        Ok(props)
    }
}
