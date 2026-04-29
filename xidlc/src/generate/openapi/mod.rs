use crate::generate::http_hir::HttpHirDocument;
use crate::generate::openapi::context::{
    RenderedOpenApi, patch_openapi_stream_content, render_openapi,
};
use crate::generate::openapi::render::render_openapi_json_string;
use crate::jsonrpc::{Artifact, ArtifactFile};
use serde_json::Value;
use std::collections::HashMap;
use xidl_parser::hir::{ParserProperties, Specification};

pub mod context;
pub mod operation;
pub mod render;
pub mod schema;
#[cfg(test)]
mod tests;
pub mod utils;

/// OpenAPI generator implementation.
pub(crate) struct OpenApiCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenApiCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::new())
    }

    async fn generate(
        &self,
        hir: Specification,
        _path: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let http_hir =
            HttpHirDocument::from_props(&props).map_err(|err| xidl_jsonrpc::Error::Rpc {
                code: xidl_jsonrpc::ErrorCode::ServerError,
                message: err.to_string(),
                data: None,
            })?;

        let rendered = render_openapi(&hir, &http_hir);
        let content = render_json_artifact(&rendered).map_err(|err| xidl_jsonrpc::Error::Rpc {
            code: xidl_jsonrpc::ErrorCode::ServerError,
            message: err.to_string(),
            data: None,
        })?;

        Ok(vec![Artifact::new_file(ArtifactFile {
            path: "openapi.json".to_string(),
            content,
        })])
    }
}

fn render_json_artifact(rendered: &RenderedOpenApi) -> Result<String, Box<dyn std::error::Error>> {
    let mut value = serde_json::to_value(&rendered.document)?;
    let version = if rendered.stream_patches.is_empty() {
        "3.1.0"
    } else {
        "3.2.0"
    };

    if let Some(openapi) = value.get_mut("openapi") {
        *openapi = Value::String(version.to_string());
    }

    for patch in &rendered.stream_patches {
        patch_openapi_stream_content(&mut value, patch);
    }

    Ok(render_openapi_json_string(&value)?)
}
