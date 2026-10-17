//! End-to-end coverage for `import` compilation units.
//!
//! Each fixture is a directory of IDL files generated into a temporary output
//! directory, so the tests exercise import resolution, recursive generation and
//! type imports without touching the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xidlc::driver::{ArgsGenerate, Driver};

/// Creates an empty fixture directory unique to this test.
fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "xidlc-import-{name}-{}-{nanos}",
        std::process::id()
    ));
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

/// Generates `lang` for `files` into `out_dir`.
fn generate(lang: &str, out_dir: &Path, files: Vec<PathBuf>) -> Result<(), String> {
    Driver::run(ArgsGenerate {
        lang: lang.to_string(),
        out_dir: out_dir.to_string_lossy().into_owned(),
        client: false,
        server: false,
        dry_run: false,
        mock: false,
        api_version: None,
        api_versions: None,
        files,
    })
    .map_err(|err| err.to_string())
}

const TIMESTAMP_ENTRY: &str = r#"
#pragma xidlc package timestamp

@version
enum ApiVersions {
    @rename("1.0") v1,
    @rename("2.0") v2,
};

struct Instant {
    string at;
    @added(ApiVersions::v2)
    @optional string monotonic;
};
"#;

const APP_SOURCE: &str = r#"
#pragma xidlc package app

import "timestamp";
import "metrics";

struct Event {
    timestamp::v1::Instant at;
};
"#;

#[test]
fn an_imported_unit_is_generated_next_to_its_importer() {
    let root = temp_dir("rust");
    let out = root.join("out");
    let app = root.join("app.idl");
    write(&root.join("timestamp/index.idl"), TIMESTAMP_ENTRY);
    write(
        &root.join("metrics/index.idl"),
        "struct Counter { string name; };\n",
    );
    write(&app, APP_SOURCE);

    generate("rust", &out, vec![app]).expect("generation succeeds");

    let importer = fs::read_to_string(out.join("app.rs")).expect("app.rs");
    assert!(
        importer.contains("use crate::timestamp;"),
        "app.rs: {importer}"
    );
    assert!(
        importer.contains("pub at: timestamp::v1::Instant"),
        "app.rs: {importer}"
    );
    assert!(
        !importer.contains("use crate::metrics;"),
        "an unreferenced import adds no use statement: {importer}"
    );

    let unit = fs::read_to_string(out.join("timestamp.rs")).expect("timestamp.rs");
    assert!(unit.contains("pub mod v1"), "timestamp.rs: {unit}");
    assert!(unit.contains("pub mod v2"), "timestamp.rs: {unit}");
    assert!(
        unit.contains("pub monotonic: Option<String>"),
        "the imported unit keeps every version: {unit}"
    );
    assert!(
        out.join("metrics.rs").is_file(),
        "every imported unit is generated"
    );
}

#[test]
fn an_import_cycle_fails_the_run() {
    let root = temp_dir("cycle");
    let out = root.join("out");
    let app = root.join("app.idl");
    write(&root.join("other.idl"), "import \"app.idl\";\n");
    write(&app, "import \"other.idl\";\n");

    let err = generate("rust", &out, vec![app]).expect_err("cycle is rejected");
    assert!(err.contains("cyclic import"), "error: {err}");
}

#[test]
fn a_missing_import_fails_the_run() {
    let root = temp_dir("missing");
    let out = root.join("out");
    let app = root.join("app.idl");
    write(&app, "import \"nowhere.idl\";\n");

    let err = generate("rust", &out, vec![app]).expect_err("missing target is rejected");
    assert!(err.contains("nowhere.idl"), "error: {err}");
}
