use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Errors raised while reading or validating a `xidl.toml` manifest.
#[derive(Debug)]
pub enum ManifestError {
    /// The manifest file could not be read.
    Read {
        /// Manifest that failed to load.
        path: PathBuf,
        /// Underlying IO failure.
        source: std::io::Error,
    },
    /// The manifest is not valid TOML for the manifest schema.
    Deserialize {
        /// Manifest that failed to parse.
        path: PathBuf,
        /// Underlying TOML failure, which already carries the offending span.
        source: toml::de::Error,
    },
    /// A field is present in the manifest but its value is not usable.
    InvalidField {
        /// Manifest that failed validation.
        path: PathBuf,
        /// Dotted field path, for example `dependencies.timestamp.src`.
        field: String,
        /// Why the value was rejected.
        reason: String,
    },
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(f, "failed to read manifest {}: {source}", path.display())
            }
            Self::Deserialize { path, source } => {
                write!(f, "failed to parse manifest {}: {source}", path.display())
            }
            Self::InvalidField {
                path,
                field,
                reason,
            } => write!(
                f,
                "invalid `{field}` in manifest {}: {reason}",
                path.display()
            ),
        }
    }
}

impl Error for ManifestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Deserialize { source, .. } => Some(source),
            Self::InvalidField { .. } => None,
        }
    }
}
