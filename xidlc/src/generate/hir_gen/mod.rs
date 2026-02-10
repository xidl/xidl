use xidl_parser::hir::{ParserProperties, Specification};

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};

pub struct HirGen;

impl crate::jsonrpc::Codegen for HirGen {
    fn get_engine_version<'a>(
        &'a self,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<String, xidl_jsonrpc::Error>> {
        Box::pin(async move { Ok("*".to_string()) })
    }

    fn get_properties<'a>(
        &'a self,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<ParserProperties, xidl_jsonrpc::Error>> {
        Box::pin(async move {
            Ok(crate::macros::hashmap! {
                "enable_metadata" => true
            })
        })
    }

    fn generate<'a>(
        &'a self,
        _hir: Specification,
        _input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<Vec<Artifact>, xidl_jsonrpc::Error>> {
        Box::pin(async move {
            let source: String = serde_json::from_value(props.get("idl").unwrap().clone()).unwrap();
            let target_lang: String =
                serde_json::from_value(props.get("target_lang").unwrap().clone()).unwrap();

            let typed = xidl_parser::parser::parser_text(&source).unwrap();
            let hir = xidl_parser::hir::Specification::from_typed_ast_with_properties(
                typed,
                props.clone(),
            );

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
        })
    }
}
