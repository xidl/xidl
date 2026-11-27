use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Plugin {
    #[serde(alias = "hir")]
    Hir,
    #[serde(alias = "rest_hir", alias = "rest-hir")]
    RestHir,
    #[serde(alias = "jsonrpc_hir", alias = "jsonrpc-hir")]
    JsonRpcHir,
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
    /// Separate opt-in generation of Rust Axum version adapters.
    #[serde(alias = "rust-axum-adapters")]
    RustAxumAdapters,
    #[serde(alias = "ts", alias = "typescript")]
    Typescript,
    #[serde(
        alias = "ts_rest",
        alias = "ts-rest",
        alias = "typescript_rest",
        alias = "typescript-rest"
    )]
    TypescriptRest,
    /// Separate opt-in generation of TypeScript REST version adapters.
    #[serde(alias = "typescript-rest-adapters")]
    TypescriptRestAdapters,
    #[serde(alias = "go", alias = "golang")]
    Go,
    #[serde(alias = "go_rest", alias = "go-rest")]
    GoRest,
    #[serde(alias = "openapi")]
    Openapi,
    #[serde(alias = "openrpc", alias = "open-rpc")]
    Openrpc,
    #[serde(alias = "version_map", alias = "version-map")]
    VersionMap,
    Custom(String),
}

impl From<&str> for Plugin {
    fn from(s: &str) -> Self {
        serde_json::from_str(&format!("\"{s}\"")).unwrap_or_else(|_| Self::Custom(s.to_string()))
    }
}

impl Plugin {
    /// Canonical language key used to share one session across aliases.
    pub(crate) fn cache_key(&self) -> String {
        match self {
            Self::Hir => "hir",
            Self::RestHir => "rest_hir",
            Self::JsonRpcHir => "jsonrpc_hir",
            Self::TypedAst => "typed_ast",
            Self::Rust => "rust",
            Self::RustJsonRpc => "rust-jsonrpc",
            Self::Axum => "axum",
            Self::RustAxumAdapters => "rust-axum-adapters",
            Self::Typescript => "typescript",
            Self::TypescriptRest => "typescript-rest",
            Self::TypescriptRestAdapters => "typescript-rest-adapters",
            Self::Go => "go",
            Self::GoRest => "go-rest",
            Self::Openapi => "openapi",
            Self::Openrpc => "openrpc",
            Self::VersionMap => "version_map",
            Self::Custom(lang) => lang,
        }
        .to_owned()
    }
}

#[cfg(test)]
mod tests;
