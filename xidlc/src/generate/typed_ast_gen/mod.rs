use std::collections::BTreeMap;
use std::path::PathBuf;
use xidl_parser::hir::ParserProperties;

use crate::driver::lang::Plugin;
use crate::jsonrpc::{Artifact, ArtifactFile, ArtifactHir, RpcError};
use crate::library::MOUNT_PROPERTY;

pub struct TypedAstGen;

impl crate::jsonrpc::Codegen for TypedAstGen {
    fn get_engine_version(&self) -> Result<String, RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, RpcError> {
        Ok(crate::macros::hashmap! {
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, RpcError> {
        crate::generate::versioning::reject_version_selection("typed-ast", &props)?;
        let hir = input_hir.try_into_rpc_hir()?;
        let source = required_string_prop(&props, "idl", "typed-ast")?;
        let target_lang = required_string_prop(&props, "target_lang", "typed-ast")?;

        let mounts = props
            .get(MOUNT_PROPERTY)
            .and_then(|value| {
                serde_json::from_value::<BTreeMap<String, PathBuf>>(value.clone()).ok()
            })
            .unwrap_or_default();
        let mut resolver = crate::library::include_resolver(&mounts);
        let typed =
            xidl_parser::parser::parser_text_with_resolver(&source, Some(&input), &mut *resolver)
                .map_err(|err| RpcError::new(format!("typed-ast parser: {err}")))?;

        if Plugin::from(target_lang.as_str()) == Plugin::TypedAst {
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

fn required_string_prop(
    props: &ParserProperties,
    key: &str,
    stage: &str,
) -> Result<String, RpcError> {
    let value = props.get(key).ok_or_else(|| {
        RpcError::invalid_params(format!("{stage}: missing required property '{key}'"))
    })?;
    serde_json::from_value(value.clone()).map_err(|err| {
        RpcError::invalid_params(format!("{stage}: invalid property '{key}': {err}"))
    })
}
