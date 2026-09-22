use std::collections::HashMap;

use xidl_parser::hir;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactRestHir};

pub(crate) struct RestHirCodegen;

impl crate::jsonrpc::Codegen for RestHirCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<hir::ParserProperties, crate::jsonrpc::RpcError> {
        Ok(HashMap::from([(
            "hir_kind".to_string(),
            serde_json::Value::String("http".to_string()),
        )]))
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let target_lang: String = serde_json::from_value(
            props
                .get("target_lang")
                .cloned()
                .unwrap_or_else(|| serde_json::Value::String("rest-hir".to_string())),
        )
        .map_err(|err| crate::jsonrpc::RpcError::invalid_params(err.to_string()))?;
        let rest_hir = input_hir.into_rest_hir();

        if target_lang == "rest-hir" {
            let content = serde_json::to_string_pretty(&rest_hir)?;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: path.replace(".idl", ".rest_hir.json"),
                content,
            })])
        } else {
            Ok(vec![Artifact::new_rest_hir(ArtifactRestHir {
                lang: target_lang,
                rest_hir,
                props,
            })])
        }
    }
}
