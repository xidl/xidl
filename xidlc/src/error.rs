#![allow(unused_assignments)]

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdlcError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(#[from] xidl_parser::error::ParseError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Template(String),
    #[error("{0}")]
    Rpc(String),
    #[error("{0}")]
    Fmt(String),
    #[error("{0}")]
    Diagnostics(#[from] DiagnosticListError),
}

#[derive(Debug, Error)]
#[error("{} diagnostics found", diagnostics.len())]
pub struct DiagnosticListError {
    pub diagnostics: Vec<DiagnosticError>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("{message}")]
#[diagnostic()]
pub struct DiagnosticError {
    // The Source that we're gonna be printing snippets out of.
    // This can be a String if you don't have or care about file names.
    pub message: String,
    #[source_code]
    pub src: NamedSource<String>,
    // Snippets and highlights can be included in the diagnostic!
    #[label("{label}")]
    pub bad_bit: SourceSpan,
    pub label: String,
}

impl DiagnosticError {
    pub fn from_label(filename: &str, source: &str, label: LabeledSpan) -> Self {
        let span: SourceSpan = (label.offset(), label.len()).into();
        let label_text = label.label().unwrap_or("error").to_string();
        Self {
            message: "Parse source error:".to_string(),
            src: NamedSource::new(filename, source.to_owned()).with_language("idl"),
            bad_bit: span,
            label: label_text,
        }
    }
}

pub type IdlcResult<T> = std::result::Result<T, IdlcError>;

impl IdlcError {
    pub fn template(message: impl Into<String>) -> Self {
        Self::Template(message.into())
    }

    pub fn rpc(message: impl Into<String>) -> Self {
        Self::Rpc(message.into())
    }

    pub fn fmt(message: impl Into<String>) -> Self {
        Self::Fmt(message.into())
    }

    pub fn diagnostic(err: DiagnosticError) -> Self {
        Self::Diagnostics(DiagnosticListError {
            diagnostics: vec![err],
        })
    }

    pub fn diagnostics(diagnostics: Vec<DiagnosticError>) -> Self {
        Self::Diagnostics(DiagnosticListError { diagnostics })
    }
}
