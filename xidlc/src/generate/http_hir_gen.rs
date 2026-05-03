use std::collections::HashMap;

use xidl_parser::hir;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHttpHir};

pub(crate) struct HttpHirCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for HttpHirCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<hir::ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::from([(
            "hir_kind".to_string(),
            serde_json::Value::String("http".to_string()),
        )]))
    }

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let target_lang: String = serde_json::from_value(
            props
                .get("target_lang")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::String("http-hir".to_string())),
        )
        .map_err(|err| xidl_jsonrpc::Error::invalid_params(err.to_string()))?;
        let http_hir = input_hir.into_http_hir();

        if target_lang == "http-hir" {
            let content = serde_json::to_string_pretty(&http_hir)?;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: path.replace(".idl", ".http_hir.json"),
                content,
            })])
        } else {
            Ok(vec![Artifact::new_http_hir(ArtifactHttpHir {
                lang: target_lang,
                http_hir,
                props,
            })])
        }
    }
}
