use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Plugin {
    #[serde(alias = "hir")]
    Hir,
    #[serde(alias = "rest_hir", alias = "rest-hir")]
    RestHir,
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
    #[serde(
        alias = "ts_rest",
        alias = "ts-rest",
        alias = "typescript_rest",
        alias = "typescript-rest"
    )]
    TypescriptRest,
    #[serde(alias = "go", alias = "golang")]
    Go,
    #[serde(alias = "go_rest", alias = "go-rest")]
    GoRest,
    #[serde(alias = "py", alias = "python")]
    Python,
    #[serde(
        alias = "py_rest",
        alias = "py-rest",
        alias = "python_rest",
        alias = "python-rest"
    )]
    PythonRest,
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
            ("rest_hir", Plugin::RestHir),
            ("rest-hir", Plugin::RestHir),
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
            ("py", Plugin::Python),
            ("python", Plugin::Python),
            ("py_rest", Plugin::PythonRest),
            ("py-rest", Plugin::PythonRest),
            ("python_rest", Plugin::PythonRest),
            ("python-rest", Plugin::PythonRest),
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
