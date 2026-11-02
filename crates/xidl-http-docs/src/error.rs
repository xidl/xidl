use thiserror::Error;

/// Failures of documentation input, publication or rendering contracts.
#[derive(Debug, Error)]
pub enum DocumentationError {
    /// An invalid compiler input or publication contract.
    #[error("xidl-http-docs: {0}")]
    Contract(String),
    /// An unreadable configuration or release note.
    #[error("xidl-http-docs file access: {0}")]
    Io(#[from] std::io::Error),
    /// A malformed semantic JSON value.
    #[error("xidl-http-docs semantic input: {0}")]
    Json(#[from] serde_json::Error),
    /// A malformed TOML configuration or note header.
    #[error("xidl-http-docs configuration: {0}")]
    Config(#[from] toml::de::Error),
    /// A failed template render.
    #[error("xidl-http-docs template: {0}")]
    Render(#[from] minijinja::Error),
    /// A failed Markdown serialization for Starlight content.
    #[error("xidl-http-docs Markdown: {0}")]
    Markdown(#[from] pulldown_cmark_to_cmark::Error),
}

pub(crate) type Result<T> = std::result::Result<T, DocumentationError>;
