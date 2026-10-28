mod bitmask_dcl;
mod bitset_dcl;
mod const_dcl;
mod constr_type;
mod definition;
mod enum_dcl;
mod exception_dcl;
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
    let imports = renderer.referenced_imports(&output.source);
    let source = renderer.render_spec(&output.source, &imports)?;

    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content: source,
    })])
}

pub(crate) struct RustCodegen;

impl crate::jsonrpc::Codegen for RustCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(crate::macros::hashmap! {
            "enable_render_header" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let hir = input_hir.into_rpc_hir();
        generate_with_properties(&hir, Path::new(&input), &props)
            .map_err(crate::jsonrpc::RpcError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::RustRenderer;
    use super::generate_with_properties;
    use serde_json::json;
    use std::collections::HashMap;
    use std::path::Path;
    use xidl_parser::hir;

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
    fn rejects_interface_declarations() {
        let spec = hir::Specification(vec![hir::Definition::InterfaceDcl(hir::InterfaceDcl {
            annotations: Vec::new(),
            decl: hir::InterfaceDclInner::InterfaceForwardDcl(hir::InterfaceForwardDcl {
                ident: "Greeter".to_string(),
            }),
        })]);

        let error = generate_with_properties(&spec, Path::new("api.idl"), &HashMap::new())
            .expect_err("the rust target no longer supports interfaces");
        assert!(
            error
                .to_string()
                .contains("does not support the `interface Greeter`"),
            "unexpected error: {error}"
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

        assert!(rendered.contains("#[serde(rename = \"wireName\")]"));
        assert!(
            !rendered.contains("rename_all"),
            "the struct template never re-derives names at runtime"
        );
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

        assert!(rendered.contains("#[serde(rename = \"wire-value\")]"));
        assert!(
            !rendered.contains("rename_all"),
            "the enum template never re-derives names at runtime"
        );
    }
}
