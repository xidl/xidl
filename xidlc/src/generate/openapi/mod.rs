mod builder;
mod context;
mod naming;
mod operation;
mod schema;
mod security;
mod stream;

#[cfg(test)]
mod tests;

use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::openapi::{InfoBuilder, OpenApi, OpenApiBuilder};
use serde_json::Value;
use std::collections::HashMap;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;
use xidl_parser::rest_hir::RestHirDocument;

pub(crate) struct OpenApiCodegen;

#[async_trait::async_trait]
impl crate::jsonrpc::Codegen for OpenApiCodegen {
    async fn get_engine_version(&self) -> Result<String, xidl_jsonrpc::Error> {
        Ok("*".to_string())
    }

    async fn get_properties(&self) -> Result<ParserProperties, xidl_jsonrpc::Error> {
        Ok(HashMap::from([(
            "hir_kind".to_string(),
            serde_json::Value::String("http".to_string()),
        )]))
    }

    async fn generate(
        &self,
        input_hir: crate::jsonrpc::CodegenInput,
        input: String,
        _props: ParserProperties,
    ) -> Result<Vec<Artifact>, xidl_jsonrpc::Error> {
        let rest_hir = input_hir.into_rest_hir();
        let openapi = render_openapi_json(&rest_hir.spec, &rest_hir)?;
        let content = serde_json::to_string_pretty(&openapi)?;

        let stem = std::path::Path::new(&input)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("openapi");

        Ok(vec![Artifact::new_file(ArtifactFile {
            path: format!("openapi_{stem}.json"),
            content,
        })])
    }
}

fn render_openapi_json(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
) -> Result<Value, serde_json::Error> {
    let ctx = render_openapi(spec, rest_hir);
    let version = select_openapi_version(&ctx);
    let mut value = serde_json::to_value(ctx.document)?;
    if let Some(openapi) = value.get_mut("openapi") {
        *openapi = Value::String(version.to_string());
    }
    for patch in ctx.stream_patches {
        stream::patch_openapi_stream_content(&mut value, &patch);
    }
    Ok(value)
}

fn select_openapi_version(ctx: &RenderedOpenApi) -> &'static str {
    if ctx.stream_patches.is_empty() {
        "3.1.0"
    } else {
        "3.2.0"
    }
}

/// Renders an OpenAPI document from an XIDL specification and projected REST HIR.
pub fn render_openapi(spec: &hir::Specification, rest_hir: &RestHirDocument) -> RenderedOpenApi {
    let ctx = context::OpenApiContext::new(rest_hir).collect(spec, &[], rest_hir);
    let mut components = crate::openapi::ComponentsBuilder::new();
    for (name, schema) in ctx.schemas {
        components = components.schema(name, schema);
    }
    for (name, scheme) in ctx.security_schemes {
        components = components.security_scheme(name, scheme);
    }

    let title = ctx.info_title.as_deref().unwrap_or("xidl");
    let version = ctx.info_version.as_deref().unwrap_or("0.1.0");
    let servers = (!ctx.servers.is_empty()).then_some(ctx.servers);
    let document = OpenApiBuilder::new()
        .info(InfoBuilder::new().title(title).version(version).build())
        .paths(ctx.paths.build())
        .components(Some(components.build()))
        .servers(servers)
        .build();

    RenderedOpenApi {
        document,
        stream_patches: ctx.stream_patches,
    }
}

/// Rendered OpenAPI output plus delayed stream-specific patches needed after serialization.
pub struct RenderedOpenApi {
    pub document: OpenApi,
    stream_patches: Vec<stream::OpenApiStreamPatch>,
}
