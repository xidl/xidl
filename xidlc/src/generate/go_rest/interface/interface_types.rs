use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, ParamMeta, definition};
use serde::Serialize;
use xidl_parser::hir::HttpParameterLocation;
use xidl_parser::rest_hir::semantics::HttpSecurityRequirement;

use super::GoRestRenderer;
use super::interface_binding::render_decode_response_fn;

#[derive(Serialize)]
struct StructDefTemplate {
    name: String,
    fields: Vec<StructFieldTemplate>,
}

#[derive(Serialize)]
struct StructFieldTemplate {
    name: String,
    ty: String,
    tag: String,
}

#[derive(Serialize)]
struct SecurityTemplate {
    kind: &'static str,
    realm: Option<String>,
    location: Option<&'static str>,
    name: Option<String>,
}

#[derive(Serialize)]
struct MethodTypesTemplate<'a> {
    structs: Vec<StructDefTemplate>,
    struct_prefix: &'a str,
    security: Vec<SecurityTemplate>,
    deprecated: bool,
    deprecated_since: Option<&'a str>,
    deprecated_after: Option<&'a str>,
    deprecated_note: Option<&'a str>,
}

pub(super) fn render_method_types(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    let ctx = MethodTypesTemplate {
        structs: build_method_structs(method),
        struct_prefix: &method.struct_prefix,
        security: build_security_requirements(method),
        deprecated: method.deprecated,
        deprecated_since: method.deprecated_since.as_deref(),
        deprecated_after: method.deprecated_after.as_deref(),
        deprecated_note: method.deprecated_note.as_deref(),
    };
    out.push_str(&renderer.render_template("method_types.go.j2", &ctx)?);
    out.push_str("\n\n");
    definition::render_format_path_fn(out, method)?;
    out.push('\n');
    render_decode_response_fn(out, method, renderer)?;
    Ok(())
}

fn build_method_structs(method: &MethodMeta) -> Vec<StructDefTemplate> {
    let mut structs = Vec::new();
    if method.is_websocket {
        structs.push(StructDefTemplate {
            name: method.stream_in_ty.clone(),
            fields: method
                .body_params
                .iter()
                .map(|param| StructFieldTemplate {
                    name: param.field_name.clone(),
                    ty: optional_go_type(param),
                    tag: format!("xjson:\"{}\"", param.raw_name),
                })
                .collect(),
        });
        structs.push(StructDefTemplate {
            name: method.stream_out_ty.clone(),
            fields: response_struct_fields(method, response_params(method)),
        });
    }

    structs.push(StructDefTemplate {
        name: method.request_struct.clone(),
        fields: method
            .request_params
            .iter()
            .map(|param| StructFieldTemplate {
                name: param.field_name.clone(),
                ty: optional_go_type(param),
                tag: format!("xjson:\"{}\" form:\"{}\"", param.raw_name, param.raw_name),
            })
            .collect(),
    });

    if let Some(body_struct) = &method.request_body_struct {
        structs.push(StructDefTemplate {
            name: body_struct.clone(),
            fields: method
                .body_params
                .iter()
                .map(|param| StructFieldTemplate {
                    name: param.field_name.clone(),
                    ty: optional_go_type(param),
                    tag: format!("xjson:\"{}\" form:\"{}\"", param.wire_name, param.wire_name),
                })
                .collect(),
        });
    }

    structs.push(StructDefTemplate {
        name: method.response_struct.clone(),
        fields: response_struct_fields(method, response_params(method)),
    });

    if let Some(body_struct) = &method.response_body_struct {
        structs.push(StructDefTemplate {
            name: body_struct.clone(),
            fields: response_struct_fields(method, method.response_body_params.iter()),
        });
    }
    structs
}

fn response_struct_fields<'a>(
    method: &MethodMeta,
    params: impl Iterator<Item = &'a ParamMeta>,
) -> Vec<StructFieldTemplate> {
    let mut fields = Vec::new();
    if let Some(return_ty) = &method.return_ty {
        fields.push(StructFieldTemplate {
            name: "Return".to_string(),
            ty: return_ty.clone(),
            tag: "xjson:\"return\"".to_string(),
        });
    }
    for param in params {
        fields.push(StructFieldTemplate {
            name: param.field_name.clone(),
            ty: param.ty.clone(),
            tag: format!("xjson:\"{}\"", param.raw_name),
        });
    }
    fields
}

fn build_security_requirements(method: &MethodMeta) -> Vec<SecurityTemplate> {
    method
        .security
        .iter()
        .map(|requirement| match requirement {
            HttpSecurityRequirement::HttpBasic => SecurityTemplate {
                kind: "basic",
                realm: Some(
                    method
                        .basic_realm
                        .clone()
                        .unwrap_or_else(|| method.method_name.to_lowercase()),
                ),
                location: None,
                name: None,
            },
            HttpSecurityRequirement::HttpBearer => SecurityTemplate {
                kind: "bearer",
                realm: None,
                location: None,
                name: None,
            },
            HttpSecurityRequirement::ApiKey { location, name } => SecurityTemplate {
                kind: "api_key",
                realm: None,
                location: Some(match location {
                    HttpParameterLocation::Header => "ApiKeyHeader",
                    HttpParameterLocation::Query => "ApiKeyQuery",
                    HttpParameterLocation::Cookie => "ApiKeyCookie",
                }),
                name: Some(name.clone()),
            },
        })
        .collect()
}

fn optional_go_type(param: &ParamMeta) -> String {
    if param.optional {
        format!("*{}", param.ty)
    } else {
        param.ty.clone()
    }
}

fn response_params(method: &MethodMeta) -> impl Iterator<Item = &ParamMeta> {
    method
        .response_body_params
        .iter()
        .chain(method.response_header_params.iter())
        .chain(method.response_cookie_params.iter())
}
