use super::error::LibraryError;

/// Host segment used for sources that are plain filesystem paths.
pub const LOCAL_HOST: &str = "local";

/// A dependency `src` split into host, repository and optional subdirectory.
///
/// Accepted shapes are `github.com/xidl/xidl/xidl-std`, the equivalent
/// `https://github.com/xidl/xidl/xidl-std`, the scp form
/// `git@github.com:xidl/xidl` and plain filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSource {
    host: String,
    repo: String,
    subdir: Option<String>,
}

impl PackageSource {
    /// Splits `src`, using [`LOCAL_HOST`] for path-like sources.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Source`] when `src` has no usable segments.
    pub fn parse(name: &str, src: &str) -> Result<Self, LibraryError> {
        let trimmed = src.trim().trim_end_matches('/');
        let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
        let reject = |reason: &str| LibraryError::Source {
            name: name.to_string(),
            src: src.to_string(),
            reason: reason.to_string(),
        };

        let body = strip_scheme(trimmed);
        let (explicit_host, path) = match body.split_once(':') {
            Some((host, rest)) if host.contains('@') => {
                (Some(host.rsplit('@').next().unwrap_or(host)), rest)
            }
            _ => (None, body),
        };
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let segments = path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if segments.is_empty() {
            return Err(reject("has no repository path"));
        }
        let source = match explicit_host {
            Some(host) if !host.is_empty() => Self::from_parts(host, &segments),
            _ if is_host_segment(segments.first().copied()) => {
                Self::from_parts(segments[0], &segments[1..])
            }
            // A local source keeps its original spelling so git resolves it
            // exactly as written, absolute path included.
            _ => Self {
                host: LOCAL_HOST.to_string(),
                repo: trimmed.to_string(),
                subdir: None,
            },
        };
        if source.repo.is_empty() {
            return Err(reject("has no repository path"));
        }
        Ok(source)
    }

    /// Assembles a remote source from a host and repository segments.
    fn from_parts(host: &str, segments: &[&str]) -> Self {
        let repo = segments
            .iter()
            .take(2)
            .copied()
            .collect::<Vec<_>>()
            .join("/");
        let subdir = segments.get(2..).map(|rest| rest.join("/"));
        Self {
            host: host.to_string(),
            repo,
            subdir: subdir.filter(|dir| !dir.is_empty()),
        }
    }

    /// Host segment used in the package cache layout, such as `github.com`.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Repository path after the host, such as `xidl/xidl`.
    pub fn repo(&self) -> &str {
        &self.repo
    }

    /// Directory inside the repository that holds the package, when any.
    pub fn subdir(&self) -> Option<&str> {
        self.subdir.as_deref()
    }

    /// Reports whether the source is a filesystem path rather than a remote.
    pub fn is_local(&self) -> bool {
        self.host == LOCAL_HOST
    }

    /// Clone URL of the source.
    pub fn remote(&self) -> String {
        if self.is_local() {
            self.repo.clone()
        } else {
            format!("https://{}/{}", self.host, self.repo)
        }
    }
}

/// Drops a leading URL scheme from `src`.
fn strip_scheme(src: &str) -> &str {
    ["https://", "http://", "ssh://", "git://"]
        .iter()
        .find_map(|scheme| src.strip_prefix(scheme))
        .unwrap_or(src)
}

/// Reports whether `segment` looks like a host such as `github.com`.
///
/// A single `.` or `..` path component is a directory, not a host, so only
/// dotted names whose every part is a label are accepted.
fn is_host_segment(segment: Option<&str>) -> bool {
    let Some(segment) = segment else {
        return false;
    };
    if !segment.contains('.') || segment.starts_with(['.', '-']) {
        return false;
    }
    segment.split('.').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    })
}
