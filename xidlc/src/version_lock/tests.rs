use super::sha256::Sha256;
use super::*;

#[test]
fn sha256_matches_nist_test_vectors() {
    assert_eq!(
        Sha256::prefixed_hex(b""),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        Sha256::prefixed_hex(b"abc"),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        Sha256::prefixed_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "sha256:248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    let repeated = vec![b'a'; 1000];
    assert_eq!(
        Sha256::prefixed_hex(&repeated),
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

    let check = VersionLock::from_file(&idl)
        .unwrap()
        .check(&lock, false)
        .unwrap();
    assert!(check.is_clean(), "{:?}", check.drifts);

    let strict = VersionLock::from_file(&idl)
        .unwrap()
        .check(&lock, true)
        .unwrap();
    assert_eq!(
        strict.drifts,
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

    let report = VersionLock::from_file(&idl)
        .unwrap()
        .check(&lock, false)
        .unwrap();
    assert_eq!(report.drifts.len(), 2);
    let rendered = report.render(&idl, &lock);
    assert!(rendered.contains("2026-12-01"), "{rendered}");
    assert!(rendered.contains("2027-06-01"), "{rendered}");

    // Locking without --force refuses to overwrite drifted entries.
    let refused = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(!refused.is_written());
    assert!(refused.render_drifts(&idl).contains("--force"));

    // Locking with --force updates the lock file.
    let forced = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, true)
        .unwrap();
    assert!(forced.is_written());
    assert!(
        VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock, false)
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
    let param_drift = VersionLock::from_file(&idl)
        .unwrap()
        .check(&lock, false)
        .unwrap();
    assert_eq!(param_drift.drifts.len(), 2);

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
    let removed = VersionLock::from_file(&idl)
        .unwrap()
        .check(&lock, false)
        .unwrap();
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
    assert!(lock.check(&missing, false).is_err());

    let bad_json = dir.path().join("bad.lock.json");
    std::fs::write(&bad_json, "not json").unwrap();
    assert!(lock.check(&bad_json, false).is_err());

    let bad_schema = dir.path().join("future.lock.json");
    std::fs::write(&bad_schema, r#"{"schema_version": 99, "versions": {}}"#).unwrap();
    let err = lock.check(&bad_schema, false).unwrap_err().to_string();
    assert!(err.contains("schema_version 99"), "{err}");
}

#[test]
fn cli_version_lock_and_check_enforce_exit_codes() {
    use clap::Parser;

    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("api.idl");
    let lock = dir.path().join("api.versions.lock.json");
    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Version")
        enum V { @rename("2026-12-01") v1 };
        struct Workspace { string id; string name; };
        "#,
    )
    .unwrap();

    let cli = crate::cli::Cli::try_parse_from(["xidlc", "version", "lock", idl.to_str().unwrap()])
        .unwrap();
    cli.run().expect("initial lock succeeds");
    assert!(lock.exists());

    let cli = crate::cli::Cli::try_parse_from(["xidlc", "version", "check", idl.to_str().unwrap()])
        .unwrap();
    cli.run().expect("clean check exits 0");

    // Mutate frozen field without @added/@removed.
    std::fs::write(
        &idl,
        r#"
        @version(in = "header", name = "X-Version")
        enum V { @rename("2026-12-01") v1 };
        struct Workspace { string id; string title; };
        "#,
    )
    .unwrap();
    let cli = crate::cli::Cli::try_parse_from(["xidlc", "version", "check", idl.to_str().unwrap()])
        .unwrap();
    let err = cli.run().unwrap_err();
    assert!(matches!(err, IdlcError::CliExit { code: 1, .. }), "{err:?}");

    let cli = crate::cli::Cli::try_parse_from(["xidlc", "version", "lock", idl.to_str().unwrap()])
        .unwrap();
    let err = cli.run().unwrap_err();
    assert!(matches!(err, IdlcError::CliExit { code: 1, .. }), "{err:?}");

    let cli = crate::cli::Cli::try_parse_from([
        "xidlc",
        "version",
        "lock",
        "--force",
        idl.to_str().unwrap(),
    ])
    .unwrap();
    cli.run().expect("forced relock exits 0");

    let missing_lock = dir.path().join("missing.json");
    let cli = crate::cli::Cli::try_parse_from([
        "xidlc",
        "version",
        "check",
        "-l",
        missing_lock.to_str().unwrap(),
        idl.to_str().unwrap(),
    ])
    .unwrap();
    let err = cli.run().unwrap_err();
    assert!(matches!(err, IdlcError::CliExit { code: 2, .. }), "{err:?}");
}
