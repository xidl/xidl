use crate::error::IdlcResult;

use super::definition_templates::{
    EncodeTemplate, RequestBindTemplate, ResponseCookieDecodeTemplate, ResponseEncodeTemplate,
    ResponseHeaderDecodeTemplate,
};
use crate::generate::go_rest::{GoRestRenderer, ParamMeta, ParamSource};

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
    let renderer = GoRestRenderer::new()?;
    out.push_str(&renderer.render_template(
        "request_bind.go.j2",
        &RequestBindTemplate {
            field: &field,
            call: &call,
            requires_presence: param.requires_presence,
        },
    )?);
    out.push('\n');
    Ok(())
}

pub(crate) fn emit_bidi_request_bind(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("req.{}", param.field_name);
    let wire = &param.wire_name;
    let call = match param.source {
        ParamSource::Path => format!("xidlgohttp.GinPathString(c, {wire:?})"),
        ParamSource::Query => format!("xidlgohttp.QueryString(c.Request.URL.Query(), {wire:?})"),
        ParamSource::Header => format!("xidlgohttp.HeaderString(c.Request.Header, {wire:?})"),
        ParamSource::Cookie => format!("xidlgohttp.GinCookieString(c, {wire:?})"),
        ParamSource::Body => return Ok(()),
    };
    let renderer = GoRestRenderer::new()?;
    out.push_str(&renderer.render_template(
        "request_bind.go.j2",
        &RequestBindTemplate {
            field: &field,
            call: &call,
            requires_presence: param.requires_presence,
        },
    )?);
    out.push('\n');
    Ok(())
}

pub(crate) fn emit_query_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "query_encode.go.j2")
}

pub(crate) fn emit_header_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "header_encode.go.j2")
}

pub(crate) fn emit_cookie_encode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    emit_encode(out, param, "cookie_encode.go.j2")
}

pub(crate) fn emit_response_header_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    emit_response_encode(out, param, &field, "response_header_encode.go.j2")
}

pub(crate) fn emit_response_cookie_encode(
    out: &mut String,
    param: &ParamMeta,
    value: &str,
) -> IdlcResult<()> {
    let field = format!("{value}.{}", param.field_name);
    emit_response_encode(out, param, &field, "response_cookie_encode.go.j2")
}

pub(crate) fn emit_response_header_decode(out: &mut String, param: &ParamMeta) -> IdlcResult<()> {
    let field = format!("out.{}", param.field_name);
    {
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
    {
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

fn emit_encode(out: &mut String, param: &ParamMeta, template: &str) -> IdlcResult<()> {
    let renderer = GoRestRenderer::new()?;
    let field = format!("req.{}", param.field_name);
    out.push_str(&renderer.render_template(
        template,
        &EncodeTemplate {
            wire_name: &param.wire_name,
            field: &field,
            ty: &param.ty,
            optional: param.optional,
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
) -> IdlcResult<()> {
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
