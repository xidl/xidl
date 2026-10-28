#![cfg(feature = "gen-go")]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use xidlc::driver::{ArgsGenerate, Driver};

#[test]
fn imported_complements_use_idl_widths_and_omit_unused_packages() {
    let output = tempfile::tempdir().expect("Go complement output directory");
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../bdd/features/data/integer_complements/api.idl");
    Driver::run_with_props(
        ArgsGenerate {
            lang: "go".into(),
            out_dir: output.path().to_string_lossy().into_owned(),
            client: false,
            server: false,
            dry_run: false,
            mock: false,
            api_version: None,
            api_versions: None,
            openapi_version: None,
            files: vec![source],
        },
        HashMap::from([
            ("xidlc_version".into(), "0.0.1".into()),
            ("xidlc_shorthash".into(), "snapshot".into()),
        ]),
    )
    .expect("generate the imported Go constant contract");
    let api = fs::read_to_string(output.path().join("api.go"))
        .expect("Go importer folds constant expressions");
    assert!(!api.contains("\"masks\""));
    insta::assert_snapshot!("imported_complements", api);
}

#[test]
fn symbolic_complements_keep_go_operators_without_imported_declarations() {
    let files = xidlc::driver::Generator::new("go".into())
        .generate_from_idl(
            "const unsigned long VALUE = ~dep::MASK;",
            Path::new("api.idl"),
            HashMap::from([
                (
                    xidlc::units::IMPORT_PROPERTY.into(),
                    serde_json::json!([{"prefix":"dep","unit":"masks/dep.idl"}]),
                ),
                (xidlc::units::UNIT_PATH_PROPERTY.into(), "api.idl".into()),
            ]),
        )
        .expect("symbolic Go constant contract");
    let api = files.iter().find(|file| file.path() == "api.go").unwrap();
    assert!(api.content().contains("^masks.MASK"));
    assert!(api.content().contains("\"masks\""));
}
