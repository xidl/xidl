//! Shared fixtures for the library unit tests.

use crate::library::*;
use semver::{Version, VersionReq};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates an empty fixture directory unique to the calling test.
pub(super) fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xidlc-library-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create fixture dir");
    path
}

/// Removes a fixture directory created by [`temp_dir`].
pub(super) fn cleanup(path: PathBuf) {
    fs::remove_dir_all(path).expect("remove fixture");
}

/// Writes `content` to `path`, creating parent directories.
pub(super) fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture");
}

/// Fetcher that records calls and materializes a marker file.
#[derive(Default)]
pub(super) struct FakeFetcher {
    pub(super) fetched: RefCell<Vec<String>>,
}

impl FakeFetcher {
    /// Number of fetches performed so far.
    pub(super) fn fetch_count(&self) -> usize {
        self.fetched.borrow().len()
    }
}

impl PackageFetcher for FakeFetcher {
    fn fetch(
        &self,
        name: &str,
        dependency: &xidl_parser::manifest::Dependency,
        destination: &Path,
    ) -> Result<(), LibraryError> {
        self.fetched
            .borrow_mut()
            .push(format!("{name}@{}", dependency.revision().as_str()));
        fs::create_dir_all(destination).map_err(|source| LibraryError::Io {
            path: destination.to_path_buf(),
            source,
        })?;
        let source = PackageSource::parse(name, dependency.src())?;
        let root = match source.subdir() {
            Some(subdir) => destination.join(subdir),
            None => destination.to_path_buf(),
        };
        write(&root.join(".fetched"), dependency.revision().as_str());
        Ok(())
    }

    fn is_current(
        &self,
        dependency: &xidl_parser::manifest::Dependency,
        destination: &Path,
    ) -> bool {
        let marker = destination.join(".fetched");
        fs::read_to_string(marker).is_ok_and(|rev| rev == dependency.revision().as_str())
    }
}

/// Shares one [`FakeFetcher`] with the store under test.
pub(super) struct SharedFetcher(pub(super) Rc<FakeFetcher>);

impl PackageFetcher for SharedFetcher {
    fn fetch(
        &self,
        name: &str,
        dependency: &xidl_parser::manifest::Dependency,
        destination: &Path,
    ) -> Result<(), LibraryError> {
        self.0.fetch(name, dependency, destination)
    }

    fn is_current(
        &self,
        dependency: &xidl_parser::manifest::Dependency,
        destination: &Path,
    ) -> bool {
        self.0.is_current(dependency, destination)
    }
}

/// Toolchain source that installs a stub binary.
#[derive(Default)]
pub(super) struct FakeSource {
    pub(super) installed: RefCell<Vec<String>>,
}

impl ToolchainSource for FakeSource {
    fn latest_matching(&self, requirement: &VersionReq) -> Result<Version, LibraryError> {
        for candidate in [Version::new(9, 9, 9), Version::new(1, 0, 0)] {
            if requirement.matches(&candidate) {
                return Ok(candidate);
            }
        }
        Err(LibraryError::Toolchain {
            requirement: requirement.to_string(),
            reason: "fixture has no matching release".to_string(),
        })
    }

    fn install(&self, version: &Version, directory: &Path) -> Result<(), LibraryError> {
        self.installed.borrow_mut().push(version.to_string());
        fs::create_dir_all(directory).map_err(|source| LibraryError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        write(
            &directory.join(crate::library::toolchain::binary_file_name()),
            "stub",
        );
        Ok(())
    }
}

/// A dependency declaration for `src` pinned to `tag`.
/// A dependency declaration for `src` pinned to `tag`.
pub(super) fn dependency(src: &str, tag: &str) -> xidl_parser::manifest::Dependency {
    xidl_parser::manifest::Dependency::new(
        src,
        xidl_parser::manifest::Revision::Tag(tag.to_string()),
    )
}

/// Parses a manifest fixture under `dir`.
pub(super) fn manifest_for(dir: &Path, text: &str) -> xidl_parser::manifest::Manifest {
    let path = dir.join("xidl.toml");
    write(&path, text);
    xidl_parser::manifest::Manifest::load(&path).expect("fixture manifest parses")
}
