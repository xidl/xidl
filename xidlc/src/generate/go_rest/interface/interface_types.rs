use crate::error::IdlcResult;
use crate::generate::go_rest::{MethodMeta, ParamMeta, definition};
use std::fmt::Write;
use xidl_parser::rest_hir::semantics::{HttpApiKeyLocation, HttpSecurityRequirement};

use super::GoRestRenderer;
use super::interface_binding::render_decode_response_fn;

pub(super) fn render_method_types(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoRestRenderer,
) -> IdlcResult<()> {
    render_request_types(out, method);
    render_response_types(out, method);
    render_security_requirements(out, method);
    render_deprecated_info(out, method);
    definition::render_format_path_fn(out, method)?;
    writeln!(out).unwrap();
    render_decode_response_fn(out, method, renderer)?;
    Ok(())
}

fn render_request_types(out: &mut String, method: &MethodMeta) {
    writeln!(out, "type {} struct {{", method.request_struct).unwrap();
    for param in &method.request_params {
        let ty = if param.optional {
            format!("*{}", param.ty)
        } else {
            param.ty.clone()
        };
        writeln!(
            out,
            "\t{} {} `xjson:\"{}\" form:\"{}\"`",
            param.field_name, ty, param.raw_name, param.raw_name
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    if let Some(body_struct) = &method.request_body_struct {
        writeln!(out, "type {body_struct} struct {{").unwrap();
        for param in &method.body_params {
            let ty = if param.optional {
                format!("*{}", param.ty)
            } else {
                param.ty.clone()
            };
            writeln!(
                out,
                "\t{} {} `xjson:\"{}\" form:\"{}\"`",
                param.field_name, ty, param.wire_name, param.wire_name
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }
}

fn render_response_types(out: &mut String, method: &MethodMeta) {
    writeln!(out, "type {} struct {{", method.response_struct).unwrap();
    if let Some(return_ty) = &method.return_ty {
        writeln!(out, "\tReturn {return_ty} `xjson:\"return\"`").unwrap();
    }
    for param in response_params(method) {
        writeln!(
            out,
            "\t{} {} `xjson:\"{}\"`",
            param.field_name, param.ty, param.raw_name
        )
        .unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    if let Some(body_struct) = &method.response_body_struct {
        writeln!(out, "type {body_struct} struct {{").unwrap();
        if let Some(return_ty) = &method.return_ty {
            writeln!(out, "\tReturn {return_ty} `xjson:\"return\"`").unwrap();
        }
        for param in &method.response_body_params {
            writeln!(
                out,
                "\t{} {} `xjson:\"{}\"`",
                param.field_name, param.ty, param.raw_name
            )
            .unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }
}

fn render_security_requirements(out: &mut String, method: &MethodMeta) {
    writeln!(
        out,
        "func {}SecurityRequirements() []xidlgohttp.SecurityRequirement {{",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\treturn []xidlgohttp.SecurityRequirement{{").unwrap();
    for requirement in &method.security {
        match requirement {
            HttpSecurityRequirement::HttpBasic => writeln!(
                out,
                "\t\t{{Kind: xidlgohttp.SecurityBasic, Realm: {:?}}},",
                method
                    .basic_realm
                    .clone()
                    .unwrap_or_else(|| method.method_name.to_lowercase())
            )
            .unwrap(),
            HttpSecurityRequirement::HttpBearer => {
                writeln!(out, "\t\t{{Kind: xidlgohttp.SecurityBearer}},").unwrap()
            }
            HttpSecurityRequirement::ApiKey { location, name } => {
                let loc = match location {
                    HttpApiKeyLocation::Header => "ApiKeyHeader",
                    HttpApiKeyLocation::Query => "ApiKeyQuery",
                    HttpApiKeyLocation::Cookie => "ApiKeyCookie",
                };
                writeln!(out, "\t\t{{Kind: xidlgohttp.SecurityAPIKey, Location: xidlgohttp.{loc}, Name: {:?}}},", name).unwrap();
            }
            HttpSecurityRequirement::OAuth2 { scopes } => {
                writeln!(
                    out,
                    "\t\t{{Kind: xidlgohttp.SecurityOAuth2, Scopes: []string{{{}}}}},",
                    scopes
                        .iter()
                        .map(|scope| format!("{scope:?}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .unwrap();
            }
        }
    }
    writeln!(out, "\t}}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

fn render_deprecated_info(out: &mut String, method: &MethodMeta) {
    writeln!(
        out,
        "func {}Deprecated() xidlgohttp.DeprecatedInfo {{",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\treturn xidlgohttp.DeprecatedInfo{{").unwrap();
    writeln!(out, "\t\tDeprecated: {},", method.deprecated).unwrap();
    if let Some(since) = &method.deprecated_since {
        writeln!(out, "\t\tSince: {:?},", since).unwrap();
    }
    if let Some(after) = &method.deprecated_after {
        writeln!(out, "\t\tAfter: {:?},", after).unwrap();
    }
    if let Some(note) = &method.deprecated_note {
        writeln!(out, "\t\tNote: {:?},", note).unwrap();
    }
    writeln!(out, "\t}}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();
}

fn response_params(method: &MethodMeta) -> impl Iterator<Item = &ParamMeta> {
    method
        .response_body_params
        .iter()
        .chain(method.response_header_params.iter())
        .chain(method.response_cookie_params.iter())
}
