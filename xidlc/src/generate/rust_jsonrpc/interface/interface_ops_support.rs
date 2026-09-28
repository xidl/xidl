use super::interface_model::StreamKind;

pub(super) fn stream_signature(
    kind: StreamKind,
    param_list: Vec<String>,
    unary_ret: String,
    req_item_ty: String,
) -> (Vec<String>, String) {
    match kind {
        StreamKind::Server => (
            param_list,
            format!("xidl_jsonrpc::stream::BoxStream<'a, {unary_ret}>"),
        ),
        StreamKind::Client => (
            vec!["stream: xidl_jsonrpc::stream::BoxStream<'a, serde_json::Value>".to_string()],
            unary_ret,
        ),
        StreamKind::Bidi => (
            vec![format!(
                "stream: xidl_jsonrpc::stream::BoxStream<'static, {req_item_ty}>"
            )],
            format!("xidl_jsonrpc::stream::BoxStream<'static, {unary_ret}>"),
        ),
    }
}
