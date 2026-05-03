use super::interface_model::StreamKind;

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
