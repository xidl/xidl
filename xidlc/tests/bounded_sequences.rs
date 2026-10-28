#![cfg(all(
    feature = "gen-openapi",
    feature = "gen-rust-axum-adapters",
    feature = "gen-typescript-rest-adapters"
))]

use std::{collections::HashMap, fs, path::Path};
use xidlc::driver::{ArgsGenerate, Driver, Generator};

#[test]
fn billing_bounds_are_rendered_as_maxima_and_fixed_arrays_remain_exact() {
    let input =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data/bounded_sequences.idl");
    let source = fs::read_to_string(&input).unwrap();
    for (target, snapshot) in [
        ("rust-axum", "billing_rust"),
        ("typescript-rest", "billing_typescript"),
        ("openapi", "billing_openapi"),
    ] {
        let files = Generator::new(target.into())
            .generate_from_idl(
                &source,
                &input,
                HashMap::from([
                    ("enable_client".into(), true.into()),
                    ("enable_server".into(), true.into()),
                ]),
            )
            .expect("included billing module with expression bounds");
        let output = files
            .iter()
            .map(|file| format!("=== {} ===\n{}", file.path(), file.content()))
            .collect::<Vec<_>>()
            .join("\n");
        insta::assert_snapshot!(snapshot, output);
        if target == "typescript-rest" {
            assert!(output.contains(".max(2)"));
            assert!(output.contains(".length(2)"), "fixed arrays remain exact");
        }
        if target == "openapi" {
            let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
            let query = &document["paths"]["/query"]["get"]["parameters"][0]["schema"];
            assert_eq!(query["maxItems"], 2);
            assert!(query.get("minItems").is_none());
        }
    }
}

#[test]
fn adapter_wire_decoders_keep_the_same_bound_in_every_version() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Batch { sequence<long, 2> ids; };
        interface Batches { @post(path="/batches") Batch reserve(@body Batch batch); };
    "#;
    for (target, snapshot) in [
        ("rust-axum-adapters", "bounded_rust_adapters"),
        ("typescript-rest-adapters", "bounded_typescript_adapters"),
    ] {
        let files = Generator::new(target.into())
            .generate_from_idl(source, Path::new("batches.idl"), HashMap::new())
            .expect("versioned batch contract");
        let output = files[0].content();
        assert!(output.contains("v1") && output.contains("v2"));
        assert!(output.contains(".max(2)") || output.contains("sequence maximum length exceeded"));
        insta::assert_snapshot!(snapshot, output);
    }
}

#[test]
fn imported_constants_define_the_wire_bound_without_target_specific_evaluation() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("dep.idl"),
        "const unsigned long LIMIT = 2;",
    )
    .unwrap();
    fs::write(
        root.path().join("api.idl"),
        r#"
        import "dep.idl";
        interface Probe {
            @get(path="/batch") sequence<long, dep::LIMIT> batch();
        };
    "#,
    )
    .unwrap();
    for target in ["rust-axum", "typescript-rest", "openapi"] {
        let output = root.path().join(target);
        Driver::run_with_props(
            ArgsGenerate {
                lang: target.into(),
                out_dir: output.to_string_lossy().into_owned(),
                files: vec![root.path().join("api.idl")],
                client: true,
                server: true,
                dry_run: false,
                mock: false,
                api_version: None,
                api_versions: None,
                openapi_version: None,
            },
            HashMap::new(),
        )
        .expect("imported sequence bound resolves in shared semantics");
        let rendered = fs::read_dir(output)
            .unwrap()
            .map(|entry| fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        let expected = match target {
            "rust-axum" => "SequenceWire<i32, 2>",
            "typescript-rest" => ".max(2)",
            _ => "\"maxItems\": 2",
        };
        assert!(
            rendered.contains(expected),
            "{target} must retain the imported bound"
        );
    }
}
