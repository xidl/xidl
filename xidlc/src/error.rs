use std::error::Error;
use std::fmt;

#[cfg(feature = "cli")]
use miette::{Diagnostic, NamedSource, SourceSpan};

/// Result type for IDL compiler operations.
pub type IdlcResult<T> = std::result::Result<T, IdlcError>;

/// Errors produced by the IDL compiler driver.
#[derive(Debug)]
pub enum IdlcError {
    /// IO error.
    Io(std::io::Error),
    /// Parse error from the IDL parser.
    Parse(xidl_parser::error::ParseError),
    /// JSON serialization error.
    Json(serde_json::Error),
    /// Template rendering error.
    Template(String),
    /// RPC/validation error.
    Rpc(String),
    /// Formatting error.
    Fmt(String),
    /// Collection of diagnostics.
    Diagnostics(DiagnosticListError),
}

impl fmt::Display for IdlcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Parse(err) => write!(f, "{err}"),
            Self::Json(err) => write!(f, "{err}"),
            Self::Template(message) | Self::Rpc(message) | Self::Fmt(message) => {
                f.write_str(message)
            }
            Self::Diagnostics(err) => write!(f, "{err}"),
        }
    }
}

impl Error for IdlcError {
    // The wrapping variants stay transparent: they forward the wrapped error's own
    // source chain instead of reporting the wrapper as the source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => err.source(),
            Self::Parse(err) => err.source(),
            Self::Json(err) => err.source(),
            Self::Diagnostics(err) => Some(err),
            Self::Template(_) | Self::Rpc(_) | Self::Fmt(_) => None,
        }
    }
}

impl From<std::io::Error> for IdlcError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<xidl_parser::error::ParseError> for IdlcError {
    fn from(err: xidl_parser::error::ParseError) -> Self {
        Self::Parse(err)
    }
}

impl From<serde_json::Error> for IdlcError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<DiagnosticListError> for IdlcError {
    fn from(err: DiagnosticListError) -> Self {
        Self::Diagnostics(err)
    }
}

impl IdlcError {
    /// Create a template error.
    pub fn template(message: impl Into<String>) -> Self {
        Self::Template(message.into())
    }

    /// Create an RPC error.
    pub fn rpc(message: impl Into<String>) -> Self {
        Self::Rpc(message.into())
    }

    /// Create a formatting error.
    pub fn fmt(message: impl Into<String>) -> Self {
        Self::Fmt(message.into())
    }

    /// Create a single diagnostic error.
    pub fn diagnostic(err: DiagnosticError) -> Self {
        Self::Diagnostics(DiagnosticListError {
            diagnostics: vec![err],
        })
    }

    /// Create a diagnostics error from multiple diagnostics.
    pub fn diagnostics(diagnostics: Vec<DiagnosticError>) -> Self {
        Self::Diagnostics(DiagnosticListError { diagnostics })
    }
}

/// Collection of diagnostics.
#[derive(Debug)]
pub struct DiagnosticListError {
    /// Diagnostics in the collection.
    pub diagnostics: Vec<DiagnosticError>,
}

impl fmt::Display for DiagnosticListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} diagnostics found", self.diagnostics.len())
    }
}

impl Error for DiagnosticListError {}

/// Diagnostic emitted for a source span.
#[derive(Debug)]
pub struct DiagnosticError {
    /// Human readable message.
    pub message: String,
    /// File name the diagnostic originates from.
    pub filename: String,
    /// Source text.
    pub src: String,
    /// Label for the highlighted span.
    pub label: String,
    /// Byte offset of the diagnostic span.
    pub offset: usize,
    /// Byte length of the diagnostic span.
    pub len: usize,
    #[cfg(feature = "cli")]
    miette_source: NamedSource<String>,
    #[cfg(feature = "cli")]
    miette_span: SourceSpan,
}

impl fmt::Display for DiagnosticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for DiagnosticError {}

impl DiagnosticError {
    /// Create a diagnostic from a byte span.
    pub fn from_span(
        filename: &str,
        source: &str,
        offset: usize,
        len: usize,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        #[cfg(feature = "cli")]
        {
            let miette_source = NamedSource::new(filename, source.to_owned()).with_language("idl");
            let miette_span: SourceSpan = (offset, len).into();
            Self {
                message: "Parse source error:".to_string(),
                filename: filename.to_owned(),
                src: source.to_owned(),
                label: label.clone(),
                offset,
                len,
                miette_source,
                miette_span,
            }
        }
        #[cfg(not(feature = "cli"))]
        {
            Self {
                message: "Parse source error:".to_string(),
                filename: filename.to_owned(),
                src: source.to_owned(),
                label,
                offset,
                len,
            }
        }
    }

    /// Create a diagnostic from a `miette::LabeledSpan`.
    #[cfg(feature = "cli")]
    pub fn from_label(filename: &str, source: &str, label: miette::LabeledSpan) -> Self {
        let offset = label.offset();
        let len = label.len();
        let label_text = label.label().unwrap_or("error").to_string();
        Self::from_span(filename, source, offset, len, label_text)
    }
}

#[cfg(feature = "cli")]
impl Diagnostic for DiagnosticError {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.miette_source)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(miette::LabeledSpan::at(
            self.miette_span,
            self.label.clone(),
        ))))
    }
}
