use std::collections::HashMap;

fn generate_axum(source: &str) -> String {
    let files = xidlc::generate_from_source("axum", source, HashMap::new())
        .expect("axum generation should succeed");
    assert!(!files.is_empty(), "axum should emit at least one file");
    let mut out = String::new();
    for file in files {
        out.push_str(file.content());
        out.push('\n');
    }
    out
}

fn module_case_source() -> &'static str {
    r#"
#pragma xidlc package Demo API
#pragma xidlc version v1

module demo {

struct Point {
    int64 x;
    int64 y;
};

@no_security
interface Things {
    @get(path = "/points")
    Point get_point();

    @get(path = "/points/{id}")
    Point get_point_by_id(
        @path string id,
        @header @rename("If-None-Match") @optional string if_none_match,
        @cookie @rename("session_token") @optional string session_token,
        @header @rename("X-Tags") @optional sequence<string> tags,
        @cookie @rename("pref") @optional sequence<string> prefs
    );
};

};
"#
}

#[test]
fn axum_module_keeps_point_definition() {
    let output = generate_axum(module_case_source());

    assert!(
        output.contains("pub struct Point"),
        "module Point definition should be emitted:\n{output}"
    );
    assert!(
        output.contains("pub trait Things"),
        "module Things interface should be emitted:\n{output}"
    );
    assert_eq!(
        output.matches("pub mod demo").count(),
        1,
        "module demo should be emitted once in a single file:\n{output}"
    );
}

#[test]
fn axum_module_emits_point_before_things() {
    let output = generate_axum(module_case_source());
    let point = output.find("pub struct Point").expect("Point should exist");
    let things = output
        .find("pub trait Things")
        .expect("Things should exist");

    assert!(
        point < things,
        "Point definition should appear before Things interface:\n{output}"
    );
}

#[test]
fn axum_top_level_emits_point_before_things() {
    let output = generate_axum(
        r#"
#pragma xidlc package Demo API
#pragma xidlc version v1

struct Point {
    int64 x;
    int64 y;
};

@no_security
interface Things {
    @get(path = "/points")
    Point get_point();
};
"#,
    );
    let point = output.find("pub struct Point").expect("Point should exist");
    let things = output
        .find("pub trait Things")
        .expect("Things should exist");

    assert!(
        point < things,
        "top-level Point should appear before Things:\n{output}"
    );
}

#[test]
fn axum_client_unwraps_optional_header() {
    let output = generate_axum(module_case_source());

    assert!(
        output.contains("if let Some(if_none_match) = if_none_match {"),
        "client should unwrap optional header before insertion:\n{output}"
    );
}

#[test]
fn axum_client_unwraps_optional_cookie_and_multi_header() {
    let output = generate_axum(module_case_source());

    assert!(
        output.contains("if let Some(if_none_match) = if_none_match {"),
        "client should unwrap optional header:\n{output}"
    );
    assert!(
        output.contains("if let Some(session_token) = session_token {"),
        "client should unwrap optional cookie:\n{output}"
    );
    assert!(
        output.contains("if let Some(tags) = tags {"),
        "client should unwrap optional multi-header:\n{output}"
    );
    assert!(
        output.contains("if let Some(prefs) = prefs {"),
        "client should unwrap optional multi-cookie:\n{output}"
    );
}
