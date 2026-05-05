use serde_json::{Map, Value, json};
use xidl_parser::jsonrpc_hir::{
    JsonRpcField, JsonRpcMethod, JsonRpcMethodKind, JsonRpcMethodSource,
};

use super::annotations::{StreamKind, doc_text, has_annotation, stream_extension_direct};
use super::schema_types::schema_for_type;
pub(super) fn render_method(method: &JsonRpcMethod) -> Value {
    let params = method
        .request_fields
        .iter()
        .map(render_param)
        .collect::<Vec<_>>();
    let mut rendered = json!({
        "name": method.rpc_name,
        "params": params,
        "result": {
            "name": "result",
            "schema": result_object_schema(&method.response_fields),
        },
    });
    if let Some(description) = doc_text(&method.annotations) {
        rendered["description"] = Value::String(description);
    }
    if let Some(extension) =
        stream_extension(method.kind).or_else(|| attribute_stream_extension(method))
    {
        rendered["x-xidl-stream"] = extension;
    }
    rendered
}

fn render_param(field: &JsonRpcField) -> Value {
    let mut param = json!({
        "name": field.wire_name,
        "required": field.required,
        "schema": schema_for_type(&field.ty),
    });
    if let Some(description) = doc_text(&field.annotations) {
        param["description"] = Value::String(description);
    }
    param
}

pub(super) fn result_object_schema(fields: &[JsonRpcField]) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for field in fields {
        properties.insert(field.wire_name.clone(), schema_for_type(&field.ty));
        required.push(Value::String(field.wire_name.clone()));
    }

    let mut out = json!({
        "type": "object",
        "properties": properties,
    });
    if !required.is_empty() {
        out["required"] = Value::Array(required);
    }
    out
}

fn stream_extension(kind: JsonRpcMethodKind) -> Option<Value> {
    let stream_kind = match kind {
        JsonRpcMethodKind::Unary => return None,
        JsonRpcMethodKind::ServerStream | JsonRpcMethodKind::StreamSource => StreamKind::Server,
        JsonRpcMethodKind::ClientStream => StreamKind::Client,
        JsonRpcMethodKind::BidiStream => StreamKind::Bidi,
    };
    Some(stream_extension_direct(stream_kind))
}

fn attribute_stream_extension(method: &JsonRpcMethod) -> Option<Value> {
    if matches!(method.source, JsonRpcMethodSource::AttributeGet)
        && has_annotation(&method.annotations, "server_stream")
    {
        return Some(stream_extension_direct(StreamKind::Server));
    }
    None
}
