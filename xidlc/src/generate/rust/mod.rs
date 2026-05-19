mod bitmask_dcl;
mod bitset_dcl;
mod const_dcl;
mod constr_type;
mod definition;
mod enum_dcl;
mod exception_dcl;
mod interface_dcl;
mod render;
mod spec;
mod struct_dcl;
mod type_dcl;
mod union_def;
pub(crate) mod util;

pub use render::{RustRender, RustRenderOutput, RustRenderer};

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub fn generate_with_properties(
    spec: &hir::Specification,
    input_path: &Path,
    properties: &ParserProperties,
) -> IdlcResult<Vec<Artifact>> {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("output");
    let filename = format!("{stem}.rs");

    let renderer = RustRenderer::new(properties.clone())?;
    let output = spec.render(&renderer)?;
    let source = renderer.render_spec(&output.source)?;

    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content: source,
    })])
}

pub(crate) struct RustCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for RustCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(crate::macros::hashmap! {
            "enable_render_header" => true,
            "enable_metadata" => true
        })
    }

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let hir = input_hir.into_rpc_hir();
        generate_with_properties(&hir, Path::new(&input), &props).map_err(map_codegen_error)
    }
}

fn map_codegen_error(err: crate::error::IdlcError) -> xidl_jsonrpc::Error {
    xidl_jsonrpc::Error::Rpc {
        code: xidl_jsonrpc::ErrorCode::ServerError,
        message: err.to_string(),
        data: None,
    }
}

#[cfg(test)]
mod tests {
    use super::RustRenderer;
    use super::generate_with_properties;
    use crate::generate::rust::util::{
        serde_rename_all_from_annotations, serde_rename_from_annotations,
    };
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;
    use xidl_parser::hir;
    use xidl_parser::hir::RenameRule;

    #[test]
    fn preserves_numeric_file_stem_in_output_path() {
        let artifacts = generate_with_properties(
            &hir::Specification(vec![]),
            Path::new("e2e_test.idl"),
            &HashMap::new(),
        )
        .expect("rust generation should succeed");

        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].as_file().path, "e2e_test.rs");
    }

    #[test]
    fn custom_annotation_helpers_read_standard_params() {
        let annotations = vec![
            hir::Annotation::Rename {
                name: "wireName".into(),
            },
            hir::Annotation::RenameAll {
                rule: RenameRule::CamelCase,
            },
        ];

        assert_eq!(
            serde_rename_from_annotations(&annotations),
            Some("wireName".to_string())
        );
        assert_eq!(
            serde_rename_all_from_annotations(&annotations),
            Some(RenameRule::CamelCase)
        );
    }

    #[test]
    fn struct_template_uses_serde_rename_semantics() {
        let renderer = RustRenderer::new(HashMap::new()).expect("renderer");
        let rendered = renderer
            .render_template(
                "struct.rs.j2",
                &json!({
                    "doc": [],
                    "rust_attrs": [],
                    "derive": ["::serde::Serialize", "::serde::Deserialize"],
                    "enable_serde_attrs": true,
                    "serde_rename_all": "camelCase",
                    "ident": "RenameStruct",
                    "parent": null,
                    "members": [{
                        "doc": [],
                        "rust_attrs": [],
                        "serde_rename": "wireName",
                        "serde_skip": false,
                        "name": "plain_name",
                        "ty": "String"
                    }]
                }),
            )
            .expect("render struct template");

        assert!(rendered.contains("#[serde(rename_all = \"camelCase\")]"));
        assert!(rendered.contains("#[serde(rename = \"wireName\")]"));
    }

    #[test]
    fn enum_template_uses_serde_rename_semantics() {
        let renderer = RustRenderer::new(HashMap::new()).expect("renderer");
        let rendered = renderer
            .render_template(
                "enum.rs.j2",
                &json!({
                    "doc": [],
                    "rust_attrs": [],
                    "ident": "RenameEnum",
                    "has_serde_serialize": true,
                    "has_serde_deserialize": true,
                    "serde_rename_all": "SCREAMING-KEBAB-CASE",
                    "members": [{
                        "doc": [],
                        "rust_attrs": [],
                        "serde_rename": "wire-value",
                        "is_default": true,
                        "name": "LocalValue"
                    }]
                }),
            )
            .expect("render enum template");

        assert!(rendered.contains("#[serde(rename_all = \"SCREAMING-KEBAB-CASE\")]"));
        assert!(rendered.contains("#[serde(rename = \"wire-value\")]"));
    }
}
