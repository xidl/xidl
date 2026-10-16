use super::command::{CommandRunner, argument};
use super::error::LibraryError;
use semver::{Version, VersionReq};
use std::path::{Path, PathBuf};

/// Directory of the toolchain cache, relative to the xidl home.
pub const TOOLCHAIN_DIR: &str = "toolchain";

/// Repository that publishes `xidlc` release binaries.
pub const RELEASE_REPOSITORY: &str = "xidl/xidl";

/// Source of released `xidlc` binaries.
pub trait ToolchainSource {
    /// Highest released version satisfying `requirement`.
    fn latest_matching(&self, requirement: &VersionReq) -> Result<Version, LibraryError>;

    /// Installs the `version` binary into `directory`.
    fn install(&self, version: &Version, directory: &Path) -> Result<(), LibraryError>;
}

/// Cache of downloaded `xidlc` toolchains, rooted at `~/.xidl/toolchain`.
///
/// A manifest may require a compiler range the running binary does not
/// satisfy. [`Toolchain::ensure`] then installs the newest matching release
/// into `<root>/<version>/xidlc`, so the requested binary can be executed
/// directly.
pub struct Toolchain {
    root: Option<PathBuf>,
    source: Box<dyn ToolchainSource>,
}

impl Toolchain {
    /// Creates a toolchain cache over the default home and GitHub releases.
    pub fn new() -> Self {
        Self {
            root: crate::library::home::CacheHome::dir(TOOLCHAIN_DIR),
            source: Box::new(GithubReleaseSource::new()),
        }
    }

    /// Creates a toolchain cache rooted at `root` using `source`.
    pub fn with_root(root: PathBuf, source: Box<dyn ToolchainSource>) -> Self {
        Self {
            root: Some(root),
            source,
        }
    }

    /// Version of the running `xidlc`.
    pub fn current_version() -> Version {
        Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| Version::new(0, 0, 0))
    }

    /// Binary to run for `requirement`, or `None` to keep the current one.
    ///
    /// # Errors
    ///
    /// Returns [`LibraryError::Toolchain`] when the requirement is out of range
    /// and no matching release can be installed.
    pub fn ensure(&self, requirement: &VersionReq) -> Result<Option<PathBuf>, LibraryError> {
        if requirement.matches(&Self::current_version()) {
            return Ok(None);
        }
        let root = self.root.as_deref().ok_or(LibraryError::CacheRoot)?;
        if let Some(binary) = self.cached(root, requirement) {
            return Ok(Some(binary));
        }

        let version = self.source.latest_matching(requirement)?;
        let directory = root.join(version.to_string());
        self.source.install(&version, &directory)?;

        let binary = directory.join(binary_file_name());
        if !binary.is_file() {
            return Err(LibraryError::Toolchain {
                requirement: requirement.to_string(),
                reason: format!("{} was not created by the installer", binary.display()),
            });
        }
        Ok(Some(binary))
    }

    /// Newest already-installed version satisfying `requirement`, if any.
    fn cached(&self, root: &Path, requirement: &VersionReq) -> Option<PathBuf> {
        let entries = std::fs::read_dir(root).ok()?;
        let mut best: Option<(Version, PathBuf)> = None;
        for entry in entries.flatten() {
            let name = entry.file_name();
            let Ok(version) = Version::parse(&name.to_string_lossy()) else {
                continue;
            };
            if !requirement.matches(&version) {
                continue;
            }
            let binary = entry.path().join(binary_file_name());
            if !binary.is_file() {
                continue;
            }
            if best.as_ref().is_none_or(|(best, _)| version > *best) {
                best = Some((version, binary));
            }
        }
        best.map(|(_, binary)| binary)
    }
}

impl Default for Toolchain {
    fn default() -> Self {
        Self::new()
    }
}

/// Name of the `xidlc` executable on this platform.
pub fn binary_file_name() -> &'static str {
    if cfg!(windows) { "xidlc.exe" } else { "xidlc" }
}

/// Rust target triple of the release asset for this platform.
///
/// # Errors
///
/// Returns [`LibraryError::Toolchain`] for platforms the release workflow does
/// not build, so the failure names the platform instead of guessing a URL.
pub fn target_triple() -> Result<&'static str, LibraryError> {
    let triple = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-musl",
        ("linux", "aarch64") => "aarch64-unknown-linux-musl",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc",
        (os, arch) => {
            return Err(LibraryError::Toolchain {
                requirement: "*".to_string(),
                reason: format!("xidlc publishes no release binary for {os}/{arch}"),
            });
        }
    };
    Ok(triple)
}

/// Reads `xidlc` release binaries from GitHub release assets.
pub struct GithubReleaseSource {
    repository: String,
    base_url: String,
    api_url: String,
}

impl GithubReleaseSource {
    /// Source for the default `xidl/xidl` repository.
    pub fn new() -> Self {
        Self::with_endpoints(
            RELEASE_REPOSITORY.to_string(),
            "https://github.com".to_string(),
            "https://api.github.com".to_string(),
        )
    }

    /// Source for `repository`, reachable through `base_url`/`api_url`.
    pub fn with_endpoints(repository: String, base_url: String, api_url: String) -> Self {
        Self {
            repository,
            base_url,
            api_url,
        }
    }

    /// URL of the release asset carrying the binary for this platform.
    fn asset_url(&self, version: &Version, triple: &str) -> String {
        format!(
            "{}/{}/releases/download/v{version}/xidlc-{triple}.tar.gz",
            self.base_url, self.repository
        )
    }
}

impl Default for GithubReleaseSource {
    fn default() -> Self {
        Self::new()
    }
}

/// The only part of a release payload this source reads.
#[derive(serde::Deserialize)]
struct ReleasePayload {
    tag_name: String,
}

impl ToolchainSource for GithubReleaseSource {
    fn latest_matching(&self, requirement: &VersionReq) -> Result<Version, LibraryError> {
        let url = format!(
            "{}/repos/{}/releases?per_page=100",
            self.api_url, self.repository
        );
        let body = CommandRunner::output(
            "curl",
            &[
                "-fsSL".to_string(),
                "-H".to_string(),
                "Accept: application/vnd.github+json".to_string(),
                url,
            ],
        )
        .map_err(|err| LibraryError::Toolchain {
            requirement: requirement.to_string(),
            reason: format!("cannot list releases: {err}"),
        })?;

        let payload: Vec<ReleasePayload> =
            serde_json::from_str(&body).map_err(|err| LibraryError::Toolchain {
                requirement: requirement.to_string(),
                reason: format!("cannot read the release listing: {err}"),
            })?;

        payload
            .iter()
            .filter_map(|release| Version::parse(release.tag_name.trim_start_matches('v')).ok())
            .filter(|version| requirement.matches(version))
            .max()
            .ok_or_else(|| LibraryError::Toolchain {
                requirement: requirement.to_string(),
                reason: format!("no {} release satisfies it", self.repository),
            })
    }

    fn install(&self, version: &Version, directory: &Path) -> Result<(), LibraryError> {
        let triple = target_triple()?;
        std::fs::create_dir_all(directory).map_err(|source| LibraryError::Io {
            path: directory.to_path_buf(),
            source,
        })?;

        let archive = directory.join("xidlc.tar.gz");
        let archive_arg = argument(&archive);
        CommandRunner::run(
            "curl",
            &[
                "-fsSL".to_string(),
                "-o".to_string(),
                archive_arg,
                self.asset_url(version, triple),
            ],
        )?;
        CommandRunner::run(
            "tar",
            &[
                "-xzf".to_string(),
                argument(&archive),
                "-C".to_string(),
                argument(directory),
            ],
        )?;
        std::fs::remove_file(&archive).map_err(|source| LibraryError::Io {
            path: archive,
            source,
        })?;
        Ok(())
    }
}
