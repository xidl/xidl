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
use xidl_parser::hir::{ParserProperties, Specification};

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

    let typeobject_path = "xidl_typeobject";

    let renderer = RustRenderer::new(typeobject_path.to_string(), properties.clone())?;
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
            "enable_serialize" => true,
            "enable_deserialize" => true,
            "enable_metadata" => true
        })
    }

    async fn generate(
        &self,
        hir: Specification,
        input: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
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
    use super::generate_with_properties;
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
}
