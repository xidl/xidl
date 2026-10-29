#![cfg(all(
    feature = "gen-openapi",
    feature = "gen-rust-axum",
    feature = "gen-typescript-rest",
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
        if target == "rust-axum" {
            assert!(output.contains("BoundedVec<i32, 2>"));
            assert!(output.contains("BoundedMap<String, InvoiceIds, 2>"));
            assert!(!output.contains("SequenceWire"));
            assert!(!output.contains("SequenceLimit"));
        }
        if target == "typescript-rest" {
            assert!(output.contains(".max(2)"));
            assert!(output.contains(".length(2)"), "fixed arrays remain exact");
            assert!(output.contains("models.billing.InvoiceIdsSchema"));
            assert!(!output.contains("models.InvoiceIdsSchema"));
        }
        if target == "openapi" {
            let document: serde_json::Value = serde_json::from_str(files[0].content()).unwrap();
            let query = &document["paths"]["/query"]["get"]["parameters"][0]["schema"];
            assert_eq!(query["maxItems"], 2);
            assert!(query.get("minItems").is_none());
            let regions = &document["components"]["schemas"]["billing.Regions"];
            assert_eq!(regions["maxProperties"], 2);
            assert_eq!(
                regions["additionalProperties"]["$ref"],
                "#/components/schemas/billing.InvoiceIds"
            );
            let header = &document["paths"]["/metadata"]["get"]["parameters"][1]["schema"];
            assert_eq!(header["maxProperties"], 2);
            assert_eq!(
                header["additionalProperties"]["$ref"],
                "#/components/schemas/billing.InvoiceIds"
            );
        }
    }
}

#[test]
fn plain_rust_models_do_not_depend_on_the_axum_runtime() {
    let source =
        "typedef sequence<long, 2> Ids; struct Batch { Ids ids; map<string, Ids, 2> groups; };";
    let files = Generator::new("rust".into())
        .generate_from_idl(source, Path::new("batch.idl"), HashMap::new())
        .expect("plain Rust model generation");
    let output = files[0].content();
    assert!(output.contains("Vec<i32>"));
    assert!(output.contains("BTreeMap<String, Ids>"));
    assert!(!output.contains("xidl_rust_axum"));
}

#[test]
fn unbounded_collections_do_not_require_checked_construction() {
    let source = r#"
        typedef sequence<long> Ids;
        interface Batches {
            @get(path="/metadata") void metadata(@header Ids ids, @optional @cookie Ids retry);
            @post(path="/upload") void upload(@body sequence<octet> bytes);
        };
    "#;
    let files = Generator::new("rust-axum".into())
        .generate_from_idl(
            source,
            Path::new("batches.idl"),
            HashMap::from([
                ("enable_client".into(), true.into()),
                ("enable_server".into(), true.into()),
            ]),
        )
        .expect("unbounded metadata aliases and raw bytes");
    assert!(!files[0].content().contains(".try_into()"));
}

#[test]
fn axum_dynamic_types_keep_the_runtime_owned_json_dependency() {
    let source = r#"
        interface Echo {
            @post(path="/echo") any echo(@body any value);
            @post(path="/object") Object object(@body Object value);
            @post(path="/base") ValueBase base(@body ValueBase value);
        };
    "#;
    let files = Generator::new("rust-axum".into())
        .generate_from_idl(source, Path::new("echo.idl"), HashMap::new())
        .expect("runtime JSON values remain usable without a direct serde_json dependency");
    let output = files[0].content();
    assert!(output.contains("xidl_rust_axum::serde_json::Value"));
    assert!(!output.contains(" ::serde_json::Value"));
}

#[test]
fn adapter_wire_decoders_keep_the_same_bound_in_every_version() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Batch { sequence<long, 2> ids; map<string, sequence<long, 2>, 2> groups; };
        interface Batches {
            @post(path="/batches") Batch reserve(@body Batch batch);
            @post(path="/upload") long upload(@body sequence<octet, 2> bytes);
        };
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
        assert!(output.contains(".max(2)") || output.contains("BoundedVec<i32, 2>"));
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
            "rust-axum" => "BoundedVec<i32, 2>",
            "typescript-rest" => ".max(2)",
            _ => "\"maxItems\": 2",
        };
        assert!(
            rendered.contains(expected),
            "{target} must retain the imported bound"
        );
    }
}
