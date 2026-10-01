use std::error::Error as StdError;
use std::fmt;
use std::io;

/// Errors raised while generating artifacts from IDL inputs.
#[derive(Debug)]
pub enum Error {
    /// Filesystem or subprocess IO failure outside the compiler itself.
    Io(io::Error),
    /// The builder configuration cannot be executed by the selected backend.
    Config(String),
    /// The xidlc compiler rejected the inputs or failed while generating.
    Compile(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "{err}"),
            Self::Config(message) | Self::Compile(message) => f.write_str(message),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(err) => err.source(),
            Self::Config(_) | Self::Compile(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
