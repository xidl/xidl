use serde_json::json;

pub(super) fn get_test_cases() -> Vec<(&'static str, &'static str, serde_json::Value)> {
    vec![
        ("ipc", include_str!("../../../jsonrpc/ipc.idl"), json!({})),
        //
    ]
}
