/// How a dependency pins the revision it wants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    /// An immutable commit hash.
    Rev(String),
    /// A release tag such as `v1.2.3`.
    Tag(String),
}

impl Revision {
    /// The revision text, without its kind.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Rev(rev) => rev,
            Self::Tag(tag) => tag,
        }
    }
}

/// Where a dependency library comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencySource {
    /// A Git package pinned to a tag or commit.
    Git {
        /// Repository or repository subdirectory source.
        src: String,
        /// Revision fetched from the repository.
        revision: Revision,
    },
    /// A local library resolved relative to the manifest directory.
    Path {
        /// Filesystem path as written in the manifest.
        path: String,
    },
}

/// One `[dependencies]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    source: DependencySource,
}

impl Dependency {
    /// Creates a Git dependency from an already validated source and revision.
    pub fn new(src: impl Into<String>, revision: Revision) -> Self {
        Self {
            source: DependencySource::Git {
                src: src.into(),
                revision,
            },
        }
    }

    /// Creates a local dependency from an already validated path.
    pub fn from_path(path: impl Into<String>) -> Self {
        Self {
            source: DependencySource::Path { path: path.into() },
        }
    }

    /// Declared dependency source.
    pub fn source(&self) -> &DependencySource {
        &self.source
    }

    /// Declared Git source, when this is a Git dependency.
    pub fn src(&self) -> Option<&str> {
        match &self.source {
            DependencySource::Git { src, .. } => Some(src),
            DependencySource::Path { .. } => None,
        }
    }

    /// Pinned Git revision, when this is a Git dependency.
    pub fn revision(&self) -> Option<&Revision> {
        match &self.source {
            DependencySource::Git { revision, .. } => Some(revision),
            DependencySource::Path { .. } => None,
        }
    }

    /// Declared local path, when this is a path dependency.
    pub fn path(&self) -> Option<&str> {
        match &self.source {
            DependencySource::Git { .. } => None,
            DependencySource::Path { path } => Some(path),
        }
    }

    /// Git source and revision, when this is a Git dependency.
    pub fn git(&self) -> Option<(&str, &Revision)> {
        match &self.source {
            DependencySource::Git { src, revision } => Some((src, revision)),
            DependencySource::Path { .. } => None,
        }
    }
}
