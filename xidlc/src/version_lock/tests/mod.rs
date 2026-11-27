mod cli;
mod imports;
#[cfg(feature = "gen-openapi")]
mod openapi;
#[cfg(feature = "gen-openapi")]
mod openapi_lock;
mod subset;

use super::*;

#[test]
fn sha256_matches_nist_test_vectors() {
    assert_eq!(
        VersionLock::fingerprint(b""),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        VersionLock::fingerprint(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        VersionLock::fingerprint(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "sha256:248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    let repeated = vec![b'a'; 1000];
    assert_eq!(
        VersionLock::fingerprint(&repeated),
        "sha256:41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
    );
}

#[test]
fn locked_version_stays_stable_when_later_versions_or_comments_are_added() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("workspace.idl");
    let lock = VersionLock::default_lock_path(&idl);
    assert_eq!(lock, dir.path().join("workspace.versions.lock.json"));

    std::fs::write(
        &idl,
        r#"
        @version enum V { @rename("2026-12-01") v1 };
        struct Workspace { string id; string name; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();

    let first = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(first.is_written());
    assert_eq!(first.document.versions.len(), 1);
    let v1_hash = first.document.versions["2026-12-01"].fingerprint.clone();

    // Add a later version, a v2-only struct, and trivia/comments.
    std::fs::write(
        &idl,
        r#"
        /* comment added */
        @version enum V {
            @rename("2026-12-01") v1,
            @rename("2027-06-01") v2
        };
        struct Workspace {
            string id;
            string name;
        };
        @added(V::v2)
        struct AuditLog { string actor; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();

    let check = VersionLock::from_file(&idl).unwrap().check(&lock).unwrap();
    assert!(check.is_clean(), "{:?}", check.drifts);

    let selected = VersionLock::from_file(&idl)
        .unwrap()
        .with_versions(Some(["2026-12-01".into(), "2027-06-01".into()].into()))
        .check(&lock)
        .unwrap();
    assert_eq!(
        selected.drifts,
        vec![LockDrift::Unlocked {
            version: "2027-06-01".into(),
            id: "v2".into(),
        }]
    );

    let updated = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(updated.is_written());
    assert_eq!(updated.document.versions["2026-12-01"].fingerprint, v1_hash);
    assert!(updated.document.versions.contains_key("2027-06-01"));
}

#[test]
fn detects_silent_edit_removed_version_and_parameter_changes() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("workspace.idl");
    let lock = dir.path().join("custom/versions.lock.json");

    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Version")
        enum V {
            @rename("2026-12-01") v1,
            @rename("2027-06-01") v2
        };
        struct Workspace { string id; string name; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();

    let created = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(created.is_written());

    // Silent field rename without @added/@removed mutates both frozen versions.
    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Version")
        enum V {
            @rename("2026-12-01") v1,
            @rename("2027-06-01") v2
        };
        struct Workspace { string id; string title; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();

    let before_refusal = std::fs::read_to_string(&lock).unwrap();
    let report = VersionLock::from_file(&idl).unwrap().check(&lock).unwrap();
    assert_eq!(report.drifts.len(), 2);
    assert!(
        report.drifts.iter().all(|drift| matches!(
            drift,
            LockDrift::Changed { expected, actual, .. } if expected != actual
        )),
        "{:?}",
        report.drifts
    );
    let rendered = report.render(&idl, &lock);
    assert!(rendered.contains("2026-12-01"), "{rendered}");
    assert!(rendered.contains("2027-06-01"), "{rendered}");

    // Locking without --force refuses to overwrite drifted entries and preserves disk bytes.
    let refused = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(!refused.is_written());
    assert!(refused.render_drifts(&idl).contains("--force"));
    assert_eq!(
        std::fs::read_to_string(&lock).unwrap(),
        before_refusal,
        "unforced lock must not modify existing lock file"
    );

    // Locking with --force updates the lock file.
    let forced = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, true)
        .unwrap();
    assert!(forced.is_written());
    assert!(
        VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .is_clean()
    );

    // Changing the @version header name changes frozen fingerprints.
    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Api-Version")
        enum V {
            @rename("2026-12-01") v1,
            @rename("2027-06-01") v2
        };
        struct Workspace { string id; string title; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();
    let param_drift = VersionLock::from_file(&idl).unwrap().check(&lock).unwrap();
    assert_eq!(param_drift.drifts.len(), 2);
    assert!(
        param_drift
            .drifts
            .iter()
            .all(|drift| matches!(drift, LockDrift::Changed { .. })),
        "{:?}",
        param_drift.drifts
    );

    // Removing a locked version is flagged as Removed.
    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Version")
        enum V { @rename("2026-12-01") v1 };
        struct Workspace { string id; string title; };
        interface Api {
            @get(path = "/workspaces")
            Workspace get();
        };
        "#,
    )
    .unwrap();
    let removed = VersionLock::from_file(&idl).unwrap().check(&lock).unwrap();
    assert_eq!(
        removed.drifts,
        vec![LockDrift::Removed {
            version: "2027-06-01".into(),
            id: "v2".into(),
        }]
    );
}

#[test]
fn invalid_lock_files_and_missing_paths_return_typed_errors() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("api.idl");
    std::fs::write(&idl, "@version enum V { v1 }; struct S { string id; };").unwrap();
    let lock = VersionLock::from_file(&idl).unwrap();

    let missing = dir.path().join("missing.lock.json");
    let missing_err = lock.check(&missing).unwrap_err().to_string();
    assert!(missing_err.contains("cannot read"), "{missing_err}");

    let bad_json = dir.path().join("bad.lock.json");
    std::fs::write(&bad_json, "not json").unwrap();
    let bad_json_err = lock.check(&bad_json).unwrap_err().to_string();
    assert!(bad_json_err.contains("invalid JSON"), "{bad_json_err}");
    for force in [false, true] {
        let write_bad_err = lock.write_lock(&bad_json, force).unwrap_err().to_string();
        assert!(write_bad_err.contains("invalid JSON"), "{write_bad_err}");
        assert_eq!(std::fs::read_to_string(&bad_json).unwrap(), "not json");
    }

    let bad_schema = dir.path().join("future.lock.json");
    std::fs::write(&bad_schema, r#"{"schema_version": 99, "versions": {}}"#).unwrap();
    let err = lock.check(&bad_schema).unwrap_err().to_string();
    assert!(err.contains("schema_version 99"), "{err}");
    let err = lock.write_lock(&bad_schema, true).unwrap_err().to_string();
    assert!(err.contains("schema_version 99"), "{err}");

    assert_eq!(
        VersionLock::default_lock_path(Path::new("api.idl")),
        PathBuf::from("api.versions.lock.json")
    );
    assert_eq!(
        VersionLock::default_lock_path(Path::new("/")),
        PathBuf::from("contract.versions.lock.json")
    );
}
