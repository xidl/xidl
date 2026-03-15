use xidl_jsonrpc::{Error, ErrorCode};
use xidl_parser::hir::{ParserProperties, Specification};

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};

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
        _hir: Specification,
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
        let hir = xidl_parser::hir::Specification::from_typed_ast_with_properties_and_path(
            typed,
            props.clone(),
            std::path::Path::new(&_input),
        )
        .map_err(|err| Error::Rpc {
            code: ErrorCode::InternalError,
            message: err.to_string(),
            data: None,
        })?;

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
}
