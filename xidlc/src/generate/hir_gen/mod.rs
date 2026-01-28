use xidl_parser::hir::{ParserProperties, Specification};

use crate::generate::Artifact;

pub struct HirGen;

impl crate::jsonrpc::Codegen for HirGen {
    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(Default::default())
    }

    fn generate(
        &self,
        _hir: Specification,
        _input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let source: String = serde_json::from_value(props.get("idl").unwrap().clone()).unwrap();
        let target_lang: String =
            serde_json::from_value(props.get("target_lang").unwrap().clone()).unwrap();

        let typed = xidl_parser::parser::parser_text(&source).unwrap();
        let hir =
            xidl_parser::hir::Specification::from_typed_ast_with_properties(typed, props.clone());

        if target_lang == "hir" {
            Ok(vec![Artifact::File {
                path: _input,
                content: serde_json::to_string(&hir)?,
            }])
        } else {
            Ok(vec![Artifact::Hir {
                lang: target_lang,
                hir,
                properties: props,
            }])
        }
    }
}
