use super::*;

#[test]
fn test_plugin_from_str() {
    let cases = [
        ("hir", Plugin::Hir),
        ("rest_hir", Plugin::RestHir),
        ("rest-hir", Plugin::RestHir),
        ("jsonrpc_hir", Plugin::JsonRpcHir),
        ("jsonrpc-hir", Plugin::JsonRpcHir),
        ("typed_ast", Plugin::TypedAst),
        ("typed-ast", Plugin::TypedAst),
        ("rs", Plugin::Rust),
        ("rust_jsonrpc", Plugin::RustJsonRpc),
        ("rust-jsonrpc", Plugin::RustJsonRpc),
        ("rs_jsonrpc", Plugin::RustJsonRpc),
        ("rs-jsonrpc", Plugin::RustJsonRpc),
        ("axum", Plugin::Axum),
        ("rust_axum", Plugin::Axum),
        ("rust-axum", Plugin::Axum),
        ("rs_axum", Plugin::Axum),
        ("rs-axum", Plugin::Axum),
        ("ts", Plugin::Typescript),
        ("typescript", Plugin::Typescript),
        ("ts_rest", Plugin::TypescriptRest),
        ("ts-rest", Plugin::TypescriptRest),
        ("typescript_rest", Plugin::TypescriptRest),
        ("typescript-rest", Plugin::TypescriptRest),
        ("go", Plugin::Go),
        ("golang", Plugin::Go),
        ("go_rest", Plugin::GoRest),
        ("go-rest", Plugin::GoRest),
        ("openapi", Plugin::Openapi),
        ("openrpc", Plugin::Openrpc),
        ("open-rpc", Plugin::Openrpc),
        ("version-map", Plugin::VersionMap),
        ("custom_plugin", Plugin::Custom("custom_plugin".into())),
    ];

    for case in cases {
        let (input, expected) = case;
        let plugin = Plugin::from(input);
        assert_eq!(plugin, expected);
    }
}
