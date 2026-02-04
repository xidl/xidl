use xidl_parser::hir::ParserProperties;

use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir};

pub struct TypedAstGen;

impl crate::jsonrpc::Codegen for TypedAstGen {
    fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok(crate::generate::compatible_xidlc_version())
    }

    fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(Default::default())
    }

    fn generate(
        &self,
        hir: xidl_parser::hir::Specification,
        input: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let source: String = serde_json::from_value(props.get("idl").unwrap().clone()).unwrap();
        let target_lang: String =
            serde_json::from_value(props.get("target_lang").unwrap().clone()).unwrap();

        let typed = xidl_parser::parser::parser_text(&source).unwrap();

        if target_lang == "typed_ast" || target_lang == "typed-ast" {
            Ok(vec![Artifact::new_file(ArtifactFile {
                path: input,
                content: serde_json::to_string(&typed)?,
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
