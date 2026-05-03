use crate::error::IdlcResult;
use crate::generate::rust::util::{rust_ident, rust_passthrough_attrs_from_annotations};
use crate::generate::rust_jsonrpc::{JsonRpcRenderOutput, JsonRpcRenderer};
use xidl_parser::jsonrpc_hir::{
    JsonRpcInterface, JsonRpcMethod, JsonRpcMethodKind, JsonRpcResponseKind, JsonRpcWatchMethod,
};

use super::interface_model::{
    MethodContext, OutputField, ParamField, StreamKind, WatchMethodContext,
};
use super::interface_names::{params_struct_name, response_struct_name};
use super::interface_ops_support::stream_signature;
use super::interface_types::jsonrpc_type;

pub(super) fn render_interface_def(
    interface: &JsonRpcInterface,
    renderer: &JsonRpcRenderer,
) -> IdlcResult<JsonRpcRenderOutput> {
    let methods = interface
        .methods
        .iter()
        .map(|method| render_method(method, &interface.ident))
        .collect::<Vec<_>>();
    let watch_methods = interface
        .watch_methods
        .iter()
        .map(render_watch_method)
        .collect::<Vec<_>>();
    let bidi_method_names = methods
        .iter()
        .filter(|method| matches!(method.kind.as_str(), "stream_op" | "stream_source"))
        .map(|method| method.rpc_name.clone())
        .collect::<Vec<_>>();
    let ctx = serde_json::json!({
        "ident": rust_ident(&interface.ident),
        "methods": methods,
        "bidi_method_names": bidi_method_names,
        "watch_methods": watch_methods,
        "rust_attrs": rust_passthrough_attrs_from_annotations(&interface.annotations),
    });
    Ok(JsonRpcRenderOutput {
        source: vec![renderer.render_template("interface.rs.j2", &ctx)?],
    })
}

fn render_method(method: &JsonRpcMethod, interface_name: &str) -> MethodContext {
    let method_name = rust_ident(&method.name);
    let request_fields = method
        .request_fields
        .iter()
        .map(|field| ParamField {
            name: rust_ident(&field.name),
            ty: jsonrpc_type(&field.ty),
        })
        .collect::<Vec<_>>();
    let request_params = request_fields
        .iter()
        .map(|field| format!("{}: {}", field.name, field.ty))
        .collect::<Vec<_>>();
    let args = request_fields
        .iter()
        .map(|field| field.name.clone())
        .collect::<Vec<_>>();
    let response_fields = method
        .response_fields
        .iter()
        .map(|field| OutputField {
            name: rust_ident(&field.name),
            json_name: field.wire_name.clone(),
            ty: jsonrpc_type(&field.ty),
        })
        .collect::<Vec<_>>();
    let response_kind = response_kind_name(method.response_kind).to_string();
    let unary_ret = response_return_type(
        method.response_kind,
        &response_fields,
        interface_name,
        &method_name,
    );
    let (kind, stream_mode, params, params_fields, params_struct, ret, args, stream_item_ty) =
        match method.kind {
            JsonRpcMethodKind::Unary => (
                "rpc".to_string(),
                String::new(),
                request_params,
                request_fields,
                params_struct_name(interface_name, &method_name),
                unary_ret,
                args,
                String::new(),
            ),
            JsonRpcMethodKind::ServerStream => {
                let (params, ret) = stream_signature(StreamKind::Server, request_params, unary_ret);
                (
                    "stream_op".to_string(),
                    "server".to_string(),
                    params,
                    request_fields,
                    params_struct_name(interface_name, &method_name),
                    ret,
                    args,
                    String::new(),
                )
            }
            JsonRpcMethodKind::ClientStream => {
                let (params, ret) = stream_signature(StreamKind::Client, request_params, unary_ret);
                (
                    "stream_op".to_string(),
                    "client".to_string(),
                    params,
                    Vec::new(),
                    String::new(),
                    ret,
                    Vec::new(),
                    String::new(),
                )
            }
            JsonRpcMethodKind::BidiStream => {
                let (params, ret) = stream_signature(StreamKind::Bidi, request_params, unary_ret);
                (
                    "stream_op".to_string(),
                    "bidi".to_string(),
                    params,
                    Vec::new(),
                    String::new(),
                    ret,
                    Vec::new(),
                    String::new(),
                )
            }
            JsonRpcMethodKind::StreamSource => (
                "stream_source".to_string(),
                String::new(),
                Vec::new(),
                Vec::new(),
                String::new(),
                String::new(),
                Vec::new(),
                method
                    .stream_item
                    .as_ref()
                    .map(jsonrpc_type)
                    .unwrap_or_else(|| "serde_json::Value".to_string()),
            ),
        };

    MethodContext {
        kind,
        stream_mode,
        name: method_name.clone(),
        rust_attrs: rust_passthrough_attrs_from_annotations(&method.annotations),
        params,
        params_fields,
        params_struct,
        ret,
        rpc_name: method.rpc_name.clone(),
        args,
        response_kind,
        response_struct: response_struct_name(interface_name, &method_name),
        response_fields,
        response_single_field: method
            .response_fields
            .first()
            .map(|field| rust_ident(&field.name))
            .unwrap_or_default(),
        stream_item_ty,
    }
}

fn render_watch_method(method: &JsonRpcWatchMethod) -> WatchMethodContext {
    WatchMethodContext {
        getter_name: rust_ident(&method.getter_name),
        item_ty: jsonrpc_type(&method.item_ty),
        stream_rpc_name: method.stream_rpc_name.clone(),
    }
}

fn response_kind_name(kind: JsonRpcResponseKind) -> &'static str {
    match kind {
        JsonRpcResponseKind::Empty => "empty",
        JsonRpcResponseKind::SingleReturn => "single_return",
        JsonRpcResponseKind::SingleOutput => "single_output",
        JsonRpcResponseKind::MultiOutput => "multi",
    }
}

fn response_return_type(
    kind: JsonRpcResponseKind,
    fields: &[OutputField],
    interface_name: &str,
    method_name: &str,
) -> String {
    match kind {
        JsonRpcResponseKind::Empty => "()".to_string(),
        JsonRpcResponseKind::SingleReturn | JsonRpcResponseKind::SingleOutput => {
            fields[0].ty.clone()
        }
        JsonRpcResponseKind::MultiOutput => response_struct_name(interface_name, method_name),
    }
}
