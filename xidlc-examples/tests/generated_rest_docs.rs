use std::fs;
use std::path::PathBuf;

fn api_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("api/rest/generated/")
        .join(name)
}

#[test]
fn generated_city_rest_openapi_includes_security_mappings() {
    let raw = fs::read_to_string(api_file("city_rest.json")).expect("read city_openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse openapi json");

    let schemes = &doc["components"]["securitySchemes"];
    assert!(schemes.get("http_bearer").is_some());
    assert!(schemes.get("api_key-header-x-reserve-key").is_some());

    let probe = &doc["paths"]["/v1/parking/lots/{lot_id}"]["head"]["security"];
    assert_eq!(probe, &serde_json::json!([]));

    let reserve = &doc["paths"]["/v1/parking/lots/{lot_id}/reserve"]["post"]["security"];
    assert_eq!(
        reserve,
        &serde_json::json!([{ "api_key-header-x-reserve-key": [] }])
    );

    let profile = &doc["paths"]["/v1/citizens/{citizen_id}"]["get"]["security"];
    assert_eq!(profile, &serde_json::json!([{ "http_bearer": [] }]));
}

#[test]
fn generated_city_rest_stream_openapi_includes_stream_security_mappings() {
    let raw = fs::read_to_string(api_file("city_rest_stream.json"))
        .expect("read city_rest_stream_openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse openapi json");

    let schemes = &doc["components"]["securitySchemes"];
    assert!(schemes.get("http_basic").is_some());
    assert!(schemes.get("http_bearer").is_some());

    let alerts = &doc["paths"]["/alerts/{district}"]["get"]["security"];
    assert_eq!(alerts, &serde_json::json!([{ "http_basic": [] }]));

    let ticker = &doc["paths"]["/ticker"]["get"]["security"];
    assert_eq!(ticker, &serde_json::json!([]));

    let upload = &doc["paths"]["/assets/upload"]["post"]["security"];
    assert_eq!(upload, &serde_json::json!([{ "http_bearer": [] }]));
}
