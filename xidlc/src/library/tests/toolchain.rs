//! Toolchain cache tests.

use super::support::*;
use crate::library::*;
use semver::VersionReq;
#[test]
fn toolchain_keeps_the_current_binary_inside_the_range() {
    let root = temp_dir("toolchain-current");
    let source = FakeSource::default();
    let toolchain = Toolchain::with_root(root.clone(), Box::new(FakeSource::default()));

    let binary = toolchain
        .ensure(&VersionReq::parse(">=0.0.1").unwrap())
        .unwrap();

    assert!(binary.is_none());
    assert!(source.installed.borrow().is_empty());
    cleanup(root);
}

#[test]
fn toolchain_installs_a_matching_release() {
    let root = temp_dir("toolchain-install");
    let toolchain = Toolchain::with_root(root.clone(), Box::new(FakeSource::default()));

    let binary = toolchain
        .ensure(&VersionReq::parse(">=9.0.0, <10").unwrap())
        .unwrap()
        .expect("a toolchain should be installed");

    assert_eq!(
        binary,
        root.join("9.9.9")
            .join(crate::library::toolchain::binary_file_name())
    );
    assert!(binary.is_file());
    cleanup(root);
}

#[test]
fn toolchain_reuses_an_installed_release() {
    let root = temp_dir("toolchain-cached");
    let installed = root.join("9.9.9");
    write(
        &installed.join(crate::library::toolchain::binary_file_name()),
        "stub",
    );
    let source = FakeSource::default();
    let toolchain = Toolchain::with_root(root.clone(), Box::new(source));

    let binary = toolchain
        .ensure(&VersionReq::parse(">=9.0.0, <10").unwrap())
        .unwrap();

    assert_eq!(
        binary,
        Some(installed.join(crate::library::toolchain::binary_file_name()))
    );
    cleanup(root);
}

#[test]
fn toolchain_reports_platform_targets() {
    let triple = crate::library::toolchain::target_triple();
    assert!(triple.is_ok() || cfg!(not(any(target_os = "linux", target_os = "macos"))));
}
