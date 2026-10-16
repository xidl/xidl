//! End-to-end coverage for `xidl.toml` libraries and their mounted dependencies.
//!
//! The dependency is a real git repository created in a temporary directory, so
//! the test exercises manifest expansion, package fetching, the dependency mount
//! and `#include "<name>/<file>"` resolution without touching the network.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use xidlc::driver::Generator;
use xidlc::library::{GitPackageFetcher, LibraryLoader, MOUNT_PROPERTY, PackageStore};

/// Creates an empty fixture directory unique to this test.
fn temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("xidlc-manifest-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&path).expect("create fixture dir");
    path
}

/// Writes `content` to `path`, creating parent directories.
fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create fixture parent");
    }
    fs::write(path, content).expect("write fixture");
}

/// Creates a git repository holding `files`, tagged `v1.0.0`.
fn init_dependency_repo(dir: &Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        write(&dir.join(name), content);
    }
    for args in [
        vec!["init", "--quiet"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=xidl",
            "-c",
            "user.email=xidl@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "initial",
        ],
        vec!["-c", "tag.gpgsign=false", "tag", "v1.0.0"],
    ] {
        let status = Command::new("git")
            .args(&args)
            .current_dir(dir)
            .status()
            .expect("run git");
        assert!(status.success(), "git {args:?} failed");
    }
}

#[test]
fn manifest_expands_inputs_and_mounts_a_dependency() {
    let root = temp_dir();
    let packages = root.join("cache");
    let dependency = root.join("timestamp");
    let project = root.join("project");

    init_dependency_repo(
        &dependency,
        &[("lib.idl", "struct Stamp { int32 value; };\n")],
    );
    write(
        &project.join("xidl.toml"),
        &format!(
            "name = \"project\"\nversion = \"1.0.0\"\n\n[dependencies]\n\
             \"timestamp\" = {{ src = \"{}\", tag = \"v1.0.0\" }}\n",
            dependency.display()
        ),
    );
    write(
        &project.join("contract.idl"),
        "#include \"timestamp/lib.idl\"\nstruct Contract { Stamp stamp; };\n",
    );
    // A non-IDL file next to the manifest must not become an input.
    write(&project.join("README.md"), "not an input\n");

    let loader = LibraryLoader::with_packages(PackageStore::with_root(
        packages.clone(),
        Box::new(GitPackageFetcher),
    ));
    let resolved = loader
        .resolve(std::slice::from_ref(&project))
        .expect("resolve library");

    assert_eq!(resolved.files().len(), 1, "only contract.idl is an input");
    assert_eq!(
        resolved.files()[0]
            .file_name()
            .expect("input has a file name")
            .to_string_lossy(),
        "contract.idl",
        "non-IDL files are skipped"
    );
    let mount = resolved
        .mounts()
        .get("timestamp")
        .expect("dependency mounted");
    assert!(mount.join("lib.idl").is_file(), "{}", mount.display());

    let props = HashMap::from([(MOUNT_PROPERTY.to_string(), resolved.mount_property())]);
    let mut generator = Generator::new("hir".to_string());
    for file in resolved.files() {
        let source = fs::read_to_string(file).expect("read input");
        let artifacts = generator
            .generate_from_idl(&source, file, props.clone())
            .expect("generate hir");
        let artifact = artifacts.first().expect("hir generation emits an artifact");
        let content = artifact.content();
        assert!(
            content.contains("Stamp"),
            "the mounted dependency should reach the generated HIR: {content}"
        );
    }
    fs::remove_dir_all(root).expect("remove fixture");
}

#[test]
fn manifest_requires_the_declared_compiler_range() {
    let root = temp_dir();
    let project = root.join("project");
    write(
        &project.join("xidl.toml"),
        "name = \"project\"\nversion = \"1.0.0\"\ncompiler = \">=99.0.0\"\n",
    );
    write(
        &project.join("contract.idl"),
        "struct Contract { int32 v; };\n",
    );

    let loader = LibraryLoader::with_packages(PackageStore::with_root(
        root.join("cache"),
        Box::new(GitPackageFetcher),
    ));
    let resolved = loader.resolve(&[project]).expect("resolve library");

    let err = resolved
        .check_compiler()
        .expect_err("the running compiler is out of range");
    assert!(err.to_string().contains(">=99.0.0"), "{err}");
    fs::remove_dir_all(root).expect("remove fixture");
}
