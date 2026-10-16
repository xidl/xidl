//! Source parsing tests.

use crate::library::*;
#[test]
fn source_splits_host_and_repository() {
    let source = PackageSource::parse("timestamp", "github.com/xidl/xidl/xidl-std").unwrap();
    assert_eq!(source.host(), "github.com");
    assert_eq!(source.repo(), "xidl/xidl");
    assert_eq!(source.subdir(), Some("xidl-std"));
    assert_eq!(source.remote(), "https://github.com/xidl/xidl");
    assert!(!source.is_local());
}

#[test]
fn source_accepts_urls_and_scp_forms() {
    for src in [
        "https://github.com/xidl/xidl",
        "git@github.com:xidl/xidl",
        "github.com/xidl/xidl.git",
    ] {
        let source = PackageSource::parse("timestamp", src).unwrap();
        assert_eq!(source.host(), "github.com", "{src}");
        assert_eq!(source.repo(), "xidl/xidl", "{src}");
        assert_eq!(source.subdir(), None, "{src}");
    }
}

#[test]
fn source_keeps_relative_paths_local() {
    let source = PackageSource::parse("timestamp", "../libs/timestamp").unwrap();
    assert!(source.is_local());
    assert_eq!(source.host(), LOCAL_HOST);
    assert_eq!(source.remote(), "../libs/timestamp");
}

#[test]
fn source_rejects_a_bare_host() {
    let err = PackageSource::parse("timestamp", "github.com").unwrap_err();
    assert!(err.to_string().contains("timestamp"), "{err}");
}
