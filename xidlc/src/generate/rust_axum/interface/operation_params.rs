use crate::error::IdlcResult;
use crate::generate::http_hir::{HttpOperation, semantics::has_optional_annotation};
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::annotations::{
    field_rename, field_rename_raw, has_flatten_annotation, serde_rename,
};
use crate::generate::rust_axum::interface::context::ParamContext;
use crate::generate::rust_axum::interface::params::{
    ParamDirection, ParamSource, cookie_is_multi, cookie_item_is_primitive, cookie_item_is_string,
    cookie_item_ty, find_http_param, header_is_multi, header_item_is_primitive,
    header_item_is_string, header_item_ty, http_param_kind, param_direction, param_source_code,
};
use crate::generate::rust_axum::interface::types::{
    axum_type, render_param_type, transport_param_type,
};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, TypeRegistry, decode_expr, encode_expr,
};
use xidl_parser::hir;

pub struct OperationParamsContext {
    pub param_list: Vec<String>,
    pub param_names: Vec<String>,
    pub path_params: Vec<ParamContext>,
    pub query_params: Vec<ParamContext>,
    pub header_params: Vec<ParamContext>,
    pub cookie_params: Vec<ParamContext>,
    pub body_params: Vec<ParamContext>,
    pub request_params: Vec<ParamContext>,
    pub response_params: Vec<ParamContext>,
    pub response_body_params: Vec<ParamContext>,
    pub response_header_params: Vec<ParamContext>,
    pub response_cookie_params: Vec<ParamContext>,
}

pub fn process_operation_params(
    op: &hir::OpDcl,
    http_op: &HttpOperation,
    path: &str,
    registry: &TypeRegistry,
    transport: &mut TransportTracker,
) -> IdlcResult<OperationParamsContext> {
    let mut ctx = OperationParamsContext {
        param_list: Vec::new(),
        param_names: Vec::new(),
        path_params: Vec::new(),
        query_params: Vec::new(),
        header_params: Vec::new(),
        cookie_params: Vec::new(),
        body_params: Vec::new(),
        request_params: Vec::new(),
        response_params: Vec::new(),
        response_body_params: Vec::new(),
        response_header_params: Vec::new(),
        response_cookie_params: Vec::new(),
    };

    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);

    for param in params {
        let optional = has_optional_annotation(&param.annotations);
        let flatten = has_flatten_annotation(&param.annotations);
        let inner_ty = axum_type(&param.ty);
        let ty = render_param_type(&param.ty, param.attr.as_ref(), optional);
        let name = rust_ident(&param.declarator.0);
        let direction = param_direction(param.attr.as_ref());

        if matches!(direction, ParamDirection::Out | ParamDirection::InOut)
            && let Some(shared) = find_http_param(&http_op.response_params, &param.declarator.0)
        {
            let response_ctx = ParamContext {
                name: name.clone(),
                raw_name: param.declarator.0.clone(),
                wire_name: shared.wire_name.clone(),
                path_template_name: String::new(),
                ty: inner_ty.clone(),
                in_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::In,
                    transport,
                    registry,
                )?,
                out_ty: transport_param_type(
                    &param.ty,
                    optional,
                    TransportDirection::Out,
                    transport,
                    registry,
                )?,
                source: param_source_code(http_param_kind(shared.kind)),
                serde_rename: field_rename(&param.annotations, &name)
                    .or_else(|| serde_rename(&param.declarator.0, &name)),
                header_is_multi: header_is_multi(&param.ty),
                header_item_ty: header_item_ty(&param.ty),
                header_item_is_string: header_item_is_string(&param.ty),
                header_item_is_primitive: header_item_is_primitive(&param.ty),
                cookie_is_multi: cookie_is_multi(&param.ty),
                cookie_item_ty: cookie_item_ty(&param.ty),
                cookie_item_is_string: cookie_item_is_string(&param.ty),
                cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
                optional,
                inner_ty: inner_ty.clone(),
                flatten: false,
                in_expr: decode_expr(&name, &param.ty, registry)?,
                out_expr: encode_expr(&name, &param.ty, registry)?,
                field_in_expr: decode_expr(&format!("value.{name}"), &param.ty, registry)?,
                field_out_expr: encode_expr(&format!("value.{name}"), &param.ty, registry)?,
            };
            match http_param_kind(shared.kind) {
                ParamSource::Header => ctx.response_header_params.push(response_ctx.clone()),
                ParamSource::Cookie => ctx.response_cookie_params.push(response_ctx.clone()),
                _ => ctx.response_body_params.push(response_ctx.clone()),
            }
            ctx.response_params.push(response_ctx);
        }
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        let Some(shared) = find_http_param(&http_op.request_params, &param.declarator.0) else {
            continue;
        };
        let source = http_param_kind(shared.kind);
        let wire_name = shared.wire_name.clone();
        ctx.param_list.push(format!("{name}: {ty}"));
        ctx.param_names.push(name.clone());
        let serde_name = if matches!(source, ParamSource::Body) {
            field_rename_raw(&param.annotations).unwrap_or_else(|| wire_name.clone())
        } else {
            wire_name.clone()
        };
        let p_ctx = ParamContext {
            name: name.clone(),
            raw_name: param.declarator.0.clone(),
            path_template_name: if matches!(source, ParamSource::Path)
                && path_param_is_catch_all(path, &wire_name)
            {
                format!("*{wire_name}")
            } else {
                wire_name.clone()
            },
            wire_name,
            ty,
            in_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::In,
                transport,
                registry,
            )?,
            out_ty: transport_param_type(
                &param.ty,
                optional,
                TransportDirection::Out,
                transport,
                registry,
            )?,
            inner_ty: inner_ty.clone(),
            source: param_source_code(source),
            serde_rename: serde_rename(&serde_name, &name),
            header_is_multi: header_is_multi(&param.ty),
            header_item_ty: header_item_ty(&param.ty),
            header_item_is_string: header_item_is_string(&param.ty),
            header_item_is_primitive: header_item_is_primitive(&param.ty),
            cookie_is_multi: cookie_is_multi(&param.ty),
            cookie_item_ty: cookie_item_ty(&param.ty),
            cookie_item_is_string: cookie_item_is_string(&param.ty),
            cookie_item_is_primitive: cookie_item_is_primitive(&param.ty),
            optional,
            flatten,
            in_expr: decode_expr(&name, &param.ty, registry)?,
            out_expr: encode_expr(&name, &param.ty, registry)?,
            field_in_expr: decode_expr(&format!("value.{name}"), &param.ty, registry)?,
            field_out_expr: encode_expr(&format!("value.{name}"), &param.ty, registry)?,
        };
        ctx.request_params.push(p_ctx.clone());
        match source {
            ParamSource::Path => ctx.path_params.push(p_ctx),
            ParamSource::Query => ctx.query_params.push(p_ctx),
            ParamSource::Header => ctx.header_params.push(p_ctx),
            ParamSource::Cookie => ctx.cookie_params.push(p_ctx),
            ParamSource::Body => ctx.body_params.push(p_ctx),
        }
    }
    Ok(ctx)
}

fn path_param_is_catch_all(path: &str, name: &str) -> bool {
    path.contains(&format!("{{*{name}}}"))
}
