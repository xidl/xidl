use std::error::Error;
use std::fmt;

/// Errors raised while turning IDL source into a typed AST.
#[derive(Debug, Clone)]
pub enum ParseError {
    /// Free-form parse failure.
    Message(String),
    /// Source bytes that are not valid UTF-8.
    Utf8Error(std::str::Utf8Error),
    /// A tree-sitter node that did not match the expected grammar shape.
    UnexpectedNode(String),
    /// Failure reported by tree-sitter itself.
    TreeSitterError(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(message) => f.write_str(message),
            Self::Utf8Error(err) => write!(f, "{err}"),
            Self::UnexpectedNode(node) => write!(f, "Unexpected node: {node}"),
            Self::TreeSitterError(message) => write!(f, "TreeSitterError: {message}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Utf8Error(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

/// Result type for parser operations.
pub type ParserResult<T> = Result<T, ParseError>;
