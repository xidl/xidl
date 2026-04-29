use crate::error::{IdlcError, IdlcResult};
use crate::generate::rust_axum::interface::{ParamContext, ParamDirection, ParamSource, RenderEnv};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, decode_expr, encode_expr,
};
use convert_case::{Case, Casing};
use xidl_parser::hir;

pub(crate) fn transport_param_type(
    ty: &hir::TypeSpec,
    optional: bool,
    direction: TransportDirection,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
) -> IdlcResult<String> {
    let inner = transport.map_type(ty, direction, env.registry)?;
    Ok(if optional {
        format!("Option<{inner}>")
    } else {
        inner
    })
}

pub(crate) fn param_direction(attr: Option<&hir::ParamAttribute>) -> ParamDirection {
    match attr.map(|value| value.0.as_str()) {
        Some("out") => ParamDirection::Out,
        Some("inout") => ParamDirection::InOut,
        _ => ParamDirection::In,
    }
}

pub(crate) fn method_struct_prefix(interface_name: &str, method_name: &str) -> String {
    let interface = interface_name.strip_prefix("r#").unwrap_or(interface_name);
    let method = method_name.strip_prefix("r#").unwrap_or(method_name);
    format!(
        "{}{}",
        interface.to_case(Case::Pascal),
        method.to_case(Case::Pascal)
    )
}

pub(crate) fn op_return_ty(op_ty: &hir::OpTypeSpec) -> String {
    match op_ty {
        hir::OpTypeSpec::Void => "()".to_string(),
        hir::OpTypeSpec::TypeSpec(ty) => {
            crate::generate::rust_axum::interface::interface_types::axum_type(ty)
        }
    }
}

pub(crate) fn op_decode_expr(op_ty: &hir::OpTypeSpec, env: RenderEnv<'_>) -> IdlcResult<String> {
    match op_ty {
        hir::OpTypeSpec::Void => Ok("()".to_string()),
        hir::OpTypeSpec::TypeSpec(ty) => decode_expr("body", ty, env.registry),
    }
}

pub(crate) fn op_encode_expr(op_ty: &hir::OpTypeSpec, env: RenderEnv<'_>) -> IdlcResult<String> {
    match op_ty {
        hir::OpTypeSpec::Void => Ok("()".to_string()),
        hir::OpTypeSpec::TypeSpec(ty) => encode_expr("resp_value", ty, env.registry),
    }
}

pub(crate) fn request_struct_name(
    struct_prefix: &str,
    auth_in_request_struct: bool,
    has_request_params: bool,
) -> Option<String> {
    if auth_in_request_struct || has_request_params {
        Some(format!("{struct_prefix}Request"))
    } else {
        None
    }
}

pub(crate) fn request_payload_ty(
    request_ty: &str,
    auth_wrapper_struct: &Option<String>,
    is_client_stream: bool,
    is_bidi_stream: bool,
    response_ty: &str,
) -> String {
    if let Some(wrapper) = auth_wrapper_struct {
        return wrapper.clone();
    }
    if is_client_stream {
        format!("xidl_rust_axum::stream::NdjsonStream<{request_ty}>")
    } else if is_bidi_stream {
        format!("xidl_rust_axum::stream::BidiServerStream<{request_ty}, {response_ty}>")
    } else {
        request_ty.to_string()
    }
}

pub(crate) fn response_ty(
    ret: &str,
    return_is_unit: bool,
    response_params: &[ParamContext],
    struct_prefix: &str,
) -> String {
    let response_value_count = usize::from(!return_is_unit) + response_params.len();
    if response_value_count > 1 || (return_is_unit && response_value_count == 1) {
        format!("{struct_prefix}Response")
    } else if !return_is_unit {
        ret.to_string()
    } else if let Some(param) = response_params.first() {
        param.ty.clone()
    } else {
        "()".to_string()
    }
}

pub(crate) fn ensure_streaming_constraints(
    op: &hir::OpDcl,
    is_client_stream: bool,
    is_bidi_stream: bool,
    path_params: &[ParamContext],
    query_params: &[ParamContext],
    header_params: &[ParamContext],
    cookie_params: &[ParamContext],
) -> IdlcResult<()> {
    let has_non_body = !path_params.is_empty()
        || !query_params.is_empty()
        || !header_params.is_empty()
        || !cookie_params.is_empty();
    if is_client_stream && has_non_body {
        return Err(IdlcError::rpc(format!(
            "@client_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    if is_bidi_stream && has_non_body {
        return Err(IdlcError::rpc(format!(
            "@bidi_stream method '{}' currently supports body parameters only",
            op.ident
        )));
    }
    Ok(())
}

pub(crate) fn path_param_template_name(path: &str, source: ParamSource, wire_name: &str) -> String {
    if matches!(source, ParamSource::Path) && path.contains(&format!("{{*{wire_name}}}")) {
        format!("*{wire_name}")
    } else {
        wire_name.to_string()
    }
}

pub(crate) fn param_source_code(source: ParamSource) -> String {
    match source {
        ParamSource::Query => "ParamSource::Query".to_string(),
        ParamSource::Body => "ParamSource::Body".to_string(),
        ParamSource::Path => "ParamSource::Path".to_string(),
        ParamSource::Header => "ParamSource::Header".to_string(),
        ParamSource::Cookie => "ParamSource::Cookie".to_string(),
    }
}
