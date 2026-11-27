use super::openapi::{FROZEN, ROOT, wire_lock};
use super::*;
use crate::version_lock::wire::WireContract;
use serde_json::{Value, json};

#[test]
fn a_default_force_write_refreshes_recorded_openapi() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, ROOT).unwrap();
    let original = wire_lock(&idl).write_lock(&lock, false).unwrap();

    std::fs::write(&idl, ROOT.replace("string code", "long code")).unwrap();
    let plain = VersionLock::from_file(&idl).unwrap();
    let rewritten = plain.write_lock(&lock, true).unwrap();
    assert!(rewritten.is_written());
    assert_ne!(
        rewritten.document.versions[FROZEN].openapi_fingerprint,
        original.document.versions[FROZEN].openapi_fingerprint
    );
    assert!(plain.check(&lock).unwrap().is_clean());
    assert!(wire_lock(&idl).check(&lock).unwrap().is_clean());
}

#[test]
fn adding_openapi_to_a_changed_contract_requires_force() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, ROOT).unwrap();
    let previous = VersionLock::from_file(&idl)
        .unwrap()
        .write_lock(&lock, false)
        .unwrap();

    std::fs::write(&idl, ROOT.replace("string code", "long code")).unwrap();
    let with_openapi = wire_lock(&idl);
    let blocked = with_openapi.write_lock(&lock, false).unwrap();
    assert!(matches!(
        blocked.drifts.as_slice(),
        [LockDrift::Changed { artifact: None, .. }]
    ));
    assert_eq!(
        VersionLock::read_document(&lock).unwrap(),
        previous.document
    );

    let rewritten = with_openapi.write_lock(&lock, true).unwrap();
    assert!(rewritten.is_written());
    assert!(
        VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .is_clean()
    );
    assert!(with_openapi.check(&lock).unwrap().is_clean());
}

#[test]
fn oauth_scope_names_contribute_to_wire_evidence_without_their_descriptions() {
    let mut document = json!({
        "security": [{"access": ["read"]}],
        "components": {"securitySchemes": {"access": {
            "type": "oauth2",
            "flows": {"clientCredentials": {
                "tokenUrl": "https://auth.example.com/token",
                "scopes": {"read": "Read items", "description": "Read descriptions"}
            }}
        }}}
    });
    let digest = |document: &Value| WireContract::fingerprint(document, |_, _| Ok(None)).unwrap();
    let original = digest(&document);
    let scopes = "/components/securitySchemes/access/flows/clientCredentials/scopes";
    *document.pointer_mut(scopes).unwrap() = json!({
        "read": "View available items",
        "description": "View descriptions",
    });
    assert_eq!(digest(&document), original);

    *document.pointer_mut(scopes).unwrap() = json!({"read": "View available items"});
    assert_ne!(digest(&document), original);
}

#[test]
fn idl_and_openapi_evidence_share_the_input_from_a_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let manifest = dir.path().join("xidl.toml");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, ROOT).unwrap();
    std::fs::write(
        &manifest,
        concat!(r#"name = "items""#, "\n", r#"version = "1.0.0""#, "\n"),
    )
    .unwrap();

    let written = wire_lock(&manifest).write_lock(&lock, false).unwrap();
    assert!(written.is_written());
    assert!(
        VersionLock::from_file(&idl)
            .unwrap()
            .check(&lock)
            .unwrap()
            .is_clean()
    );
    assert!(wire_lock(&idl).check(&lock).unwrap().is_clean());
}

#[test]
fn a_forced_algorithm_upgrade_recomputes_every_recorded_fingerprint() {
    let dir = tempfile::tempdir().unwrap();
    let idl = dir.path().join("items.idl");
    let lock = VersionLock::default_lock_path(&idl);
    std::fs::write(&idl, ROOT).unwrap();
    let written = wire_lock(&idl).write_lock(&lock, false).unwrap();
    let mut legacy = written.document.clone();
    legacy.algorithm_version = Some(LOCK_ALGORITHM_VERSION - 1);
    let entry = legacy.versions.get_mut(FROZEN).unwrap();
    entry.fingerprint = "old IDL fingerprint".into();
    entry.openapi_fingerprint = Some("old HTTP fingerprint".into());
    std::fs::write(&lock, serde_json::to_vec(&legacy).unwrap()).unwrap();

    let command = VersionLock::from_file(&idl).unwrap();
    let error = command.check(&lock).unwrap_err().to_string();
    assert!(error.contains("version lock --force"), "{error}");
    let rewritten = command.write_lock(&lock, true).unwrap();
    assert_eq!(rewritten.document, written.document);
    assert!(command.check(&lock).unwrap().is_clean());
}
