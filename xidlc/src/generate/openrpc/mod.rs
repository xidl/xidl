#[cfg(test)]
mod tests;

mod annotations;
mod context;
mod methods;
mod names;
mod schema;
mod schema_types;
mod units;

use serde_json::{Value, json};
use xidl_parser::hir::ParserProperties;
use xidl_parser::jsonrpc_hir::JsonRpcHirDocument;

use crate::jsonrpc::{Artifact, ArtifactFile};
use context::OpenRpcContext;
use units::DocumentUnits;

pub(crate) struct OpenRpcCodegen;

impl crate::jsonrpc::Codegen for OpenRpcCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(std::collections::HashMap::from([
            (
                "hir_kind".to_string(),
                serde_json::Value::String("jsonrpc".to_string()),
            ),
            (
                xidl_parser::versioning::VERSION_LAYOUT_PROPERTY.to_string(),
                serde_json::Value::String(xidl_parser::versioning::LAYOUT_PER_VERSION.to_string()),
            ),
        ]))
    }

    fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        _path: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let hir = input_hir.into_jsonrpc_hir();
        let units = DocumentUnits::of(&props);
        let openrpc = render_openrpc(&hir, &units);
        let content = serde_json::to_string_pretty(&openrpc)?;
        // Each unit is a document of its own, so the file carries its name.
        let stem = std::path::Path::new(&_path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("openrpc");
        let path = match api_version(&props) {
            Some(version) => format!("openrpc_{stem}.{version}.json"),
            None => format!("openrpc_{stem}.json"),
        };
        Ok(vec![Artifact::new_file(ArtifactFile { path, content })])
    }
}

/// API version of this projection, set by the HIR stage for versioned contracts.
fn api_version(props: &ParserProperties) -> Option<&str> {
    props
        .get(xidl_parser::versioning::API_VERSION_PROPERTY)
        .and_then(serde_json::Value::as_str)
}

pub(crate) fn render_openrpc(doc: &JsonRpcHirDocument, units: &DocumentUnits) -> Value {
    let mut ctx = OpenRpcContext::default();
    ctx.collect_doc(doc);
    ctx.methods.sort_by(|left, right| {
        let left_name = left.get("name").and_then(Value::as_str).unwrap_or_default();
        let right_name = right
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default();
        left_name.cmp(right_name)
    });

    let mut out = json!({
        "openrpc": "1.3.2",
        "info": {
            "title": ctx.info_title.as_deref().unwrap_or("xidl"),
            "version": ctx.info_version.as_deref().unwrap_or("0.1.0"),
        },
        "methods": ctx.methods,
    });

    if !ctx.schemas.is_empty() {
        out["components"] = json!({
            "schemas": ctx.schemas.clone(),
        });
    }
    units.rewrite(&mut out, &ctx.schemas);

    out
}
