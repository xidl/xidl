use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Errors raised while resolving imported compilation units.
#[derive(Debug)]
pub enum ImportError {
    /// An input file could not be read.
    Io {
        /// File the operation targeted.
        path: PathBuf,
        /// Underlying IO failure.
        source: std::io::Error,
    },
    /// An input could not be parsed while collecting its imports.
    Parse {
        /// File being scanned.
        path: PathBuf,
        /// Parser failure.
        reason: String,
    },
    /// An `import` named a scope instead of a quoted path.
    Unquoted {
        /// File holding the declaration.
        importer: PathBuf,
        /// Text of the named scope.
        scope: String,
    },
    /// An import path carries no usable name.
    InvalidPath {
        /// File holding the declaration.
        importer: PathBuf,
        /// Import path as written.
        import: String,
        /// Why the path was rejected.
        reason: String,
    },
    /// An import target could not be resolved to an IDL file.
    Unresolved {
        /// File holding the declaration.
        importer: PathBuf,
        /// Import path as written.
        import: String,
        /// Why the target was rejected.
        reason: String,
    },
    /// Two units claim one reference prefix.
    PrefixConflict {
        /// Prefix both units claim.
        prefix: String,
        /// Unit registered first.
        first: PathBuf,
        /// Unit that collided with it.
        second: PathBuf,
    },
    /// One file was imported under two different prefixes.
    UnitAliased {
        /// File imported twice.
        file: PathBuf,
        /// Prefix of the first import.
        first: String,
        /// Prefix of the second import.
        second: String,
    },
    /// The import graph contains a cycle.
    Cycle {
        /// Import chain, from the first unit back to itself.
        chain: Vec<PathBuf>,
    },
    /// A reference names a declaration the imported unit never declares.
    UnknownType {
        /// File holding the reference.
        importer: PathBuf,
        /// Unit the reference resolved to.
        unit: PathBuf,
        /// Reference as written.
        reference: String,
        /// Version the reference named, when it named one.
        version: Option<String>,
    },
    /// A reference names an API version the imported unit does not declare.
    UnknownVersion {
        /// File holding the reference.
        importer: PathBuf,
        /// Unit the reference resolved to.
        unit: PathBuf,
        /// Reference as written.
        reference: String,
        /// Version the reference named.
        version: String,
        /// Versions the unit declares.
        versions: Vec<String>,
    },
    /// A reference reaches a versioned unit without naming a version.
    MissingVersion {
        /// File holding the reference.
        importer: PathBuf,
        /// Unit the reference resolved to.
        unit: PathBuf,
        /// Reference as written.
        reference: String,
        /// Versions the unit declares.
        versions: Vec<String>,
    },
    /// A reference names a version on a unit without a version timeline.
    UnexpectedVersion {
        /// File holding the reference.
        importer: PathBuf,
        /// Unit the reference resolved to.
        unit: PathBuf,
        /// Reference as written.
        reference: String,
        /// Version the reference named.
        version: String,
    },
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "failed to read import unit {}: {source}", path.display())
            }
            Self::Parse { path, reason } => {
                write!(f, "failed to scan imports of {}: {reason}", path.display())
            }
            Self::Unquoted { importer, scope } => write!(
                f,
                "xidl import: {} imports `{scope}`; an import names a quoted path such as \
                 import \"pkg/index.idl\"",
                importer.display()
            ),
            Self::InvalidPath {
                importer,
                import,
                reason,
            } => write!(
                f,
                "xidl import: {} imports `{import}`, which is not a usable path: {reason}",
                importer.display()
            ),
            Self::Unresolved {
                importer,
                import,
                reason,
            } => write!(
                f,
                "xidl import: {} imports `{import}`, which does not resolve: {reason}",
                importer.display()
            ),
            Self::PrefixConflict {
                prefix,
                first,
                second,
            } => write!(
                f,
                "xidl import: `{prefix}` names both {} and {}; import one of them under a \
                 distinct name",
                first.display(),
                second.display()
            ),
            Self::UnitAliased {
                file,
                first,
                second,
            } => write!(
                f,
                "xidl import: {} is imported as both `{first}` and `{second}`; one unit keeps \
                 one reference prefix",
                file.display()
            ),
            Self::Cycle { chain } => {
                let chain = chain
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" -> ");
                write!(f, "xidl import: cyclic import detected: {chain}")
            }
            Self::UnknownType {
                importer,
                unit,
                reference,
                version,
            } => {
                let at = match version {
                    Some(version) => format!("`{version}`"),
                    None => "its only version".to_string(),
                };
                write!(
                    f,
                    "xidl import: {} references `{reference}`, which {} does not declare at {at}",
                    importer.display(),
                    unit.display()
                )
            }
            Self::UnknownVersion {
                importer,
                unit,
                reference,
                version,
                versions,
            } => write!(
                f,
                "xidl import: {} references `{reference}`, and `{version}` is not a version of {} \
                 (declared versions: {})",
                importer.display(),
                unit.display(),
                versions.join(", ")
            ),
            Self::MissingVersion {
                importer,
                unit,
                reference,
                versions,
            } => write!(
                f,
                "xidl import: {} references `{reference}`, and {} is versioned \
                 (declared versions: {}); name the version in the reference or bind it with an \
                 import that carries `@<version>`",
                importer.display(),
                unit.display(),
                versions.join(", ")
            ),
            Self::UnexpectedVersion {
                importer,
                unit,
                reference,
                version,
            } => write!(
                f,
                "xidl import: {} references `{reference}` with version `{version}`, and {} \
                 declares no version timeline",
                importer.display(),
                unit.display()
            ),
        }
    }
}

impl Error for ImportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { .. }
            | Self::Unquoted { .. }
            | Self::InvalidPath { .. }
            | Self::Unresolved { .. }
            | Self::PrefixConflict { .. }
            | Self::UnitAliased { .. }
            | Self::Cycle { .. }
            | Self::UnknownType { .. }
            | Self::UnknownVersion { .. }
            | Self::MissingVersion { .. }
            | Self::UnexpectedVersion { .. } => None,
        }
    }
}
