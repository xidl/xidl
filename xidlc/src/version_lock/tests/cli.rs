use super::*;

#[test]
fn imported_unit_changes_trigger_drift_while_unrelated_imported_versions_stay_stable() {
    let dir = tempfile::tempdir().unwrap();
    let shared_dir = dir.path().join("timestamp");
    std::fs::create_dir(&shared_dir).unwrap();
    let shared_idl = shared_dir.join("index.idl");
    let root_idl = dir.path().join("api.idl");
    let lock = VersionLock::default_lock_path(&root_idl);

    std::fs::write(
        &shared_idl,
        "@version enum TV { v1 }; struct Instant { long seconds; };",
    )
    .unwrap();
    std::fs::write(
        &root_idl,
        r#"
        import "timestamp@v1";
        @version enum V { @rename("2026-12-01") v1 };
        struct Event { timestamp::Instant at; };
        "#,
    )
    .unwrap();
    assert!(
        VersionLock::from_file(&root_idl)
            .unwrap()
            .write_lock(&lock, false)
            .unwrap()
            .is_written()
    );

    // Appending v2 to the imported library must not drift `timestamp@v1`.
    std::fs::write(
        &shared_idl,
        r#"
        @version enum TV { v1, v2 };
        struct Instant {
            long seconds;
            @added(TV::v2) long nanos;
        };
        "#,
    )
    .unwrap();
    assert!(
        VersionLock::from_file(&root_idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .is_clean()
    );

    // Mutating `Instant` in `timestamp@v1` must be detected as drift.
    std::fs::write(
        &shared_idl,
        r#"
        @version enum TV { v1, v2 };
        struct Instant {
            long epoch_seconds;
            @added(TV::v2) long nanos;
        };
        "#,
    )
    .unwrap();
    let drift = VersionLock::from_file(&root_idl)
        .unwrap()
        .check(&lock)
        .unwrap();
    assert!(
        matches!(
            drift.drifts.as_slice(),
            [LockDrift::Changed { version, .. }] if version == "2026-12-01"
        ),
        "{:?}",
        drift.drifts
    );
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
    assert!(
        matches!(
            &err,
            IdlcError::CliExit {
                code: 1,
                message: Some(msg)
            } if msg.contains("2026-12-01")
        ),
        "{err:?}"
    );

    let cli = crate::cli::Cli::try_parse_from(["xidlc", "version", "lock", idl.to_str().unwrap()])
        .unwrap();
    let err = cli.run().unwrap_err();
    assert!(
        matches!(
            &err,
            IdlcError::CliExit {
                code: 1,
                message: Some(msg)
            } if msg.contains("--force")
        ),
        "{err:?}"
    );

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
    assert!(
        matches!(
            &err,
            IdlcError::CliExit {
                code: 2,
                message: Some(msg)
            } if msg.contains("version check:") && msg.contains("cannot read")
        ),
        "{err:?}"
    );

    let unversioned = dir.path().join("plain.idl");
    std::fs::write(&unversioned, "struct Plain { string id; };").unwrap();
    let cli = crate::cli::Cli::try_parse_from([
        "xidlc",
        "version",
        "lock",
        unversioned.to_str().unwrap(),
    ])
    .unwrap();
    let err = cli.run().unwrap_err();
    assert!(
        matches!(
            &err,
            IdlcError::CliExit {
                code: 2,
                message: Some(msg)
            } if msg.contains("version lock:") && !msg.contains("version diff:")
        ),
        "{err:?}"
    );
}
