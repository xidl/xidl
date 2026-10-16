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

/// One `[dependencies]` entry: where a library comes from and its revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    src: String,
    revision: Revision,
}

impl Dependency {
    /// Creates a dependency from an already validated source and revision.
    pub fn new(src: impl Into<String>, revision: Revision) -> Self {
        Self {
            src: src.into(),
            revision,
        }
    }

    /// Declared source, for example `github.com/xidl/xidl/xidl-std`.
    pub fn src(&self) -> &str {
        &self.src
    }

    /// Revision the dependency is pinned to.
    pub fn revision(&self) -> &Revision {
        &self.revision
    }
}
