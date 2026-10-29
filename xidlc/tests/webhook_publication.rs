#![cfg(feature = "gen-openapi")]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde_json::Value;
use xidlc::driver::Generator;
use xidlc::driver::{ArgsGenerate, Driver};

struct Contract;

impl Contract {
    fn render(source: &str, target: &str) -> BTreeMap<String, String> {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../bdd/features/data/webhook_http.idl");
        Generator::new(target.into())
            .generate_from_idl(source, &path, HashMap::new())
            .unwrap_or_else(|error| panic!("{target}: {error}"))
            .into_iter()
            .map(|file| (file.path().into(), file.content().into()))
            .collect()
    }

    fn document(source: &str) -> Value {
        let files = Self::render(source, "openapi");
        assert_eq!(files.len(), 1);
        serde_json::from_str(files.values().next().unwrap()).unwrap()
    }

    fn imported(source: &str, target: &str) -> BTreeMap<String, String> {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("events.idl"),
            r#"
            @version enum V { v1,v2 };
            struct Base { string id; @added(V::v2) @optional string note; };
            struct Event : Base { string event_type; };
        "#,
        )
        .unwrap();
        let input = root.path().join("api.idl");
        std::fs::write(&input, source).unwrap();
        let output = root.path().join("out");
        Driver::run(ArgsGenerate {
            lang: target.into(),
            out_dir: output.to_string_lossy().into_owned(),
            client: true,
            server: true,
            dry_run: false,
            mock: false,
            api_version: None,
            api_versions: None,
            openapi_version: None,
            files: vec![input],
        })
        .unwrap_or_else(|error| panic!("{target}: {error}"));
        std::fs::read_dir(output)
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read_to_string(path).unwrap(),
                )
            })
            .collect()
    }
}

#[test]
fn webhook_reuses_the_complete_operation_and_preserves_other_methods() {
    let source = include_str!("../../bdd/features/data/webhook_http.idl");
    let plain = Contract::document(&source.replace("@webhooks", ""));
    let marked = Contract::document(source);
    assert_eq!(
        marked["webhooks"]["Receiver.receive"]["post"],
        plain["paths"]["/events"]["post"]
    );
    assert_eq!(
        marked["paths"]["/events"]["get"],
        plain["paths"]["/events"]["get"]
    );
    assert!(marked["paths"]["/events"].get("post").is_none());
    assert_eq!(marked["components"], plain["components"]);
    assert_eq!(marked, Contract::document(source));
    insta::assert_json_snapshot!("mixed_http_webhooks", marked);
}

#[test]
fn authentication_encodings_and_responses_use_the_same_http_renderer() {
    let source = r#"
        @http(409) exception Rejected { @header(name="X-Retry") string retry; string reason; };
        @webhooks @http_bearer @request("urlencoded")
        @response("json") interface Receiver {
            @deprecated @put(path="/events")
            string receive(@query string region, @cookie string session,
                @body string event, @header(name="X-Receipt") out string receipt) raises(Rejected);
        };
    "#;
    let plain = Contract::document(&source.replace("@webhooks", ""));
    let marked = Contract::document(source);
    assert_eq!(
        marked["webhooks"]["Receiver.receive"]["put"],
        plain["paths"]["/events"]["put"]
    );
    assert_eq!(marked["components"], plain["components"]);
    assert_eq!(marked["paths"], serde_json::json!({}));
}

#[test]
fn versions_filter_publication_and_keep_the_fixed_old_contract() {
    let source = r#"
        @version(in="header", name="X-Version") enum V { v1,v2,v3 };
        struct Event { string event_type; @added(V::v3) @optional string note; };
        @webhooks interface Receiver {
            @added(V::v2) @post(path="/events") void receive(@body Event event);
            @added(V::v2) @removed(V::v3) @delete(path="/legacy") void legacy();
        };
    "#;
    let files = Contract::render(source, "openapi");
    let documents = files
        .iter()
        .map(|(name, text)| (name.clone(), serde_json::from_str::<Value>(text).unwrap()))
        .collect::<BTreeMap<_, _>>();
    assert!(
        documents["openapi_webhook_http.v1.json"]
            .get("webhooks")
            .is_none()
    );
    assert_eq!(
        documents["openapi_webhook_http.v2.json"]["webhooks"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        documents["openapi_webhook_http.v3.json"]["webhooks"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let earlier = Contract::render(
        &source.replace("@added(V::v3) @optional string note;", ""),
        "openapi",
    );
    for (name, original) in &files {
        if !name.ends_with(".v3.json") {
            assert_eq!(original, &earlier[name]);
        }
    }
    insta::assert_json_snapshot!("versioned_http_webhooks", documents);
}

#[test]
fn aliases_and_interface_markers_publish_once_per_operation() {
    let document = Contract::document(
        r#"
        @webhooks interface Receiver {
            @webhooks @post(path="/events") @post(path="/compat") void receive(@body string event);
            attribute string state;
        };
    "#,
    );
    assert_eq!(document["webhooks"].as_object().unwrap().len(), 3);
    assert_eq!(document["paths"], serde_json::json!({}));
}

#[test]
fn streaming_item_schemas_follow_the_publication_container() {
    let source = r#"
        @webhooks interface Receiver {
            @server_stream @get(path="/events") string watch();
            @client_stream @post(path="/ingest") void receive(@body string event);
        };
    "#;
    let marked = Contract::document(source);
    let plain = Contract::document(&source.replace("@webhooks", ""));
    assert_eq!(marked["openapi"], "3.2.0");
    for (name, path, method) in [("watch", "/events", "get"), ("receive", "/ingest", "post")] {
        assert_eq!(
            marked["webhooks"][format!("Receiver.{name}")][method],
            plain["paths"][path][method]
        );
    }
}

#[test]
fn publication_conflicts_and_unrepresentable_path_parameters_fail_explicitly() {
    for (source, expected) in [
        (
            "@webhooks interface Receiver { readonly attribute string state; @post(path=\"/other\") string get_attribute_state(); };",
            "duplicate webhook operationId",
        ),
        (
            "@webhooks interface Receiver { @post(path=\"/events/{id}\") void receive(@path string id); };",
            "Paths Object template",
        ),
        (
            "@webhooks interface Receiver { @head(path=\"/events\") string receive(); };",
            "HEAD",
        ),
    ] {
        let error = Generator::new("openapi".into())
            .generate_from_idl(source, Path::new("invalid.idl"), HashMap::new())
            .err()
            .expect("invalid publication")
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn imported_and_versioned_included_units_validate_marker_arguments() {
    let root = tempfile::tempdir().unwrap();
    let dependency = root.path().join("events.idl");
    std::fs::write(&dependency,
        "@version enum V { v1,v2 }; @webhooks(signature=\"standard\") interface Receiver { void receive(); };"
    ).unwrap();
    for source in ["#include \"events.idl@v1\"\n", "import \"events.idl@v1\";"] {
        let input = root.path().join("api.idl");
        std::fs::write(&input, source).unwrap();
        let error = Driver::run(ArgsGenerate {
            lang: "openapi".into(),
            out_dir: root.path().join("out").to_string_lossy().into_owned(),
            client: false,
            server: false,
            dry_run: false,
            mock: false,
            api_version: None,
            api_versions: None,
            openapi_version: None,
            files: vec![input],
        })
        .expect_err("dependency marker must be checked")
        .to_string();
        assert!(error.contains("takes no arguments"), "{source}: {error}");
    }
}

#[test]
fn ordinary_code_targets_keep_their_contract_and_dependencies() {
    let source = include_str!("../../bdd/features/data/webhook_http.idl");
    let plain = source.replace("@webhooks", "");
    let targets = [
        #[cfg(feature = "gen-rust-axum")]
        "rust-axum",
        #[cfg(feature = "gen-typescript-rest")]
        "typescript-rest",
        #[cfg(feature = "gen-go-rest")]
        "go-rest",
    ];
    for target in targets {
        assert_eq!(
            Contract::render(source, target),
            Contract::render(&plain, target),
            "{target}"
        );
    }
}

#[test]
fn version_bound_imports_and_inherited_types_keep_external_contracts() {
    let source = r#"
        import "events.idl@v1";
        @webhooks interface Receiver {
            @post(path="/events") events::Event receive(@body events::Event event);
        };
    "#;
    let plain = source.replace("@webhooks", "");
    for target in [
        #[cfg(feature = "gen-rust-axum")]
        "rust-axum",
        #[cfg(feature = "gen-typescript-rest")]
        "typescript-rest",
        #[cfg(feature = "gen-go-rest")]
        "go-rest",
    ] {
        assert_eq!(
            Contract::imported(source, target),
            Contract::imported(&plain, target),
            "{target}"
        );
    }
    let marked = Contract::imported(source, "openapi");
    let ordinary = Contract::imported(&plain, "openapi");
    let marked_api: Value = serde_json::from_str(&marked["openapi_api.json"]).unwrap();
    let ordinary_api: Value = serde_json::from_str(&ordinary["openapi_api.json"]).unwrap();
    let operation = &marked_api["webhooks"]["Receiver.receive"]["post"];
    assert_eq!(operation, &ordinary_api["paths"]["/events"]["post"]);
    assert!(operation.to_string().contains("openapi_events"));
    for (name, document) in &marked {
        if name != "openapi_api.json" {
            assert_eq!(document, &ordinary[name], "{name}");
        }
    }
}
