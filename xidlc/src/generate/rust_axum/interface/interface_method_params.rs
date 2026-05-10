use crate::error::IdlcResult;
use crate::generate::rust::util::rust_ident;
use crate::generate::rust_axum::interface::interface_annotations::serde_rename;
use crate::generate::rust_axum::interface::interface_method_support::{
    param_source_code, path_param_template_name, transport_param_type,
};
use crate::generate::rust_axum::interface::interface_types::{
    axum_type, cookie_is_multi, cookie_item_is_primitive, cookie_item_is_string, cookie_item_ty,
    header_is_multi, header_item_is_primitive, header_item_is_string, header_item_ty,
    render_param_type,
};
use crate::generate::rust_axum::interface::{ParamContext, ParamSource, RenderEnv};
use crate::generate::rust_axum::transport::{
    TransportDirection, TransportTracker, decode_expr, encode_expr,
};
use xidl_parser::rest_hir::{
    HttpOperation, HttpOutputSource, HttpRequestBodyShape, HttpSignatureParam,
};

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
    sig_param: &'a HttpSignatureParam,
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
    _op: &xidl_parser::hir::OpDcl,
    http_op: &HttpOperation,
    path: &str,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
    params: &mut MethodParams,
) -> IdlcResult<()> {
    for p in &http_op.signature.params {
        let name = rust_ident(&p.name);
        let optional = p.is_optional;
        let flatten = p.is_flatten;
        let inner_ty = axum_type(&p.ty);
        let ty = render_param_type(&p.ty, optional);

        // Input
        if matches!(
            p.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::In
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            let (source, wire_name) = find_input_binding(http_op, &p.name, flatten);
            let source_enum = match source {
                xidl_parser::rest_hir::HttpParamKind::Path => ParamSource::Path,
                xidl_parser::rest_hir::HttpParamKind::Query => ParamSource::Query,
                xidl_parser::rest_hir::HttpParamKind::Header => ParamSource::Header,
                xidl_parser::rest_hir::HttpParamKind::Cookie => ParamSource::Cookie,
                xidl_parser::rest_hir::HttpParamKind::Body => ParamSource::Body,
            };

            params.params.push(format!("{name}: {ty}"));
            params.param_names.push(name.clone());
            params.server_params.push(format!("{name}: {ty}"));
            params.server_param_names.push(name.clone());

            let serde_name = if matches!(source_enum, ParamSource::Body) {
                // In Axum we need the wire name for serde if it's in a struct
                if wire_name.is_empty() {
                    None
                } else {
                    Some(wire_name.clone())
                }
            } else {
                Some(wire_name.clone())
            };

            let ctx = build_param_context(
                ParamBuildInput {
                    sig_param: p,
                    name: &name,
                    wire_name: &wire_name,
                    path_template_name: path_param_template_name(path, source_enum, &wire_name),
                    inner_ty: &inner_ty,
                    source: source_enum,
                    optional,
                    flatten,
                    serde_name: serde_name.and_then(|sn| serde_rename(&sn, &name)),
                },
                transport,
                env,
            )?;
            params.request_params.push(ctx.clone());
            match source_enum {
                ParamSource::Path => params.path_params.push(ctx),
                ParamSource::Query => params.query_params.push(ctx),
                ParamSource::Header => params.header_params.push(ctx),
                ParamSource::Cookie => params.cookie_params.push(ctx),
                ParamSource::Body => params.body_params.push(ctx),
            }
        }

        // Output
        if matches!(
            p.direction,
            xidl_parser::rest_hir::HttpSignatureParamDirection::Out
                | xidl_parser::rest_hir::HttpSignatureParamDirection::InOut
        ) {
            let (source, wire_name) = find_output_binding(http_op, &p.name);
            let source_enum = match source {
                xidl_parser::rest_hir::HttpParamKind::Header => ParamSource::Header,
                xidl_parser::rest_hir::HttpParamKind::Cookie => ParamSource::Cookie,
                _ => ParamSource::Body,
            };

            let response_ctx = build_param_context(
                ParamBuildInput {
                    sig_param: p,
                    name: &name,
                    wire_name: &wire_name,
                    path_template_name: String::new(),
                    inner_ty: &inner_ty,
                    source: source_enum,
                    optional,
                    flatten: false,
                    serde_name: serde_rename(&p.name, &name),
                },
                transport,
                env,
            )?;

            match source_enum {
                ParamSource::Header => params.response_header_params.push(response_ctx.clone()),
                ParamSource::Cookie => params.response_cookie_params.push(response_ctx.clone()),
                _ => params.response_body_params.push(response_ctx.clone()),
            }
            params.response_params.push(response_ctx);
        }
    }
    Ok(())
}

pub(super) fn find_input_binding(
    op: &HttpOperation,
    name: &str,
    flatten: bool,
) -> (xidl_parser::rest_hir::HttpParamKind, String) {
    if let Some(b) = op.http.request.path.iter().find(|b| b.source_param == name) {
        return (
            xidl_parser::rest_hir::HttpParamKind::Path,
            b.wire_name.clone(),
        );
    }
    if let Some(b) = op
        .http
        .request
        .query
        .iter()
        .find(|b| b.source_param == name)
    {
        return (
            xidl_parser::rest_hir::HttpParamKind::Query,
            b.wire_name.clone(),
        );
    }
    if let Some(b) = op
        .http
        .request
        .header
        .iter()
        .find(|b| b.source_param == name)
    {
        return (
            xidl_parser::rest_hir::HttpParamKind::Header,
            b.wire_name.clone(),
        );
    }
    if let Some(b) = op
        .http
        .request
        .cookie
        .iter()
        .find(|b| b.source_param == name)
    {
        return (
            xidl_parser::rest_hir::HttpParamKind::Cookie,
            b.wire_name.clone(),
        );
    }

    match &op.http.request.body.shape {
        HttpRequestBodyShape::SingleValue {
            source_param,
            flatten: f,
            ..
        } if source_param == name => (
            xidl_parser::rest_hir::HttpParamKind::Body,
            if *f { "".to_string() } else { name.to_string() },
        ),
        HttpRequestBodyShape::Object { fields } => {
            if let Some(f) = fields.iter().find(|f| f.source_param == name) {
                (
                    xidl_parser::rest_hir::HttpParamKind::Body,
                    f.field_name.clone(),
                )
            } else {
                (xidl_parser::rest_hir::HttpParamKind::Body, name.to_string())
            }
        }
        _ => (
            xidl_parser::rest_hir::HttpParamKind::Body,
            if flatten {
                "".to_string()
            } else {
                name.to_string()
            },
        ),
    }
}

pub(super) fn find_output_binding(
    op: &HttpOperation,
    name: &str,
) -> (xidl_parser::rest_hir::HttpParamKind, String) {
    if let Some(b) = op.http.response.header.iter().find(|b| match &b.source {
        HttpOutputSource::Param { name: n } => n == name,
        _ => false,
    }) {
        return (
            xidl_parser::rest_hir::HttpParamKind::Header,
            b.wire_name.clone(),
        );
    }
    if let Some(b) = op.http.response.cookie.iter().find(|b| match &b.source {
        HttpOutputSource::Param { name: n } => n == name,
        _ => false,
    }) {
        return (
            xidl_parser::rest_hir::HttpParamKind::Cookie,
            b.wire_name.clone(),
        );
    }
    (xidl_parser::rest_hir::HttpParamKind::Body, name.to_string())
}

fn build_param_context(
    input: ParamBuildInput<'_>,
    transport: &mut TransportTracker,
    env: RenderEnv<'_>,
) -> IdlcResult<ParamContext> {
    Ok(ParamContext {
        name: input.name.to_string(),
        raw_name: input.sig_param.name.clone(),
        wire_name: input.wire_name.to_string(),
        path_template_name: input.path_template_name,
        ty: render_param_type(&input.sig_param.ty, input.optional),
        in_ty: transport_param_type(
            &input.sig_param.ty,
            input.optional,
            TransportDirection::In,
            transport,
            env,
        )?,
        out_ty: transport_param_type(
            &input.sig_param.ty,
            input.optional,
            TransportDirection::Out,
            transport,
            env,
        )?,
        source: param_source_code(input.source),
        serde_rename: input.serde_name,
        header_is_multi: header_is_multi(&input.sig_param.ty),
        header_item_ty: header_item_ty(&input.sig_param.ty),
        header_item_is_string: header_item_is_string(&input.sig_param.ty),
        header_item_is_primitive: header_item_is_primitive(&input.sig_param.ty),
        cookie_is_multi: cookie_is_multi(&input.sig_param.ty),
        cookie_item_ty: cookie_item_ty(&input.sig_param.ty),
        cookie_item_is_string: cookie_item_is_string(&input.sig_param.ty),
        cookie_item_is_primitive: cookie_item_is_primitive(&input.sig_param.ty),
        optional: input.optional,
        inner_ty: input.inner_ty.to_string(),
        flatten: input.flatten,
        in_expr: decode_expr(input.name, &input.sig_param.ty, env.registry)?,
        out_expr: encode_expr(input.name, &input.sig_param.ty, env.registry)?,
        field_in_expr: decode_expr(
            &format!("value.{}", input.name),
            &input.sig_param.ty,
            env.registry,
        )?,
        field_out_expr: encode_expr(
            &format!("value.{}", input.name),
            &input.sig_param.ty,
            env.registry,
        )?,
    })
}
