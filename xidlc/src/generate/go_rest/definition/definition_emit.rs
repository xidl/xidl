use crate::error::IdlcResult;

use super::definition_templates::{
    EncodeTemplate, RequestBindTemplate, ResponseCookieDecodeTemplate, ResponseEncodeTemplate,
    ResponseHeaderDecodeTemplate,
};
use crate::generate::go_rest::{GoRestRenderer, ParamMeta};

pub(crate) fn emit_request_bind(
    out: &mut String,
    _source_expr: &str,
    param: &ParamMeta,
    source_kind: &str,
) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    let wire = &param.wire_name;
    let call = match source_kind {
        "Path" => format!("xidlgohttp.GinPathString(c, {wire:?})"),
        "Query" => format!("xidlgohttp.QueryString(c.Request.URL.Query(), {wire:?})"),
        "Header" => format!("xidlgohttp.HeaderString(c.Request.Header, {wire:?})"),
        "Cookie" => format!("xidlgohttp.GinCookieString(c, {wire:?})"),
        _ => return Ok(()),
    };
    if param.ty != "string" && param.ty != "uint32" && param.ty != "int32" && param.ty != "bool" {
        return Ok(());
    }
    let renderer = GoRestRenderer::new()?;
    out.push_str(&renderer.render_template(
        "request_bind.go.j2",
        &RequestBindTemplate {
            field: &field,
            call: &call,
            ty: &param.ty,
            optional_string: source_kind == "Query" && param.optional && param.ty == "string",
        },
    )?);
    out.push('\n');
    Ok(())
}

pub(crate) fn emit_query_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "query_encode.go.j2", supports_query_encode)
}

pub(crate) fn emit_header_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "header_encode.go.j2", |ty| {
        ty == "string" || ty == "uint32"
    })
}

pub(crate) fn emit_cookie_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "cookie_encode.go.j2", |ty| ty == "string")
}

pub(crate) fn emit_response_header_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    emit_response_encode(out, param, &field, "response_header_encode.go.j2", |ty| {
        ty == "string" || ty == "uint32"
    })
}

pub(crate) fn emit_response_cookie_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    emit_response_encode(out, param, &field, "response_cookie_encode.go.j2", |ty| {
        ty == "string"
    })
}

pub(crate) fn emit_response_header_decode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("out.{}", param.field_name);
    if param.ty == "string" || param.ty == "uint32" {
        let renderer = GoRestRenderer::new()?;
        out.push_str(&renderer.render_template(
            "response_header_decode.go.j2",
            &ResponseHeaderDecodeTemplate {
                wire_name: &param.wire_name,
                field: &field,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

pub(crate) fn emit_response_cookie_decode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    if param.ty == "string" {
        let renderer = GoRestRenderer::new()?;
        out.push_str(&renderer.render_template(
            "response_cookie_decode.go.j2",
            &ResponseCookieDecodeTemplate {
                wire_name: &param.wire_name,
                field_name: &param.field_name,
                ty: &param.ty,
            },
        )?);
        out.push('\n');
    }
    Ok(())
}

fn emit_encode(
    out: &mut String,
    param: &ParamMeta,
    template: &str,
    supports: impl Fn(&str) -> bool,
) -> IdlcResult<()> {
    if !supports(&param.ty) {
        return Ok(());
    }
    let renderer = GoRestRenderer::new()?;
    let field = format!("req.{}", param.field_name);
    out.push_str(&renderer.render_template(
        template,
        &EncodeTemplate {
            wire_name: &param.wire_name,
            field: &field,
            ty: &param.ty,
        },
    )?);
    out.push('\n');
    Ok(())
}

fn emit_response_encode(
    out: &mut String,
    param: &ParamMeta,
    field: &str,
    template: &str,
    supports: impl Fn(&str) -> bool,
) -> IdlcResult<()> {
    if !supports(&param.ty) {
        return Ok(());
    }
    let renderer = GoRestRenderer::new()?;
    out.push_str(&renderer.render_template(
        template,
        &ResponseEncodeTemplate {
            wire_name: &param.wire_name,
            field,
            ty: &param.ty,
        },
    )?);
    out.push('\n');
    Ok(())
}

fn supports_query_encode(ty: &str) -> bool {
    matches!(ty, "string" | "*string" | "uint32" | "int32" | "bool")
}
