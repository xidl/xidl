use std::{collections::HashMap, path::Path};

use serde_json::Value;

const SOURCE: &str = include_str!("../../bdd/features/data/relay_messaging/relay_messaging.idl");
const VERSIONS: [&str; 5] = ["v2026_03", "v2026_05", "v2026_07", "v2026_09", "v2026_11"];

fn generate(target: &str, props: HashMap<String, Value>) -> Vec<xidlc::driver::File> {
    xidlc::driver::Generator::new(target.to_owned())
        .generate_from_idl(SOURCE, Path::new("relay_messaging.idl"), props)
        .expect("messaging artifacts")
}

#[test]
fn complete_openapi_history_keeps_errors_security_streams_and_wire_names() {
    let files = generate("openapi", HashMap::new());
    assert_eq!(files.len(), VERSIONS.len());
    for version in VERSIONS {
        let file = files
            .iter()
            .find(|file| file.path() == format!("openapi_relay_messaging.{version}.json"))
            .expect("one document for each declared version");
        let document: Value = serde_json::from_str(file.content()).unwrap();
        let paths = &document["paths"];
        let schemas = &document["components"]["schemas"];
        let message = &schemas[format!("{version}.Relay.Message")];
        let properties = message["properties"]
            .as_object()
            .expect("Message schema properties");
        assert!(!properties.contains_key("internal_trace"));
        assert!(file.content().contains("NotFound"));
        assert!(!file.content().contains("Relay-Version"));
        assert!(!file.content().contains("X-Api-Version"));
        let parameters = paths["/messages/{id}"]["patch"]["parameters"].as_array();
        let idempotency = parameters
            .into_iter()
            .flatten()
            .filter(|parameter| {
                parameter["in"] == "header" && parameter["name"] == "Idempotency-Key"
            })
            .collect::<Vec<_>>();
        if version >= "v2026_09" {
            let patch = &paths["/messages/{id}"]["patch"];
            assert_eq!(idempotency.len(), 1);
            assert_eq!(idempotency[0]["required"], false);
            assert_eq!(idempotency[0]["schema"]["type"], "string");
            assert!(patch["responses"].get("409").is_some());
            let block = &paths["/users/{id}/block"]["post"];
            assert_eq!(
                block["security"][0],
                serde_json::json!({"api_key-header-x-relay-key": []})
            );
            assert!(
                block["responses"]["429"]["headers"]
                    .get("Retry-After")
                    .is_some()
            );
            assert!(paths["/spaces/{space_id}/messages/watch"]["get"]["responses"]["200"]["content"]["text/event-stream"].get("itemSchema").is_some());
            assert!(
                schemas
                    .get(format!("{version}.Relay.Admin.Quota"))
                    .is_none()
            );
        } else {
            assert!(idempotency.is_empty());
            assert!(paths.get("/spaces/{space_id}/messages/watch").is_none());
            assert_eq!(
                paths["/users/{id}/block"]["post"]["security"][0],
                serde_json::json!({"http_bearer": []})
            );
        }
        if version >= "v2026_07" {
            assert!(
                paths["/uploads"]["post"]["requestBody"]["content"]["application/x-ndjson"]
                    .get("itemSchema")
                    .is_some()
            );
            assert_eq!(
                schemas[format!("{version}.Relay.UploadChunk")]["properties"]["index"]["format"],
                "int64"
            );
        }
        if version == "v2026_11" {
            assert!(paths.get("/poll").is_none());
            assert!(paths.get("/realtime").is_some());
            assert!(message["properties"].get("attachments").is_none());
        }
        insta::assert_json_snapshot!(format!("messaging_openapi_{version}"), document);
    }
}

#[test]
fn complete_version_map_is_stable_and_does_not_depend_on_selected_output_version() {
    let all = generate("version-map", HashMap::new());
    let selected = generate(
        "version-map",
        HashMap::from([("api_version".into(), "v2026_11".into())]),
    );
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].path(), "relay_messaging.version-map.json");
    assert_eq!(all[0].content(), selected[0].content());
    assert!(all[0].content().ends_with('\n'));
    insta::assert_snapshot!("messaging_version_map", all[0].content());
}
