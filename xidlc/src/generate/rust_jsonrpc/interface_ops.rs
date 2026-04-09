use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use xidl_parser::hir;

use super::interface_annotations::{param_mode, stream_kind_from_annotations, stream_mode_name};
use super::interface_model::{MethodContext, OutputField, ParamField, ParamMode, StreamKind};
use super::interface_names::{params_struct_name, response_struct_name, rpc_method_name};
use super::interface_ops_support::{response_kind, response_return_type, stream_signature};
use super::interface_types::{jsonrpc_type, render_param_type};

pub(super) fn render_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
) -> IdlcResult<Vec<MethodContext>> {
    let stream_kind = stream_kind_from_annotations(&op.annotations)?;
    if let Some(kind) = stream_kind {
        return Ok(vec![render_stream_op(
            op,
            interface_name,
            module_path,
            kind,
        )]);
    }
    Ok(vec![render_unary_op(op, interface_name, module_path)])
}

fn render_unary_op(op: &hir::OpDcl, interface_name: &str, module_path: &[String]) -> MethodContext {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let (param_list, param_fields, args, response_fields) = unary_parts(op, params);
    let method_name = rust_ident(&op.ident);
    let response_kind = response_kind(&response_fields);
    let ret = response_return_type(
        &response_kind,
        &response_fields,
        interface_name,
        &method_name,
    );

    MethodContext {
        kind: "rpc".to_string(),
        stream_mode: String::new(),
        name: method_name.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&op.annotations),
        params: param_list,
        params_fields: param_fields,
        params_struct: params_struct_name(interface_name, &method_name),
        ret,
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        args,
        response_kind,
        response_struct: response_struct_name(interface_name, &method_name),
        response_single_field: response_fields
            .first()
            .map(|value| value.name.clone())
            .unwrap_or_default(),
        response_fields,
        stream_item_ty: String::new(),
    }
}

fn render_stream_op(
    op: &hir::OpDcl,
    interface_name: &str,
    module_path: &[String],
    kind: StreamKind,
) -> MethodContext {
    let params = op
        .parameter
        .as_ref()
        .map(|value| value.0.as_slice())
        .unwrap_or(&[]);
    let (param_list, param_fields, args, response_fields) = unary_parts(op, params);
    let method_name = rust_ident(&op.ident);
    let response_kind = response_kind(&response_fields);
    let unary_ret =
        response_return_type(&response_kind, &response_fields, interface_name, &op.ident);
    let (params, ret) = stream_signature(kind, param_list, unary_ret);

    MethodContext {
        kind: "stream_op".to_string(),
        stream_mode: stream_mode_name(kind).to_string(),
        name: method_name.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&op.annotations),
        params,
        params_fields: if matches!(kind, StreamKind::Server) {
            param_fields
        } else {
            Vec::new()
        },
        params_struct: if matches!(kind, StreamKind::Server) {
            params_struct_name(interface_name, &method_name)
        } else {
            String::new()
        },
        ret,
        rpc_name: rpc_method_name(module_path, interface_name, &op.ident),
        args: if matches!(kind, StreamKind::Server) {
            args
        } else {
            Vec::new()
        },
        response_kind,
        response_struct: response_struct_name(interface_name, &method_name),
        response_single_field: response_fields
            .first()
            .map(|value| value.name.clone())
            .unwrap_or_default(),
        response_fields,
        stream_item_ty: String::new(),
    }
}

fn unary_parts(
    op: &hir::OpDcl,
    params: &[hir::ParamDcl],
) -> (Vec<String>, Vec<ParamField>, Vec<String>, Vec<OutputField>) {
    let mut param_list = Vec::new();
    let mut param_fields = Vec::new();
    let mut args = Vec::new();
    let mut response_fields = return_fields(op);

    for param in params {
        let mode = param_mode(param.attr.as_ref());
        let ty = render_param_type(&param.ty, param.attr.as_ref());
        let raw_name = param.declarator.0.as_str();
        let name = rust_ident(raw_name);
        if matches!(mode, ParamMode::In | ParamMode::InOut) {
            param_list.push(format!("{name}: {ty}"));
            param_fields.push(ParamField {
                name: name.clone(),
                ty: ty.clone(),
            });
            args.push(name);
        }
        if matches!(mode, ParamMode::Out | ParamMode::InOut) {
            response_fields.push(OutputField {
                name: rust_ident(raw_name),
                json_name: raw_name.to_string(),
                ty,
            });
        }
    }
    (param_list, param_fields, args, response_fields)
}

fn return_fields(op: &hir::OpDcl) -> Vec<OutputField> {
    let mut response_fields = Vec::new();
    if let hir::OpTypeSpec::TypeSpec(ty) = &op.ty {
        response_fields.push(OutputField {
            name: rust_ident("return"),
            json_name: "return".to_string(),
            ty: jsonrpc_type(ty),
        });
    }
    response_fields
}
