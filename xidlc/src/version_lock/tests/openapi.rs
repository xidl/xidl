use super::*;
use crate::version_lock::wire::WireContract;
use serde_json::{Value, json};
use std::collections::HashMap;
use xidl_parser::versioning::API_VERSION_PROPERTY;

pub(super) const FROZEN: &str = "2026-12-01";
pub(super) const ROOT: &str = r#"
@version enum V { @rename("2026-12-01") v1 };
struct Payload { string code; };
struct Item { string description; boolean deprecated; Payload data; };
interface Items { @get(path = "/items") Item get(); };
"#;

pub(super) fn wire_lock(path: &Path) -> VersionLock {
    VersionLock::from_file(path)
        .unwrap()
        .with_versions(Some([FROZEN.into()].into()))
        .with_artifact(Some(VersionArtifact::OpenApi))
}

#[test]
fn an_idl_only_lock_can_add_wire_checks_and_ignore_metadata_changes() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, ROOT).unwrap();
    let first = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert!(first.is_written());
    assert!(
        first.document.versions[FROZEN]
            .openapi_fingerprint
            .is_none()
    );
    assert!(matches!(
        wire_lock(&idl).check(&lock).unwrap().drifts.as_slice(),
        [LockDrift::OpenApiUnlocked { .. }]
    ));

    let upgraded = wire_lock(&idl).write_lock(&lock, false).unwrap();
    assert!(upgraded.is_written());
    let expected = upgraded.document.versions[FROZEN]
        .openapi_fingerprint
        .clone()
        .unwrap();
    let plain = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();
    assert_eq!(
        plain.document.versions[FROZEN].openapi_fingerprint.as_ref(),
        Some(&expected)
    );

    let documented = ROOT
        .replace(
            "struct Item { string description;",
            "struct Item {\n    /// More detail about the value.\n    @deprecated string description;",
        )
        .replace(
            "Payload data;",
            "\n    /// An attached value.\n    @deprecated Payload data;",
        );
    std::fs::write(&idl, &documented).unwrap();
    assert!(
        !VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .is_clean()
    );
    assert!(wire_lock(&idl).check(&lock).unwrap().is_clean());
    let refreshed = wire_lock(&idl).write_lock(&lock, false).unwrap();
    assert!(matches!(
        refreshed.drifts.as_slice(),
        [LockDrift::Changed { artifact: None, .. }]
    ));
    assert_eq!(
        VersionLock::read_document(&lock).unwrap(),
        upgraded.document
    );

    std::fs::write(
        &idl,
        documented.replace("string description", "long description"),
    )
    .unwrap();
    assert!(matches!(
        wire_lock(&idl).check(&lock).unwrap().drifts.as_slice(),
        [LockDrift::Changed {
            artifact: Some(VersionArtifact::OpenApi),
            ..
        }]
    ));
    assert!(matches!(
        VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .drifts
            .as_slice(),
        [
            LockDrift::Changed { artifact: None, .. },
            LockDrift::Changed {
                artifact: Some(VersionArtifact::OpenApi),
                ..
            }
        ]
    ));
}

#[test]
fn a_generator_only_change_is_detected_with_identical_idl() {
    let source = r#"
        @version enum V { @rename("2026-12-01") v1 };
        enum Kind { @rename("kind-a") alpha, beta };
        struct A { string x; };
        union U switch (Kind) {
            case Kind::alpha: A first;
            case Kind::beta: A second;
        };
        struct Holder { U u; Kind k; };
        interface Api { @get(path = "/h") Holder get(); };
    "#;
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("union.idl");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, source).unwrap();
    let written = wire_lock(&idl).write_lock(&lock, false).unwrap();
    assert!(written.is_written());
    let mut stored = written.document;
    let version = stored.versions.get_mut(FROZEN).unwrap();

    let artifacts = crate::generate_from_source(
        "openapi",
        source,
        HashMap::from([(API_VERSION_PROPERTY.into(), FROZEN.into())]),
    )
    .unwrap();
    assert_eq!(artifacts.len(), 1);
    let mut current: Value = serde_json::from_str(artifacts[0].content()).unwrap();
    let digest = |document: &Value| WireContract::fingerprint(document, |_, _| Ok(None)).unwrap();
    let wire = digest(&current);
    assert_eq!(version.openapi_fingerprint.as_ref(), Some(&wire));
    current["info"]["title"] = json!("A different document name");
    current["paths"]["/h"]["get"]["operationId"] = json!("renamed");
    current["paths"]["/h"]["get"]["tags"] = json!(["New tag"]);
    current["paths"]["/h"]["get"]["summary"] = json!("New summary");
    current["paths"]["/h"]["get"]["responses"]["200"]["content"]["application/json"]["examples"] =
        json!({"sample": {"value": {"u": null}}});
    assert_eq!(digest(&current), wire);

    let renamed = source
        .replace("struct Holder", "struct Envelope")
        .replace("Holder get()", "Envelope fetch()");
    let renamed_outputs = crate::generate_from_source(
        "openapi",
        &renamed,
        HashMap::from([(API_VERSION_PROPERTY.into(), FROZEN.into())]),
    )
    .unwrap();
    let renamed_document = serde_json::from_str(renamed_outputs[0].content()).unwrap();
    assert_eq!(digest(&renamed_document), wire);

    let tag = current
        .pointer_mut("/components/schemas/v1.U/oneOf/0/properties/tag/enum/0")
        .unwrap();
    assert_eq!(*tag, json!("kind-a"));
    *tag = json!("alpha");
    version.openapi_fingerprint = Some(digest(&current));
    std::fs::write(&lock, serde_json::to_vec(&stored).unwrap()).unwrap();

    for reloaded in [VersionLock::from_file(&idl).unwrap(), wire_lock(&idl)] {
        assert!(matches!(
            reloaded.check(&lock).unwrap().drifts.as_slice(),
            [LockDrift::Changed {
                artifact: Some(VersionArtifact::OpenApi),
                ..
            }]
        ));
    }
}

const IMPORTED_ROOT: &str = r#"
import "bridge@v1";
@version enum V { @rename("2026-12-01") v1 };
struct Item { bridge::Outer data; };
interface Api { @get(path = "/item") Item get(); };
"#;

const SHARED: &str = "struct Payload { long seconds; }; struct Unused { long count; };";

fn write_imports(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir.join("bridge/shared")).unwrap();
    std::fs::write(
        dir.join("bridge/index.idl"),
        r#"
        import "shared";
        @version enum TV { v1 };
        struct Outer { shared::Payload value; };
        "#,
    )
    .unwrap();
    std::fs::write(dir.join("bridge/shared/index.idl"), SHARED).unwrap();
    let idl = dir.join("api.idl");
    std::fs::write(&idl, IMPORTED_ROOT).unwrap();
    idl
}

#[test]
fn editing_a_schema_reached_through_imports_changes_the_wire_fingerprint() {
    let dir = tempfile::tempdir().unwrap();
    let idl = write_imports(dir.path());
    let lock = VersionLock::default_lock_path(&idl);
    assert!(
        wire_lock(&idl)
            .write_lock(&lock, false)
            .unwrap()
            .is_written()
    );
    std::fs::write(
        dir.path().join("bridge/shared/index.idl"),
        SHARED.replace("long seconds", "string seconds"),
    )
    .unwrap();
    assert!(matches!(
        wire_lock(&idl).check(&lock).unwrap().drifts.as_slice(),
        [LockDrift::Changed {
            artifact: Some(VersionArtifact::OpenApi),
            ..
        }]
    ));
}

#[test]
fn unreachable_imports_and_later_versions_do_not_change_the_wire_fingerprint() {
    let dir = tempfile::tempdir().unwrap();
    let idl = write_imports(dir.path());
    let lock = VersionLock::default_lock_path(&idl);
    assert!(
        wire_lock(&idl)
            .write_lock(&lock, false)
            .unwrap()
            .is_written()
    );

    std::fs::write(
        dir.path().join("bridge/shared/index.idl"),
        SHARED.replace("long count", "string count"),
    )
    .unwrap();
    assert!(wire_lock(&idl).check(&lock).unwrap().is_clean());

    std::fs::create_dir(dir.path().join("preview")).unwrap();
    std::fs::write(
        dir.path().join("preview/index.idl"),
        r#"
        struct Note { string text; };
        interface Preview {
            @get(path = "/same") string first();
            @get(path = "/same") string second();
        };
        "#,
    )
    .unwrap();
    let with_preview = IMPORTED_ROOT
        .replace(
            "import \"bridge@v1\";",
            "import \"bridge@v1\";\nimport \"preview\";",
        )
        .replace("v1 };", "v1, @rename(\"2027-06-01\") v2 };")
        .replace(
            "bridge::Outer data;",
            "bridge::Outer data; @added(V::v2) @optional preview::Note note;",
        );
    std::fs::write(&idl, with_preview).unwrap();
    assert!(wire_lock(&idl).check(&lock).unwrap().is_clean());
}

#[test]
fn an_external_ref_can_select_a_nondefault_import_version() {
    let dir = tempfile::tempdir().unwrap();
    let idl = write_imports(dir.path());
    let imported = dir.path().join("bridge/index.idl");
    let with_second = std::fs::read_to_string(&imported)
        .unwrap()
        .replace("enum TV { v1 };", "enum TV { v1, v2 };")
        .replace(
            "shared::Payload value;",
            "shared::Payload value; @added(TV::v2) string later;",
        );
    std::fs::write(&imported, &with_second).unwrap();
    std::fs::write(
        &idl,
        IMPORTED_ROOT.replace("bridge::Outer", "bridge::v2::Outer"),
    )
    .unwrap();
    let lock = VersionLock::default_lock_path(&idl);
    assert!(
        wire_lock(&idl)
            .write_lock(&lock, false)
            .unwrap()
            .is_written()
    );

    std::fs::write(&imported, with_second.replace("string later", "long later")).unwrap();
    assert!(matches!(
        wire_lock(&idl).check(&lock).unwrap().drifts.as_slice(),
        [LockDrift::Changed {
            artifact: Some(VersionArtifact::OpenApi),
            ..
        }]
    ));
}
