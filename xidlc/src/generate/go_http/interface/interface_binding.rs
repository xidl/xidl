use crate::error::IdlcResult;
use crate::generate::go_http::{MethodMeta, ParamMeta, definition};
use std::fmt::Write;

use super::GoHttpRenderer;
use super::interface_templates::{
    ClientBuildRequestMethod, ClientBuildRequestTemplate, DecodeResponseMethod,
    DecodeResponseTemplate, MethodTemplateParam, RequestBindingMethod, RequestBindingTemplate,
    ResponseWriteMethod, ResponseWriteTemplate,
};

pub(super) fn render_client_build_request(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut query_encode = String::new();
    for param in &method.query_params {
        definition::emit_query_encode(&mut query_encode, param)?;
    }
    let mut header_encode = String::new();
    for param in &method.header_params {
        definition::emit_header_encode(&mut header_encode, param)?;
    }
    let mut cookie_encode = String::new();
    for param in &method.cookie_params {
        definition::emit_cookie_encode(&mut cookie_encode, param)?;
    }
    let ctx = ClientBuildRequestTemplate {
        method: ClientBuildRequestMethod {
            struct_prefix: &method.struct_prefix,
            http_method_name: definition::http_method_name(method.http_method),
            request_body_struct: method.request_body_struct.as_deref(),
            request_body_direct_field: method.request_body_direct_field.as_deref(),
            request_body_direct_ty: method.request_body_direct_ty.as_deref(),
            request_content_type: &method.request_content_type,
            response_content_type: &method.response_content_type,
            body_params: template_params(&method.body_params),
            has_query_params: !method.query_params.is_empty(),
            has_body_params: !method.body_params.is_empty(),
            has_security: !method.security.is_empty(),
            query_encode,
            header_encode,
            cookie_encode,
        },
    };
    out.push_str(&renderer.render_template("client_build_request.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

pub(super) fn render_client_decode_response(out: &mut String, method: &MethodMeta) {
    writeln!(
        out,
        "\tdecoded, err := decode{}Response(resp)",
        method.struct_prefix
    )
    .unwrap();
    writeln!(out, "\tif err != nil {{ return nil, err }}").unwrap();
    writeln!(out, "\treturn &decoded, nil").unwrap();
}

pub(super) fn render_decode_response_fn(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut response_header_decode = String::new();
    for param in &method.response_header_params {
        definition::emit_response_header_decode(&mut response_header_decode, param)?;
    }
    let mut response_cookie_decode = String::new();
    for param in &method.response_cookie_params {
        definition::emit_response_cookie_decode(&mut response_cookie_decode, param)?;
    }
    let ctx = DecodeResponseTemplate {
        method: DecodeResponseMethod {
            struct_prefix: &method.struct_prefix,
            response_struct: &method.response_struct,
            response_body_struct: method.response_body_struct.as_deref(),
            response_body_direct_field: method.response_body_direct_field.as_deref(),
            response_body_direct_ty: method.response_body_direct_ty.as_deref(),
            response_content_type: &method.response_content_type,
            return_ty: method.return_ty.as_deref(),
            response_body_params: template_params(&method.response_body_params),
            response_header_decode,
            response_cookie_decode,
        },
    };
    out.push_str(&renderer.render_template("decode_response.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

pub(super) fn render_request_binding(
    out: &mut String,
    method: &MethodMeta,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut path_bindings = String::new();
    for param in &method.path_params {
        definition::emit_request_bind(&mut path_bindings, "r", param, "Path")?;
    }
    let mut query_bindings = String::new();
    for param in &method.query_params {
        definition::emit_request_bind(&mut query_bindings, "r.URL.Query()", param, "Query")?;
    }
    let mut header_bindings = String::new();
    for param in &method.header_params {
        definition::emit_request_bind(&mut header_bindings, "r.Header", param, "Header")?;
    }
    let mut cookie_bindings = String::new();
    for param in &method.cookie_params {
        definition::emit_request_bind(&mut cookie_bindings, "r", param, "Cookie")?;
    }
    let ctx = RequestBindingTemplate {
        method: RequestBindingMethod {
            is_client_stream: matches!(
                method.stream_kind,
                Some(xidl_parser::http_hir::semantics::HttpStreamKind::Client)
            ),
            request_struct: &method.request_struct,
            request_body_struct: method.request_body_struct.as_deref(),
            request_body_direct_field: method.request_body_direct_field.as_deref(),
            request_body_direct_ty: method.request_body_direct_ty.as_deref(),
            request_content_type: &method.request_content_type,
            body_params: template_params(&method.body_params),
            path_bindings,
            query_bindings,
            header_bindings,
            cookie_bindings,
        },
    };
    out.push_str(&renderer.render_template("request_binding.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

pub(super) fn render_response_write(
    out: &mut String,
    method: &MethodMeta,
    value: &str,
    renderer: &GoHttpRenderer,
) -> IdlcResult<()> {
    let mut response_header_encode = String::new();
    for param in &method.response_header_params {
        definition::emit_response_header_encode(&mut response_header_encode, param, value)?;
    }
    let mut response_cookie_encode = String::new();
    for param in &method.response_cookie_params {
        definition::emit_response_cookie_encode(&mut response_cookie_encode, param, value)?;
    }
    let ctx = ResponseWriteTemplate {
        method: ResponseWriteMethod {
            response_body_struct: method.response_body_struct.as_deref(),
            response_body_direct_field: method.response_body_direct_field.as_deref(),
            response_body_direct_ty: method.response_body_direct_ty.as_deref(),
            response_content_type: &method.response_content_type,
            return_ty: method.return_ty.as_deref(),
            response_body_params: template_params(&method.response_body_params),
            response_header_encode,
            response_cookie_encode,
        },
        value,
    };
    out.push_str(&renderer.render_template("response_write.go.j2", &ctx)?);
    out.push('\n');
    Ok(())
}

fn template_params(params: &[ParamMeta]) -> Vec<MethodTemplateParam> {
    params
        .iter()
        .map(|param| MethodTemplateParam {
            field_name: param.field_name.clone(),
            wire_name: param.wire_name.clone(),
        })
        .collect()
}
