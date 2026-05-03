use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Plugin {
    #[serde(alias = "hir")]
    Hir,
    #[serde(alias = "http_hir", alias = "http-hir")]
    HttpHir,
    #[serde(alias = "typed_ast", alias = "typed-ast")]
    TypedAst,
    #[serde(alias = "rs", alias = "rust")]
    Rust,
    #[serde(
        alias = "rust_jsonrpc",
        alias = "rust-jsonrpc",
        alias = "rs_jsonrpc",
        alias = "rs-jsonrpc"
    )]
    RustJsonRpc,
    #[serde(
        alias = "axum",
        alias = "rust_axum",
        alias = "rust-axum",
        alias = "rs_axum",
        alias = "rs-axum"
    )]
    Axum,
    #[serde(alias = "ts", alias = "typescript")]
    Typescript,
    #[serde(alias = "go", alias = "golang")]
    Go,
    #[serde(alias = "go_http", alias = "go-http")]
    GoHttp,
    #[serde(alias = "py", alias = "python")]
    Python,
    #[serde(
        alias = "py_http",
        alias = "py-http",
        alias = "python_http",
        alias = "python-http"
    )]
    PythonHttp,
    #[serde(alias = "openapi")]
    Openapi,
    #[serde(alias = "openrpc", alias = "open-rpc")]
    Openrpc,
    Custom(String),
}

impl From<&str> for Plugin {
    fn from(s: &str) -> Self {
        serde_json::from_str(&format!("\"{s}\"")).unwrap_or_else(|_| Self::Custom(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_from_str() {
        let cases = [
            ("hir", Plugin::Hir),
            ("http_hir", Plugin::HttpHir),
            ("http-hir", Plugin::HttpHir),
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
            ("go", Plugin::Go),
            ("golang", Plugin::Go),
            ("go_http", Plugin::GoHttp),
            ("go-http", Plugin::GoHttp),
            ("py", Plugin::Python),
            ("python", Plugin::Python),
            ("py_http", Plugin::PythonHttp),
            ("py-http", Plugin::PythonHttp),
            ("python_http", Plugin::PythonHttp),
            ("python-http", Plugin::PythonHttp),
            ("openapi", Plugin::Openapi),
            ("openrpc", Plugin::Openrpc),
            ("open-rpc", Plugin::Openrpc),
            ("custom_plugin", Plugin::Custom("custom_plugin".into())),
        ];

        for case in cases {
            let (input, expected) = case;
            let plugin = Plugin::from(input);
            assert_eq!(plugin, expected);
        }
    }
}
