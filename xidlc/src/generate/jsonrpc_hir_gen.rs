use std::collections::HashMap;

use xidl_parser::hir;

use crate::driver::lang::Plugin;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactJsonRpcHir};

pub(crate) struct JsonRpcHirCodegen;

impl crate::jsonrpc::Codegen for JsonRpcHirCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<hir::ParserProperties, crate::jsonrpc::RpcError> {
        Ok(HashMap::from([(
            "hir_kind".to_string(),
            serde_json::Value::String("jsonrpc".to_string()),
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
                .unwrap_or_else(|| serde_json::Value::String("jsonrpc-hir".to_string())),
        )
        .map_err(|err| crate::jsonrpc::RpcError::invalid_params(err.to_string()))?;
        let jsonrpc_hir = input_hir.try_into_jsonrpc_hir()?;

        if Plugin::from(target_lang.as_str()) == Plugin::JsonRpcHir {
            let content = serde_json::to_string_pretty(&jsonrpc_hir)?;
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: path.replace(".idl", ".jsonrpc_hir.json"),
                content,
            })])
        } else {
            Ok(vec![Artifact::new_jsonrpc_hir(ArtifactJsonRpcHir {
                lang: target_lang,
                jsonrpc_hir,
                props,
            })])
        }
    }
}
