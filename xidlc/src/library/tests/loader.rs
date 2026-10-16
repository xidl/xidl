//! Manifest discovery, expansion, and compiler range tests.

use super::support::*;
use crate::library::*;
use std::collections::BTreeMap;
#[test]
fn discovery_ignores_plain_inputs() {
    let dir = temp_dir("discovery");
    write(&dir.join("plain.idl"), "struct A { int32 v; };\n");

    assert!(
        LoadedManifest::discover(&dir.join("plain.idl"))
            .unwrap()
            .is_none()
    );
    assert!(LoadedManifest::discover(&dir).unwrap().is_none());
    cleanup(dir);
}

#[test]
fn discovery_accepts_a_manifest_path_and_its_directory() {
    let dir = temp_dir("discovery-manifest");
    let manifest = manifest_for(&dir, "name = \"a\"\nversion = \"1.0.0\"\n");

    let by_dir = LoadedManifest::discover(&dir).unwrap().expect("directory");
    let by_file = LoadedManifest::discover(&dir.join("xidl.toml"))
        .unwrap()
        .expect("manifest file");

    assert_eq!(by_dir.dir(), dir.as_path());
    assert_eq!(by_dir.path(), dir.join("xidl.toml").as_path());
    assert_eq!(by_file.manifest(), &manifest);
    cleanup(dir);
}

#[test]
fn loader_expands_libraries_and_passes_plain_files_through() {
    let dir = temp_dir("loader");
    let packages = dir.join("pkgs");
    write(
        &dir.join("proj/xidl.toml"),
        "name = \"proj\"\nversion = \"1.0.0\"\n\n[dependencies]\n\"timestamp\" = { src = \"github.com/x/timestamp\", tag = \"v1.2.3\" }\n",
    );
    write(&dir.join("proj/lib.idl"), "struct Lib { int32 v; };\n");
    write(&dir.join("proj/draft.idl"), "struct Draft { int32 v; };\n");
    write(&dir.join("extra.idl"), "struct Extra { int32 v; };\n");

    let store = PackageStore::with_root(packages.clone(), Box::new(FakeFetcher::default()));
    let loader = LibraryLoader::with_packages(store);
    let resolved = loader
        .resolve(&[dir.join("proj"), dir.join("extra.idl")])
        .unwrap();

    let names = resolved
        .files()
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(names, ["draft.idl", "lib.idl", "extra.idl"]);
    assert_eq!(
        resolved.mounts().get("timestamp"),
        Some(&packages.join("github.com").join("timestamp-v1.2.3"))
    );
    assert_eq!(resolved.manifests().len(), 1);
    assert_eq!(
        resolved.mount_property()["timestamp"],
        serde_json::Value::String(
            packages
                .join("github.com")
                .join("timestamp-v1.2.3")
                .to_string_lossy()
                .into_owned()
        )
    );
    cleanup(dir);
}

#[test]
fn loader_deduplicates_repeated_arguments() {
    let dir = temp_dir("loader-dedup");
    write(&dir.join("extra.idl"), "struct Extra { int32 v; };\n");

    let loader = LibraryLoader::with_packages(PackageStore::with_root(
        dir.join("pkgs"),
        Box::new(FakeFetcher::default()),
    ));
    let resolved = loader
        .resolve(&[dir.join("extra.idl"), dir.join("extra.idl")])
        .unwrap();

    assert_eq!(resolved.files().len(), 1);
    cleanup(dir);
}

#[test]
fn check_compiler_accepts_the_running_version() {
    let dir = temp_dir("compiler-ok");
    let running = Toolchain::current_version();
    let manifest = manifest_for(
        &dir,
        &format!("name = \"a\"\nversion = \"1.0.0\"\ncompiler = \">={running}\"\n"),
    );
    let resolved = ResolvedInputs {
        files: Vec::new(),
        mounts: BTreeMap::new(),
        manifests: vec![LoadedManifest::discover(&dir).unwrap().unwrap()],
    };

    assert_eq!(
        LoadedManifest::discover(&dir).unwrap().unwrap().manifest(),
        &manifest
    );
    assert!(resolved.check_compiler().is_ok());
    cleanup(dir);
}

#[test]
fn check_compiler_rejects_an_unreachable_range() {
    let dir = temp_dir("compiler-bad");
    manifest_for(
        &dir,
        "name = \"a\"\nversion = \"1.0.0\"\ncompiler = \">=99.0.0\"\n",
    );
    let resolved = ResolvedInputs {
        files: Vec::new(),
        mounts: BTreeMap::new(),
        manifests: vec![LoadedManifest::discover(&dir).unwrap().unwrap()],
    };

    let err = resolved.check_compiler().unwrap_err();
    assert!(err.to_string().contains(">=99.0.0"), "{err}");
    assert!(err.to_string().contains("xidl.toml"), "{err}");
    cleanup(dir);
}

#[test]
fn errors_name_the_failing_contract() {
    let err = LibraryError::CacheRoot;
    assert!(err.to_string().contains("XIDL_HOME"), "{err}");

    let err = LibraryError::Command {
        program: "git".to_string(),
        args: vec!["clone".to_string()],
        reason: "boom".to_string(),
    };
    assert!(err.to_string().contains("git clone"), "{err}");
}
