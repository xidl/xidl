#[cfg(feature = "gen-rust-axum-adapters")]
pub(crate) mod adapters;
mod interface;
mod merged;
mod render;
mod scope;
mod transport;

use crate::error::IdlcResult;
use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::macros::hashmap;
use serde_json::json;
use std::collections::HashMap;
use std::path::Path;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;

pub use render::{RustAxumRender, RustAxumRenderOutput, RustAxumRenderer};

pub fn generate(
    rest_hir: xidl_parser::rest_hir::RestHirDocument,
    input_path: &Path,
    props: HashMap<String, serde_json::Value>,
) -> IdlcResult<Vec<Artifact>> {
    let spec = rest_hir.spec.clone();
    let file_name = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            crate::error::IdlcError::rpc(format!(
                "rust-axum generator requires a valid input file stem for '{}'",
                input_path.display()
            ))
        })?;
    let filename = format!("{file_name}.rs");

    let mut renderer = RustAxumRenderer::new()?;
    renderer.extend(&props, rest_hir.clone());

    let refs = spec.0.iter().collect::<Vec<_>>();
    let registry = transport::build_type_registry(&refs, &[]);
    let rust_renderer = crate::generate::rust::RustRenderer::new(std::collections::HashMap::new())?;
    let builder = merged::MergedAxumSpecBuilder::new(&renderer, rust_renderer, registry);
    let definitions = builder.render(&spec)?;
    let imports = crate::generate::utils::referenced_imports(&props, &definitions, "::");

    let content = renderer.render_template(
        "spec.rs.j2",
        &json!({
            "definitions": definitions,
            "imports": imports,
        }),
    )?;

    Ok(vec![Artifact::new_file(ArtifactFile {
        path: filename,
        content,
    })])
}

pub(crate) fn get_def_idents(def: &hir::Definition) -> Vec<String> {
    match def {
        hir::Definition::ConstrTypeDcl(c) => match c {
            hir::ConstrTypeDcl::StructDcl(s) => vec![s.ident.clone()],
            hir::ConstrTypeDcl::EnumDcl(e) => vec![e.ident.clone()],
            hir::ConstrTypeDcl::UnionDef(u) => vec![u.ident.clone()],
            hir::ConstrTypeDcl::BitsetDcl(b) => vec![b.ident.clone()],
            hir::ConstrTypeDcl::BitmaskDcl(b) => vec![b.ident.clone()],
            hir::ConstrTypeDcl::StructForwardDcl(s) => vec![s.ident.clone()],
            hir::ConstrTypeDcl::UnionForwardDcl(u) => vec![u.ident.clone()],
        },
        hir::Definition::TypeDcl(ty) => match ty {
            hir::TypeDcl::ConstrTypeDcl(c) => match c {
                hir::ConstrTypeDcl::StructDcl(s) => vec![s.ident.clone()],
                hir::ConstrTypeDcl::EnumDcl(e) => vec![e.ident.clone()],
                hir::ConstrTypeDcl::UnionDef(u) => vec![u.ident.clone()],
                hir::ConstrTypeDcl::BitsetDcl(b) => vec![b.ident.clone()],
                hir::ConstrTypeDcl::BitmaskDcl(b) => vec![b.ident.clone()],
                hir::ConstrTypeDcl::StructForwardDcl(s) => vec![s.ident.clone()],
                hir::ConstrTypeDcl::UnionForwardDcl(u) => vec![u.ident.clone()],
            },
            hir::TypeDcl::TypedefDcl(t) => t
                .decl
                .iter()
                .map(|d| match d {
                    hir::Declarator::SimpleDeclarator(s) => s.0.clone(),
                    hir::Declarator::ArrayDeclarator(a) => a.ident.clone(),
                })
                .collect(),
            _ => vec![],
        },
        hir::Definition::ConstDcl(c) => vec![c.ident.clone()],
        hir::Definition::ExceptDcl(e) => vec![e.ident.clone()],
        _ => vec![],
    }
}

pub(crate) struct RustAxumCodegen;

impl crate::jsonrpc::Codegen for RustAxumCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(hashmap! {
            "expand_interface" => false,
            "hir_kind" => "http",
            "enable_client" => true,
            "enable_server" => true,
            "enable_render_header" => true,
            "enable_serialize" => true,
            "enable_deserialize" => true,
            "enable_metadata" => true
        })
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        path: String,
        props: ::xidl_parser::hir::ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let rest_hir = input_hir.into_rest_hir();
        generate(rest_hir, Path::new(&path), props)
            .map_err(|err| crate::jsonrpc::RpcError::new(err.to_string()))
    }
}
