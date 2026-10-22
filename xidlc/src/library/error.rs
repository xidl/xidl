use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Errors raised while resolving `xidl.toml` libraries.
#[derive(Debug)]
pub enum LibraryError {
    /// A manifest could not be read or validated.
    Manifest(xidl_parser::manifest::ManifestError),
    /// A file or directory could not be read or written.
    Io {
        /// Path the operation targeted.
        path: PathBuf,
        /// Underlying IO failure.
        source: std::io::Error,
    },
    /// A dependency `src` could not be interpreted.
    Source {
        /// Dependency name.
        name: String,
        /// Declared source.
        src: String,
        /// Why the source was rejected.
        reason: String,
    },
    /// A dependency package could not be materialized.
    Fetch {
        /// Dependency name.
        name: String,
        /// Declared source.
        src: String,
        /// Why the fetch failed.
        reason: String,
    },
    /// A local dependency path could not be used as a mount.
    PathDependency {
        /// Dependency name.
        name: String,
        /// Resolved path named by the dependency.
        path: PathBuf,
        /// Why the path was rejected.
        reason: String,
    },
    /// An external command required for resolution failed.
    Command {
        /// Program that was executed.
        program: String,
        /// Arguments passed to the program.
        args: Vec<String>,
        /// Why the command failed.
        reason: String,
    },
    /// The required compiler could not be used or installed.
    Toolchain {
        /// Version range the manifest asked for.
        requirement: String,
        /// Why the requirement could not be satisfied.
        reason: String,
    },
    /// No cache home could be determined for downloaded artifacts.
    CacheRoot,
    /// A library resolved to no input files.
    NoInputs {
        /// Directory the inputs were searched in.
        dir: PathBuf,
        /// Patterns that were searched.
        patterns: Vec<String>,
    },
}

impl fmt::Display for LibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest(err) => write!(f, "{err}"),
            Self::Io { path, source } => write!(f, "failed to access {}: {source}", path.display()),
            Self::Source { name, src, reason } => {
                write!(
                    f,
                    "invalid source `{src}` for dependency `{name}`: {reason}"
                )
            }
            Self::Fetch { name, src, reason } => {
                write!(
                    f,
                    "failed to fetch dependency `{name}` from `{src}`: {reason}"
                )
            }
            Self::PathDependency { name, path, reason } => write!(
                f,
                "invalid path dependency `{name}` at {}: {reason}",
                path.display()
            ),
            Self::Command {
                program,
                args,
                reason,
            } => write!(f, "`{program} {}` failed: {reason}", args.join(" ")),
            Self::Toolchain {
                requirement,
                reason,
            } => write!(
                f,
                "the xidlc compiler does not satisfy `{requirement}` and could not be installed: {reason}"
            ),
            Self::CacheRoot => write!(
                f,
                "cannot locate the xidl cache: set XIDL_HOME or HOME so \
                 downloaded packages and toolchains have a home"
            ),
            Self::NoInputs { dir, patterns } => write!(
                f,
                "no IDL inputs found in {} for patterns [{}]",
                dir.display(),
                patterns.join(", ")
            ),
        }
    }
}

impl Error for LibraryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Manifest(err) => Some(err),
            Self::Io { source, .. } => Some(source),
            Self::Source { .. }
            | Self::Fetch { .. }
            | Self::PathDependency { .. }
            | Self::Command { .. }
            | Self::Toolchain { .. }
            | Self::CacheRoot
            | Self::NoInputs { .. } => None,
        }
    }
}

impl From<xidl_parser::manifest::ManifestError> for LibraryError {
    fn from(err: xidl_parser::manifest::ManifestError) -> Self {
        Self::Manifest(err)
    }
}
