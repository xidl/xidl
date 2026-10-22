//! Package cache tests.

use super::support::*;
use crate::library::*;
use std::rc::Rc;
#[test]
fn package_cache_path_follows_host_and_version() {
    let root = temp_dir("cache-path");
    let store = PackageStore::with_root(root.clone(), Box::new(FakeFetcher::default()));

    let path = store
        .cache_dir("timestamp", &dependency("github.com/x/timestamp", "v1.2.3"))
        .unwrap();

    assert_eq!(path, root.join("github.com").join("timestamp-v1.2.3"));
    cleanup(root);
}

#[test]
fn package_cache_path_sanitizes_commit_hashes() {
    let root = temp_dir("cache-rev");
    let store = PackageStore::with_root(root.clone(), Box::new(FakeFetcher::default()));
    let dependency = xidl_parser::manifest::Dependency::new(
        "github.com/x/timestamp",
        xidl_parser::manifest::Revision::Rev("9f0d/abc".to_string()),
    );

    let path = store.cache_dir("timestamp", &dependency).unwrap();
    assert_eq!(path, root.join("github.com").join("timestamp-9f0d_abc"));
    cleanup(root);
}

#[test]
fn package_cache_rejects_path_dependencies() {
    let root = temp_dir("cache-path-dependency");
    let store = PackageStore::with_root(root.clone(), Box::new(FakeFetcher::default()));
    let dependency = xidl_parser::manifest::Dependency::from_path("../timestamp");

    let err = store.cache_dir("timestamp", &dependency).unwrap_err();

    assert!(matches!(err, LibraryError::Source { .. }));
    assert!(err.to_string().contains("mounted directly"), "{err}");

    let err = store.ensure("timestamp", &dependency).unwrap_err();
    assert!(matches!(err, LibraryError::Source { .. }));
    assert!(err.to_string().contains("mounted directly"), "{err}");
    cleanup(root);
}

#[test]
fn package_ensure_fetches_once_and_reuses_the_cache() {
    let root = temp_dir("ensure");
    let fetcher = Rc::new(FakeFetcher::default());
    let store = PackageStore::with_root(root.clone(), Box::new(SharedFetcher(fetcher.clone())));
    let dependency = dependency("github.com/x/timestamp", "v1.2.3");

    let first = store.ensure("timestamp", &dependency).unwrap();
    let second = store.ensure("timestamp", &dependency).unwrap();

    assert_eq!(first, second);
    assert!(first.join(".fetched").is_file());
    assert_eq!(fetcher.fetch_count(), 1, "the cache should be reused");
    cleanup(root);
}

#[test]
fn package_ensure_mounts_the_declared_subdirectory() {
    let root = temp_dir("ensure-subdir");
    let store = PackageStore::with_root(root.clone(), Box::new(FakeFetcher::default()));
    let dependency = dependency("github.com/xidl/xidl/xidl-std", "v1.0.0");

    let mount = store.ensure("xidl/std", &dependency).unwrap();

    assert!(mount.ends_with("xidl-std"), "{}", mount.display());
    assert!(mount.join(".fetched").is_file());
    cleanup(root);
}
