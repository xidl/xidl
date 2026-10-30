#![cfg(all(
    feature = "gen-rust-axum",
    feature = "gen-go-rest",
    feature = "gen-typescript-rest",
    feature = "gen-openapi"
))]

use std::{collections::HashMap, path::Path};

#[test]
fn split_inventory_contract_pins_each_targets_wire_projection() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data/wire_probe.idl");
    let source = std::fs::read_to_string(&path).unwrap();
    for target in ["rust-axum", "go-rest", "typescript-rest", "openapi"] {
        let properties = HashMap::from([
            ("enable_client".into(), true.into()),
            ("enable_server".into(), true.into()),
        ]);
        let output = xidlc::driver::Generator::new(target.into())
            .generate_from_idl(&source, &path, properties)
            .unwrap()
            .into_iter()
            .map(|file| format!("=== {} ===\n{}\n", file.path(), file.content()))
            .collect::<String>();
        insta::assert_snapshot!(format!("inventory_wire_{target}"), output);
    }
}
