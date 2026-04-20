use std::collections::HashMap;

use xidl_parser::hir;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHttpHir};

use super::project;

pub(crate) struct HttpHirCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for HttpHirCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<hir::ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::new())
    }

    async fn generate(
        &self,
        hir: hir::Specification,
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
        let http_hir = project(&hir).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })?;

        if target_lang == "http-hir" {
            let content = serde_json::to_string_pretty(&http_hir)?;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: path.replace(".idl", ".http_hir.json"),
                content,
            })])
        } else {
            Ok(vec![Artifact::new_http_hir(ArtifactHttpHir {
                lang: target_lang,
                hir,
                http_hir,
                props,
            })])
        }
    }
}
