use std::fs;
use std::path::PathBuf;

fn api_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("api")
        .join(name)
}

#[test]
fn generated_city_http_openapi_includes_security_mappings() {
    let raw = fs::read_to_string(api_file("city_openapi.json")).expect("read city_openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse openapi json");

    let schemes = &doc["components"]["securitySchemes"];
    assert!(schemes.get("http-bearer").is_some());
    assert!(schemes.get("api-key-header-x-reserve-key").is_some());

    let probe = &doc["paths"]["/v1/parking/lots/{lot_id}"]["head"]["security"];
    assert_eq!(probe, &serde_json::json!([]));

    let reserve = &doc["paths"]["/v1/parking/lots/{lot_id}/reserve"]["post"]["security"];
    assert_eq!(
        reserve,
        &serde_json::json!([{ "api-key-header-x-reserve-key": [] }])
    );

    let profile = &doc["paths"]["/v1/citizens/{citizen_id}"]["get"]["security"];
    assert_eq!(profile, &serde_json::json!([{ "http-bearer": [] }]));
}

#[test]
fn generated_city_http_stream_openapi_includes_stream_security_mappings() {
    let raw = fs::read_to_string(api_file("city_http_stream_openapi.json"))
        .expect("read city_http_stream_openapi.json");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("parse openapi json");

    let schemes = &doc["components"]["securitySchemes"];
    assert!(schemes.get("http-basic").is_some());
    assert!(schemes.get("http-bearer").is_some());

    let alerts = &doc["paths"]["/alerts/{district}"]["get"]["security"];
    assert_eq!(alerts, &serde_json::json!([{ "http-basic": [] }]));

    let ticker = &doc["paths"]["/ticker"]["get"]["security"];
    assert_eq!(ticker, &serde_json::json!([]));

    let upload = &doc["paths"]["/assets/upload"]["post"]["security"];
    assert_eq!(upload, &serde_json::json!([{ "http-bearer": [] }]));
}
