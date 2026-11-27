use super::*;

const FIRST: &str = "2026-12-01";
const SECOND: &str = "2027-06-01";
const CONTRACT: &str = r#"
@version enum V { @rename("2026-12-01") v1, @rename("2027-06-01") v2 };
struct Item { string id; @added(V::v2) string title; };
interface Items { @get(path = "/items") Item get(); };
"#;

#[test]
fn a_subset_replaces_unselected_entries_and_preserves_selected_fingerprints() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock_path = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, CONTRACT).unwrap();

    let both = VersionLock::from_file(&idl)
        .unwrap()
        .with_versions(vec![SECOND.into(), FIRST.into(), FIRST.into()])
        .write_lock(&lock_path, false)
        .unwrap();
    assert!(both.is_written());
    assert_eq!(both.document.versions.len(), 2);
    std::fs::write(&idl, CONTRACT.replace("string title;", "string heading;")).unwrap();

    let selected = VersionLock::from_file(&idl)
        .unwrap()
        .with_versions(vec![FIRST.into()]);
    assert!(selected.check(&lock_path, true).unwrap().is_clean());
    let first = selected.write_lock(&lock_path, false).unwrap();
    assert!(first.is_written());
    assert_eq!(first.document.versions.len(), 1);
    assert_eq!(
        first.document.versions[FIRST],
        both.document.versions[FIRST]
    );
    assert_eq!(
        VersionLock::read_document(&lock_path).unwrap(),
        first.document
    );
}

#[test]
fn a_subset_check_keeps_removals_and_rejects_unknown_values() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock_path = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, CONTRACT).unwrap();
    let saved = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock_path, false)
        .unwrap();
    assert!(saved.is_written());

    let without_second = CONTRACT.replace(", @rename(\"2027-06-01\") v2", "");
    let without_second = without_second.replace(" @added(V::v2) string title;", "");
    std::fs::write(&idl, without_second).unwrap();

    for version in [FIRST, SECOND] {
        let report = VersionLock::from_file(&idl)
            .unwrap()
            .with_versions(vec![version.into()])
            .check(&lock_path, false)
            .unwrap();
        assert_eq!(
            report.drifts,
            vec![LockDrift::Removed {
                version: SECOND.into(),
                id: "v2".into(),
            }]
        );
    }
    let unknown = VersionLock::from_file(&idl)
        .unwrap()
        .with_versions(vec!["unknown".into()]);
    assert!(unknown.check(&lock_path, false).is_err());
    assert!(unknown.write_lock(&lock_path, true).is_err());
    assert_eq!(
        VersionLock::read_document(&lock_path).unwrap(),
        saved.document
    );
}
