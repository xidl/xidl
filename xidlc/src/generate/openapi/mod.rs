mod builder;
mod context;
mod naming;
mod operation;
mod schema;
mod scope;
mod security;
mod stream;
mod units;

#[cfg(test)]
mod tests;

use crate::error::{IdlcError, IdlcResult};
use crate::jsonrpc::{Artifact, ArtifactFile};
use crate::openapi::{InfoBuilder, OpenApi, OpenApiBuilder, Tag};
use serde_json::Value;
use std::collections::HashMap;
use xidl_parser::hir;
use xidl_parser::hir::ParserProperties;
use xidl_parser::rest_hir::RestHirDocument;

use units::DocumentUnits;

pub(crate) struct OpenApiCodegen;

impl crate::jsonrpc::Codegen for OpenApiCodegen {
    fn get_engine_version(&self) -> Result<String, crate::jsonrpc::RpcError> {
        Ok("*".to_string())
    }

    fn get_properties(&self) -> Result<ParserProperties, crate::jsonrpc::RpcError> {
        Ok(HashMap::from([
            (
                "hir_kind".to_string(),
                serde_json::Value::String("http".to_string()),
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
        input: String,
        props: ParserProperties,
    ) -> Result<Vec<Artifact>, crate::jsonrpc::RpcError> {
        let rest_hir = input_hir.into_rest_hir();
        let units = DocumentUnits::of(&props);
        let version = OpenApiVersion::from_props(&props)?;
        let openapi = render_openapi_json(&rest_hir.spec, &rest_hir, &units, version)?;
        let content = serde_json::to_string_pretty(&openapi)?;

        let stem = std::path::Path::new(&input)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("openapi");
        let path = match api_version(&props) {
            Some(version) => format!("openapi_{stem}.{version}.json"),
            None => format!("openapi_{stem}.json"),
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

/// OpenAPI document version requested through the `--version` generator option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OpenApiVersion {
    /// Keep the projection's own choice, upgrading streams to OpenAPI 3.2.
    Auto,
    /// Force an OpenAPI 3.1 document, rendering streams best-effort.
    V31,
}

impl OpenApiVersion {
    /// Reads the requested version from the generator properties.
    fn from_props(props: &ParserProperties) -> IdlcResult<Self> {
        match props
            .get(crate::driver::OPENAPI_VERSION_PROPERTY)
            .and_then(serde_json::Value::as_str)
        {
            None | Some("auto") => Ok(Self::Auto),
            Some("3.1") => Ok(Self::V31),
            Some(other) => Err(IdlcError::fmt(format!(
                "xidl openapi: unknown --version value '{other}'; expected 'auto' or '3.1'"
            ))),
        }
    }

    /// Document version for a projection, given whether it has stream patches.
    fn document_version(self, has_streams: bool) -> &'static str {
        match (self, has_streams) {
            (Self::V31, _) | (Self::Auto, false) => "3.1.0",
            (Self::Auto, true) => "3.2.0",
        }
    }
}

fn render_openapi_json(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
    units: &DocumentUnits,
    version: OpenApiVersion,
) -> IdlcResult<Value> {
    let ctx = render_openapi(spec, rest_hir, units)?;
    let document_version = version.document_version(!ctx.stream_patches.is_empty());
    let mut value = serde_json::to_value(ctx.document)?;
    if let Some(openapi) = value.get_mut("openapi") {
        *openapi = Value::String(document_version.to_string());
    }
    // Forced 3.1 keeps the media type's best-effort `schema`; the 3.2-only
    // `itemSchema` rewrite is skipped instead of being approximated.
    if version == OpenApiVersion::Auto {
        for patch in ctx.stream_patches {
            stream::patch_openapi_stream_content(&mut value, &patch);
        }
    }
    Ok(value)
}

/// Renders an OpenAPI document from an XIDL specification and projected REST HIR.
pub(crate) fn render_openapi(
    spec: &hir::Specification,
    rest_hir: &RestHirDocument,
    units: &DocumentUnits,
) -> IdlcResult<RenderedOpenApi> {
    let names = scope::SchemaNames::collect(spec);
    let ctx = context::OpenApiContext::new(rest_hir).collect(
        spec,
        scope::SchemaScope::new(&names, &[], units),
        rest_hir,
    )?;
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
    let tags = (!ctx.tags.is_empty()).then(|| ctx.tags.iter().map(Tag::new).collect::<Vec<_>>());
    let document = OpenApiBuilder::new()
        .info(InfoBuilder::new().title(title).version(version).build())
        .paths(ctx.paths.build())
        .components(Some(components.build()))
        .servers(servers)
        .tags(tags)
        .build();

    Ok(RenderedOpenApi {
        document,
        stream_patches: ctx.stream_patches,
    })
}

/// Rendered OpenAPI output plus delayed stream-specific patches needed after serialization.
pub struct RenderedOpenApi {
    pub document: OpenApi,
    stream_patches: Vec<stream::OpenApiStreamPatch>,
}
