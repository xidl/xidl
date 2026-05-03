use xidl_jsonrpc::{Error, ErrorCode};
use xidl_parser::hir::ParserProperties;
use xidl_parser::http_hir::ProjectedHir;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir, ArtifactJsonRpcHir};

pub struct HirGen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for HirGen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(crate::macros::hashmap! {
            "enable_metadata" => true
        })
    }

    async fn generate(
        &self,
        _input_hir: crate::jsonrpc::CodegenInput,
        _input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let source: String = serde_json::from_value(props.get("idl").unwrap().clone()).unwrap();
        let target_lang: String =
            serde_json::from_value(props.get("target_lang").unwrap().clone()).unwrap();

        let typed = xidl_parser::parser::parser_text(&source).map_err(|err| Error::Rpc {
            code: ErrorCode::InternalError,
            message: err.to_string(),
            data: None,
        })?;
        let projected =
            xidl_parser::hir::Specification::project_typed_ast_with_properties_and_path(
                typed,
                props.clone(),
                std::path::Path::new(&_input),
            )
            .map_err(|err| Error::Rpc {
                code: ErrorCode::InternalError,
                message: err.to_string(),
                data: None,
            })?;

        match projected {
            ProjectedHir::Rpc(hir) => {
                if target_lang == "hir" {
                    Ok(vec![Artifact::new_file(ArtifactFile {
                        path: _input,
                        content: serde_json::to_string(&hir)?,
                    })])
                } else {
                    Ok(vec![Artifact::new_hir(ArtifactHir {
                        lang: target_lang,
                        hir,
                        props,
                    })])
                }
            }
            ProjectedHir::Http(http_hir) => {
                if target_lang == "http-hir" {
                    Ok(vec![Artifact::new_file(ArtifactFile {
                        path: _input.replace(".idl", ".http_hir.json"),
                        content: serde_json::to_string_pretty(&http_hir)?,
                    })])
                } else {
                    Ok(vec![Artifact::new_http_hir(
                        crate::jsonrpc::ArtifactHttpHir {
                            lang: target_lang,
                            http_hir,
                            props,
                        },
                    )])
                }
            }
            ProjectedHir::JsonRpc(jsonrpc_hir) => {
                if target_lang == "jsonrpc-hir" {
                    Ok(vec![Artifact::new_file(ArtifactFile {
                        path: _input.replace(".idl", ".jsonrpc_hir.json"),
                        content: serde_json::to_string_pretty(&jsonrpc_hir)?,
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
    }
}
