use super::*;

#[test]
fn lock_follows_transitive_declarations_only_for_versions_that_reach_them() {
    let dir = tempfile::tempdir().unwrap();
    let contract = dir.path().join("api.idl");
    let common = dir.path().join("common");
    let base = common.join("base");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(
        &contract,
        r#"
        import "common";
        @version(in = "header", name = "X-Version")
        enum V { @rename("2026-12-01") v1, @rename("2027-06-01") v2 };
        struct Item { string id; @added(V::v2) @optional common::models::Money price; };
        interface Api { @get(path = "/items") Item get(); };
        "#,
    )
    .unwrap();
    let common_idl = common.join("index.idl");
    let currency_idl = base.join("index.idl");
    let initial_common = r#"
        import "base";
        module models {
            typedef long Amount;
            struct Money { Amount amount; base::Currency currency; };
            struct Unrelated { string note; };
        };
        module unused {
            #pragma custom old
        };
    "#;
    std::fs::write(&common_idl, initial_common).unwrap();
    std::fs::write(&currency_idl, "typedef string Currency;").unwrap();
    let fingerprint = || {
        VersionLock::from_file(&contract)
            .unwrap()
            .document(|_| false)
            .unwrap()
    };
    let first = fingerprint();

    std::fs::write(
        &common_idl,
        initial_common
            .replace("string note", "long note")
            .replace("custom old", "custom new")
            + "struct Other { string name; };",
    )
    .unwrap();
    std::fs::write(
        &currency_idl,
        "typedef string Currency; struct Unused { string note; };",
    )
    .unwrap();
    assert_eq!(fingerprint().versions, first.versions);

    std::fs::write(&currency_idl, "typedef long Currency;").unwrap();
    let changed = fingerprint();
    assert_eq!(changed.versions["2026-12-01"], first.versions["2026-12-01"]);
    assert_ne!(changed.versions["2027-06-01"], first.versions["2027-06-01"]);
}

#[test]
fn interface_exports_resolve_names_in_their_own_scope() {
    let dir = tempfile::tempdir().unwrap();
    let contract = dir.path().join("api.idl");
    let common = dir.path().join("common");
    std::fs::create_dir(&common).unwrap();
    std::fs::write(
        &contract,
        r#"
        import "common";
        @version enum V { v1 };
        const long API_LIMIT = common::Limits::Selected;
        "#,
    )
    .unwrap();
    let shared = common.join("index.idl");
    let source =
        "const long N = 10; interface Limits { const long N = 2; const long Selected = N; };";
    let fingerprint = || {
        VersionLock::from_file(&contract)
            .unwrap()
            .document(|_| false)
            .unwrap()
    };
    std::fs::write(&shared, source).unwrap();
    let original = fingerprint();

    std::fs::write(
        &shared,
        source.replace("= 10", "= 20").replace(
            "const long Selected = N;",
            "const long Selected = N; const long Unused = 3;",
        ),
    )
    .unwrap();
    assert_eq!(fingerprint().versions, original.versions);

    std::fs::write(&shared, source.replace("= 2;", "= 4;")).unwrap();
    assert_ne!(fingerprint().versions, original.versions);
}

#[test]
fn imported_collection_bounds_use_transitive_constants() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("api.idl");
    let shared = dir.path().join("shared.idl");
    let limits = dir.path().join("limits.idl");
    std::fs::write(
        &root,
        r#"
        import "shared.idl";
        @version enum V { v1 };
        struct Response { shared::Payload payload; };
        "#,
    )
    .unwrap();
    std::fs::write(
        &shared,
        r#"import "limits.idl"; struct Payload { sequence<long, limits::SIZE> items; };"#,
    )
    .unwrap();
    std::fs::write(&limits, "const long SIZE = 3;").unwrap();
    let lock = VersionLock::default_lock_path(&root);
    VersionLock::from_file(&root)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();

    std::fs::write(&limits, "const long SIZE = 4;").unwrap();
    let report = VersionLock::from_file(&root).unwrap().check(&lock).unwrap();
    assert!(matches!(
        report.drifts.as_slice(),
        [LockDrift::Changed { artifact: None, .. }]
    ));
}

#[test]
fn references_in_annotations_retain_imported_constants() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("api.idl");
    let shared = dir.path().join("shared.idl");
    std::fs::write(
        &root,
        r#"
        import "shared.idl";
        @version enum V { v1 };
        struct Response { @evidence(limit = shared::SIZE) long value; };
        "#,
    )
    .unwrap();
    std::fs::write(&shared, "const long SIZE = 3;").unwrap();
    let lock = VersionLock::default_lock_path(&root);
    VersionLock::from_file(&root)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();

    std::fs::write(&shared, "const long SIZE = 4;").unwrap();
    let report = VersionLock::from_file(&root).unwrap().check(&lock).unwrap();
    assert!(matches!(
        report.drifts.as_slice(),
        [LockDrift::Changed { artifact: None, .. }]
    ));
}

#[test]
fn explicitly_referenced_import_versions_contribute_idl_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("api.idl");
    let shared = dir.path().join("shared.idl");
    std::fs::write(
        &root,
        r#"
        import "shared.idl@v1";
        @version enum V { v1, v2 };
        struct Response {
            shared::Payload base;
            @added(V::v2) @optional shared::v2::Payload payload;
        };
        "#,
    )
    .unwrap();
    let imports = r#"
        @version enum V { v1, v2, v3 };
        struct Payload {
            long id;
            @added(V::v2) string note;
            @added(V::v3) string unused_field;
        };
    "#;
    std::fs::write(&shared, imports).unwrap();
    let lock = VersionLock::default_lock_path(&root);
    VersionLock::from_file(&root)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();

    std::fs::write(
        &shared,
        imports.replace("string unused_field", "long unused_field"),
    )
    .unwrap();
    let clean = VersionLock::from_file(&root).unwrap().check(&lock).unwrap();
    assert!(clean.is_clean());

    std::fs::write(&shared, imports.replace("string note", "long note")).unwrap();
    let report = VersionLock::from_file(&root).unwrap().check(&lock).unwrap();
    assert!(matches!(
        report.drifts.as_slice(),
        [LockDrift::Changed { version, artifact: None, .. }] if version == "v2"
    ));
}
