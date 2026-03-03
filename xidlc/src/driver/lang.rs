use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Plugin {
    #[serde(alias = "hir")]
    Hir,
    #[serde(alias = "typed_ast", alias = "typed-ast")]
    TypedAst,
    #[serde(alias = "c", alias = "cc")]
    C,
    #[serde(alias = "cpp", alias = "c++", alias = "cxx")]
    Cpp,
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
    #[serde(alias = "openapi")]
    Openapi,
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
            ("typed_ast", Plugin::TypedAst),
            ("typed-ast", Plugin::TypedAst),
            ("c", Plugin::C),
            ("cc", Plugin::C),
            ("cpp", Plugin::Cpp),
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
            ("openapi", Plugin::Openapi),
            ("custom_plugin", Plugin::Custom("custom_plugin".into())),
        ];

        for case in cases {
            let (input, expected) = case;
            let plugin = Plugin::from(input);
            assert_eq!(plugin, expected);
        }
    }
}
