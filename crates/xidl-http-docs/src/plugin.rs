use crate::config::DocumentationConfig;
use crate::error::{DocumentationError, Result};
use crate::site::Site;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xidl_parser::hir::ParserProperties;
use xidlc::jsonrpc::{Artifact, Codegen, CodegenInput, RpcError};

/// External stdio generator assembling one complete HTTP reference site.
pub struct DocumentationPlugin {
    config: DocumentationConfig,
}

impl DocumentationPlugin {
    /// Constructs a renderer with explicit publication configuration.
    pub fn new(config: DocumentationConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Loads `XIDL_HTTP_DOCS_CONFIG`, or `xidl-http-docs.toml` when present.
    pub fn from_environment() -> Result<Self> {
        let explicit = std::env::var_os("XIDL_HTTP_DOCS_CONFIG").map(PathBuf::from);
        let default = Path::new("xidl-http-docs.toml");
        let path = explicit
            .as_deref()
            .or_else(|| default.exists().then_some(default));
        Self::new(DocumentationConfig::load(path)?)
    }

    fn render(&self, input: CodegenInput) -> Result<Vec<Artifact>> {
        match input {
            CodegenInput::Compilation(compilation) => Site::build(*compilation, &self.config),
            _ => Err(DocumentationError::Contract("requires compilation-scoped semantic input; upgrade xidlc to a compiler with compilation generation support".into())),
        }
    }
}

impl Codegen for DocumentationPlugin {
    fn get_engine_version(&self) -> std::result::Result<String, RpcError> {
        Ok(format!("={}", env!("CARGO_PKG_VERSION")))
    }

    fn get_properties(&self) -> std::result::Result<ParserProperties, RpcError> {
        Ok(HashMap::from([
            ("generation_scope".into(), "compilation".into()),
            ("hir_kind".into(), "http".into()),
            ("version_layout".into(), "per_version".into()),
        ]))
    }

    fn generate(
        &self,
        input: CodegenInput,
        _path: String,
        _props: ParserProperties,
    ) -> std::result::Result<Vec<Artifact>, RpcError> {
        self.render(input)
            .map_err(|error| RpcError::new(error.to_string()))
    }
}
