use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::interface_annotations::{
    field_rename, field_rename_raw, has_flatten_annotation, serde_rename,
};
use crate::generate::rust_axum::interface::interface_http::{find_http_param, http_param_kind};
use crate::generate::rust_axum::interface::interface_method_support::{
    param_direction, param_source_code, path_param_template_name, transport_param_type,
};
use crate::generate::rust_axum::interface::interface_types::{
    axum_type, cookie_is_multi, cookie_item_is_primitive, cookie_item_is_string, cookie_item_ty,
    header_is_multi, header_item_is_primitive, header_item_is_string, header_item_ty,
    render_param_type,
};
use crate::generate::rust_axum::interface::{ParamContext, ParamDirection, ParamSource, RenderEnv};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, decode_expr, encode_expr,
};
use xidl_parser::hir;
use xidl_parser::http_hir::{HttpOperation, semantics::has_optional_annotation};

#[derive(Default)]
pub(crate) struct MethodParams {
    pub(crate) params: Vec<String>,
    pub(crate) param_names: Vec<String>,
    pub(crate) server_params: Vec<String>,
    pub(crate) server_param_names: Vec<String>,
    pub(crate) path_params: Vec<ParamContext>,
    pub(crate) query_params: Vec<ParamContext>,
    pub(crate) header_params: Vec<ParamContext>,
    pub(crate) cookie_params: Vec<ParamContext>,
    pub(crate) body_params: Vec<ParamContext>,
    pub(crate) request_params: Vec<ParamContext>,
    pub(crate) response_params: Vec<ParamContext>,
    pub(crate) response_body_params: Vec<ParamContext>,
    pub(crate) response_header_params: Vec<ParamContext>,
    pub(crate) response_cookie_params: Vec<ParamContext>,
}

struct ParamBuildInput<'a> {
    param: &'a hir::ParamDcl,
    name: &'a str,
    wire_name: &'a str,
    path_template_name: String,
    inner_ty: &'a str,
    source: ParamSource,
    optional: bool,
    flatten: bool,
    serde_name: Option<String>,
}

pub(crate) fn collect_method_params(
    op: &hir::OpDcl,
    http_op: &HttpOperation,
    path: &str,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
    params: &mut MethodParams,
) -> IdlcResult<()> {
    let params_slice = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    for param in params_slice {
        let optional = has_optional_annotation(&param.annotations);
        let flatten = has_flatten_annotation(&param.annotations);
        let inner_ty = axum_type(&param.ty);
        let ty = render_param_type(&param.ty, optional);
        let name = rust_ident(&param.declarator.0);
        let direction = param_direction(param.attr.as_ref());

        maybe_add_response_param(
            param,
            http_op,
            ParamBuildInput {
                param,
                name: &name,
                wire_name: "",
                path_template_name: String::new(),
                inner_ty: &inner_ty,
                source: ParamSource::Body,
                optional,
                flatten,
                serde_name: None,
            },
            transport,
            env,
            params,
        )?;
        if matches!(direction, ParamDirection::Out) {
            continue;
        }
        let Some(shared) = find_http_param(&http_op.request_params, &param.declarator.0) else {
            continue;
        };
        let source = http_param_kind(shared.kind);
        let wire_name = shared.wire_name.clone();
        params.params.push(format!("{name}: {ty}"));
        params.param_names.push(name.clone());
        params.server_params.push(format!("{name}: {ty}"));
        params.server_param_names.push(name.clone());
        let serde_name = if matches!(source, ParamSource::Body) {
            field_rename_raw(&param.annotations).unwrap_or_else(|| wire_name.clone())
        } else {
            wire_name.clone()
        };
        let ctx = build_param_context(
            ParamBuildInput {
                param,
                name: &name,
                wire_name: &wire_name,
                path_template_name: path_param_template_name(path, source, &wire_name),
                inner_ty: &inner_ty,
                source,
                optional,
                flatten,
                serde_name: serde_rename(&serde_name, &name)
                    .or_else(|| field_rename(&param.annotations, &name)),
            },
            transport,
            env,
        )?;
        params.request_params.push(ctx.clone());
        match source {
            ParamSource::Path => params.path_params.push(ctx),
            ParamSource::Query => params.query_params.push(ctx),
            ParamSource::Header => params.header_params.push(ctx),
            ParamSource::Cookie => params.cookie_params.push(ctx),
            ParamSource::Body => params.body_params.push(ctx),
        }
    }
    Ok(())
}

fn maybe_add_response_param(
    param: &hir::ParamDcl,
    http_op: &HttpOperation,
    input: ParamBuildInput<'_>,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
    params: &mut MethodParams,
) -> IdlcResult<()> {
    let direction = param_direction(param.attr.as_ref());
    if !matches!(direction, ParamDirection::Out | ParamDirection::InOut) {
        return Ok(());
    }
    let Some(shared) = find_http_param(&http_op.response_params, &param.declarator.0) else {
        return Ok(());
    };
    let response_ctx = build_param_context(
        ParamBuildInput {
            wire_name: &shared.wire_name,
            path_template_name: String::new(),
            source: http_param_kind(shared.kind),
            flatten: false,
            serde_name: field_rename(&param.annotations, input.name)
                .or_else(|| serde_rename(&param.declarator.0, input.name)),
            ..input
        },
        transport,
        env,
    )?;
    match http_param_kind(shared.kind) {
        ParamSource::Header => params.response_header_params.push(response_ctx.clone()),
        ParamSource::Cookie => params.response_cookie_params.push(response_ctx.clone()),
        _ => params.response_body_params.push(response_ctx.clone()),
    }
    params.response_params.push(response_ctx);
    Ok(())
}

fn build_param_context(
    input: ParamBuildInput<'_>,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
) -> IdlcResult<ParamContext> {
    Ok(ParamContext {
        name: input.name.to_string(),
        raw_name: input.param.declarator.0.clone(),
        wire_name: input.wire_name.to_string(),
        path_template_name: input.path_template_name,
        ty: render_param_type(&input.param.ty, input.optional),
        in_ty: transport_param_type(
            &input.param.ty,
            input.optional,
            TransportDirection::In,
            transport,
            env,
        )?,
        out_ty: transport_param_type(
            &input.param.ty,
            input.optional,
            TransportDirection::Out,
            transport,
            env,
        )?,
        source: param_source_code(input.source),
        serde_rename: input.serde_name,
        header_is_multi: header_is_multi(&input.param.ty),
        header_item_ty: header_item_ty(&input.param.ty),
        header_item_is_string: header_item_is_string(&input.param.ty),
        header_item_is_primitive: header_item_is_primitive(&input.param.ty),
        cookie_is_multi: cookie_is_multi(&input.param.ty),
        cookie_item_ty: cookie_item_ty(&input.param.ty),
        cookie_item_is_string: cookie_item_is_string(&input.param.ty),
        cookie_item_is_primitive: cookie_item_is_primitive(&input.param.ty),
        optional: input.optional,
        inner_ty: input.inner_ty.to_string(),
        flatten: input.flatten,
        in_expr: decode_expr(input.name, &input.param.ty, env.registry)?,
        out_expr: encode_expr(input.name, &input.param.ty, env.registry)?,
        field_in_expr: decode_expr(
            &format!("value.{}", input.name),
            &input.param.ty,
            env.registry,
        )?,
        field_out_expr: encode_expr(
            &format!("value.{}", input.name),
            &input.param.ty,
            env.registry,
        )?,
    })
}
