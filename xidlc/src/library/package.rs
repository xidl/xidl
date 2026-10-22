use super::command::{CommandRunner, argument};
use super::error::LibraryError;
use super::source::PackageSource;
use std::path::{Path, PathBuf};
use xidl_parser::manifest::{Dependency, Revision};

/// Directory of the shared cache, relative to the xidl home.
pub const PACKAGE_DIR: &str = "pkg";

/// Materializes dependency packages into the local cache.
pub trait PackageFetcher {
    /// Fetches `dependency` into `destination`, which is not yet present.
    fn fetch(
        &self,
        name: &str,
        dependency: &Dependency,
        destination: &Path,
    ) -> Result<(), LibraryError>;

    /// Reports whether `destination` already holds `dependency`'s revision.
    fn is_current(&self, dependency: &Dependency, destination: &Path) -> bool;
}

/// Cache of fetched dependency packages, rooted at `~/.xidl/pkg`.
///
/// A package lands in `<root>/<host>/<name>-<version>`, where the version is
/// the declared `tag`, or the short `rev` hash when the dependency pins a
/// commit instead of a release.
pub struct PackageStore {
    root: Option<PathBuf>,
    fetcher: Box<dyn PackageFetcher>,
}

impl PackageStore {
    /// Creates a store over the default cache with the git fetcher.
    pub fn new() -> Self {
        Self {
            root: crate::library::home::CacheHome::dir(PACKAGE_DIR),
            fetcher: Box::new(GitPackageFetcher),
        }
    }

    /// Creates a store rooted at `root` using `fetcher`.
    pub fn with_root(root: PathBuf, fetcher: Box<dyn PackageFetcher>) -> Self {
        Self {
            root: Some(root),
            fetcher,
        }
    }

    /// Cache directory of `name`, whether or not it has been fetched.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Source`] when the dependency's `src` cannot be
    /// interpreted, and [`LibraryError::CacheRoot`] when no cache home is known.
    pub fn cache_dir(&self, name: &str, dependency: &Dependency) -> Result<PathBuf, LibraryError> {
        let Some((src, revision)) = dependency.git() else {
            return Err(LibraryError::Source {
                name: name.to_string(),
                src: dependency.path().unwrap_or("<unknown path>").to_string(),
                reason: "path dependencies are mounted directly and have no package cache"
                    .to_string(),
            });
        };
        self.git_cache_dir(name, src, revision)
    }

    /// Cache directory for an already validated Git source and revision.
    fn git_cache_dir(
        &self,
        name: &str,
        src: &str,
        revision: &Revision,
    ) -> Result<PathBuf, LibraryError> {
        let root = self.root()?;
        let source = PackageSource::parse(name, src)?;
        let version = version_segment(revision.as_str());
        Ok(root.join(source.host()).join(format!("{name}-{version}")))
    }

    /// Directory to mount for `name`, fetching it when needed.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Fetch`] when the repository cannot be cloned, and
    /// names the dependency and its source in the message.
    pub fn ensure(&self, name: &str, dependency: &Dependency) -> Result<PathBuf, LibraryError> {
        let Some((src, revision)) = dependency.git() else {
            return Err(LibraryError::Source {
                name: name.to_string(),
                src: dependency.path().unwrap_or("<unknown path>").to_string(),
                reason: "path dependencies are mounted directly and are not fetched".to_string(),
            });
        };
        let cache_dir = self.git_cache_dir(name, src, revision)?;
        if !self.fetcher.is_current(dependency, &cache_dir) {
            if cache_dir.exists() {
                std::fs::remove_dir_all(&cache_dir).map_err(|source| LibraryError::Io {
                    path: cache_dir.clone(),
                    source,
                })?;
            }
            if let Some(parent) = cache_dir.parent() {
                std::fs::create_dir_all(parent).map_err(|source| LibraryError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            self.fetcher
                .fetch(name, dependency, &cache_dir)
                .map_err(|err| LibraryError::Fetch {
                    name: name.to_string(),
                    src: src.to_string(),
                    reason: err.to_string(),
                })?;
        }

        let source = PackageSource::parse(name, src)?;
        Ok(match source.subdir() {
            Some(subdir) => cache_dir.join(subdir),
            None => cache_dir,
        })
    }

    /// Root of the package cache.
    fn root(&self) -> Result<&Path, LibraryError> {
        self.root.as_deref().ok_or(LibraryError::CacheRoot)
    }
}

impl Default for PackageStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Turns a revision into a filesystem-safe cache segment.
fn version_segment(revision: &str) -> String {
    revision
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '+' | '_' => ch,
            _ => '_',
        })
        .collect()
}

/// File inside a cached package recording the revision it was fetched at.
const REVISION_MARKER: &str = ".xidl-revision";

/// Fetches dependencies with `git`, preferring a sparse checkout.
pub struct GitPackageFetcher;

impl GitPackageFetcher {
    /// Runs one `git` invocation against the source repository.
    fn git(args: Vec<String>) -> Result<String, LibraryError> {
        let mut full = vec!["-c".to_string(), "advice.detachedHead=false".to_string()];
        full.extend(args);
        CommandRunner::output("git", &full)
    }

    /// Fetches `revision`, returning the ref the checkout should use.
    fn fetch_revision(dest: &str, revision: &str) -> Result<String, LibraryError> {
        let shallow = Self::git(vec![
            "-C".to_string(),
            dest.to_string(),
            "fetch".to_string(),
            "--depth".to_string(),
            "1".to_string(),
            "origin".to_string(),
            revision.to_string(),
        ]);
        if shallow.is_ok() {
            return Ok("FETCH_HEAD".to_string());
        }
        // A shallow fetch refuses some revisions; a full fetch always has the
        // objects, at the cost of downloading more history.
        Self::git(vec![
            "-C".to_string(),
            dest.to_string(),
            "fetch".to_string(),
            "origin".to_string(),
        ])?;
        Ok(revision.to_string())
    }
}

impl PackageFetcher for GitPackageFetcher {
    fn fetch(
        &self,
        name: &str,
        dependency: &Dependency,
        destination: &Path,
    ) -> Result<(), LibraryError> {
        let Some((src, revision)) = dependency.git() else {
            return Err(LibraryError::Source {
                name: name.to_string(),
                src: dependency.path().unwrap_or("<unknown path>").to_string(),
                reason: "GitPackageFetcher only accepts Git dependencies".to_string(),
            });
        };
        let source = PackageSource::parse(name, src)?;
        let dest = argument(destination);
        let revision = revision.as_str();

        Self::git(vec![
            "init".to_string(),
            "--quiet".to_string(),
            dest.clone(),
        ])?;
        Self::git(vec![
            "-C".to_string(),
            dest.clone(),
            "remote".to_string(),
            "add".to_string(),
            "origin".to_string(),
            source.remote(),
        ])?;

        // A `src` that points inside a repository only needs that subtree, so
        // the checkout is narrowed before any content is written.
        if let Some(subdir) = source.subdir() {
            Self::git(vec![
                "-C".to_string(),
                dest.clone(),
                "sparse-checkout".to_string(),
                "init".to_string(),
                "--cone".to_string(),
            ])?;
            Self::git(vec![
                "-C".to_string(),
                dest.clone(),
                "sparse-checkout".to_string(),
                "set".to_string(),
                subdir.to_string(),
            ])?;
        }

        let target = Self::fetch_revision(&dest, revision)?;
        Self::git(vec![
            "-C".to_string(),
            dest.clone(),
            "checkout".to_string(),
            "--quiet".to_string(),
            target,
        ])?;

        std::fs::write(destination.join(REVISION_MARKER), revision).map_err(|source| {
            LibraryError::Io {
                path: destination.join(REVISION_MARKER),
                source,
            }
        })?;
        Ok(())
    }

    fn is_current(&self, dependency: &Dependency, destination: &Path) -> bool {
        let Some((_, revision)) = dependency.git() else {
            return false;
        };
        std::fs::read_to_string(destination.join(REVISION_MARKER))
            .is_ok_and(|current| current.trim() == revision.as_str())
    }
}
