use super::interface_model::{OutputField, StreamKind};
use super::interface_names::response_struct_name;

pub(super) fn response_kind(fields: &[OutputField]) -> String {
    if fields.is_empty() {
        "empty".to_string()
    } else if fields.len() == 1 && fields[0].json_name == "return" {
        "single_return".to_string()
    } else if fields.len() == 1 {
        "single_output".to_string()
    } else {
        "multi".to_string()
    }
}

pub(super) fn response_return_type(
    kind: &str,
    fields: &[OutputField],
    interface_name: &str,
    method_name: &str,
) -> String {
    match kind {
        "empty" => "()".to_string(),
        "single_return" | "single_output" => fields[0].ty.clone(),
        _ => response_struct_name(interface_name, method_name),
    }
}

pub(super) fn stream_signature(
    kind: StreamKind,
    param_list: Vec<String>,
    unary_ret: String,
) -> (Vec<String>, String) {
    match kind {
        StreamKind::Server => (
            param_list,
            "xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>".to_string(),
        ),
        StreamKind::Client => (
            vec!["stream: xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>".to_string()],
            unary_ret,
        ),
        StreamKind::Bidi => (
            vec![
                "stream: xidl_jsonrpc::stream::ReaderWriter<serde_json::Value, serde_json::Value>"
                    .to_string(),
            ],
            "()".to_string(),
        ),
    }
}
