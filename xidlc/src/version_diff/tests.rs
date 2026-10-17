use super::*;

const HISTORY: &str = r#"@version enum Versions {
    @rename("2026-04") v1, @rename("2026-07") v2, @rename("2026-10") v3
};"#;

fn contract(source: &str) -> (tempfile::TempDir, VersionDiff) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("api.idl");
    std::fs::write(&path, source).unwrap();
    let diff = VersionDiff::from_file(&path).unwrap();
    (dir, diff)
}

#[test]
fn distinct_versions_with_identical_contracts_have_no_diff() {
    let (_, diff) = contract(&format!(
        r#"
        #pragma xidlc package "example.api"
        {HISTORY}
        module models {{ struct Item {{ string id; }}; }};
        interface Store {{ @get(path = "/items") models::Item get(); }};
    "#
    ));
    for (from, to) in [("v1", "v2"), ("2026-04", "v3"), ("v2", "2026-07")] {
        assert_eq!(diff.compare(from, to).unwrap(), None);
    }
    let canonical = diff
        .formatted_projection(diff.record.resolve_version("v1").unwrap())
        .unwrap();
    assert!(canonical.contains("module models"));
    assert!(canonical.contains("#pragma xidlc package"));
    assert!(!canonical.contains("module v1"));
    assert!(!canonical.contains("#pragma xidlc version"));
}

#[test]
fn diff_preserves_wire_annotations_routes_and_parameter_locations() {
    let (_, diff) = contract(&format!(
        r#"
        {HISTORY}
        struct Item {{
            @removed(Versions::v2) string title;
            @added(Versions::v2) @optional @rename("title") string caption;
        }};
        interface Store {{
            @removed(Versions::v2) @get(path = "/items/{{id}}")
            Item get(@path string id);
            @added(Versions::v2) @get(path = "/items")
            Item get(@query string id);
        }};
    "#
    ));
    let rendered = diff.compare("v1", "v2").unwrap().unwrap();
    insta::assert_snapshot!("wire_contract_diff", rendered);
    assert_eq!(Some(rendered), diff.compare("2026-04", "2026-07").unwrap());
    assert_eq!(None, diff.compare("v2", "v3").unwrap());
    let reversed = diff.compare("v2", "v1").unwrap().unwrap();
    assert!(reversed.starts_with("--- api.idl @ v2 (2026-07)\n+++ api.idl @ v1 (2026-04)"));
    for side in ["v1", "v2"] {
        let error = diff.compare(side, "missing").unwrap_err().to_string();
        assert!(error.contains("v1 (2026-04), v2 (2026-07), v3 (2026-10)"));
    }
}

#[test]
fn imported_units_are_validated_and_their_binding_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("api.idl");
    std::fs::create_dir(dir.path().join("timestamp")).unwrap();
    std::fs::write(
        dir.path().join("timestamp/index.idl"),
        "@version enum V { v1, v2 }; struct Instant { long seconds; };",
    )
    .unwrap();
    std::fs::write(
        &root,
        format!(
            r#"
        import "timestamp@v1";
        {HISTORY}
        struct Item {{ timestamp::Instant created; @added(Versions::v2) string label; }};
    "#
        ),
    )
    .unwrap();
    let diff = VersionDiff::from_file(&root).unwrap();
    let source = diff
        .formatted_projection(diff.record.resolve_version("v1").unwrap())
        .unwrap();
    assert!(source.contains("import \"timestamp@v1\";"), "{source}");
    assert!(source.contains("timestamp::v1::Instant"), "{source}");
    assert!(diff.compare("v1", "v2").unwrap().is_some());

    for (import, reference, expected) in [
        ("missing", "Instant", "missing"),
        ("timestamp@v9", "Instant", "v9"),
        ("timestamp@v1", "Missing", "Missing"),
    ] {
        let prefix = import.split('@').next().unwrap();
        std::fs::write(
            &root,
            format!("import \"{import}\"; {HISTORY} struct S {{ {prefix}::{reference} value; }};"),
        )
        .unwrap();
        let error = VersionDiff::from_file(&root).err().unwrap().to_string();
        assert!(error.contains(expected), "{error}");
        assert!(error.contains("api.idl"), "{error}");
    }
}

#[test]
fn passthrough_annotations_ignore_trivia_and_preserve_quoted_values() {
    let original = format!(
        r#"{HISTORY}
        @rust-derive(Debug) @photon_audit(data=["x", "y"])
        struct Item {{ @rename("a\\b[0]") string id; @added(Versions::v2) string note; }};"#
    );
    let (_, first) = contract(&original);
    let reformatted = original
        .replace("Debug", " /* a */ Debug /* b */ ")
        .replace(
            r#"data=["x", "y"]"#,
            "data = [ /* values */ \"x\" , \"y\" ]",
        );
    let (_, second) = contract(&reformatted);
    assert_eq!(first.compare("v1", "v1").unwrap(), None);
    assert_eq!(
        first.compare("v1", "v2").unwrap(),
        second.compare("v1", "v2").unwrap()
    );
    let canonical = first
        .formatted_projection(first.record.resolve_version("v1").unwrap())
        .unwrap();
    assert!(canonical.contains(r#"@rename("a\\b[0]")"#), "{canonical}");
}

#[test]
fn invalid_and_unversioned_inputs_fail_without_a_diff() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("api.idl");
    for source in ["struct Plain { long id; };", "struct Broken {"] {
        std::fs::write(&path, source).unwrap();
        assert!(VersionDiff::from_file(&path).is_err());
    }
    std::fs::remove_file(&path).unwrap();
    assert!(
        VersionDiff::from_file(&path)
            .err()
            .unwrap()
            .to_string()
            .contains("api.idl")
    );
}
