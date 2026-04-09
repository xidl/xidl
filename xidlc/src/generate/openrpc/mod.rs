#[cfg(test)]
mod tests;

mod annotations;
mod context;
mod methods;
mod methods_attr;
mod methods_attr_support;
mod names;
mod schema;
mod schema_types;

use serde_json::{Value, json};
use xidl_parser::hir;
use xidl_parser::hir::{ParserProperties, Specification};

use crate::jsonrpc::{Artifact, ArtifactFile};
use context::OpenRpcContext;

pub(crate) struct OpenRpcCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenRpcCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(std::collections::HashMap::new())
    }

    async fn generate(
        &self,
        hir: Specification,
        _path: String,
        _props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let openrpc = render_openrpc(&hir);
        let content = serde_json::to_string_pretty(&openrpc)?;
        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openrpc.json".to_string(),
            content,
        })])
    }
}

pub fn render_openrpc(spec: &hir::Specification) -> Value {
    let mut ctx = OpenRpcContext::default();
    ctx.collect_spec(spec, &[]);
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
            "schemas": ctx.schemas,
        });
    }

    out
}
