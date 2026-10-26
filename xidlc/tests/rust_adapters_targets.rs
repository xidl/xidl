#![cfg(feature = "gen-rust-axum-adapters")]

use std::{collections::HashMap, path::Path};

fn render(source: &str) -> Vec<(String, String)> {
    xidlc::driver::Generator::new("rust-axum-adapters".into())
        .generate_from_idl(source, Path::new("api.idl"), HashMap::new())
        .expect("independent Rust adapter generation")
        .into_iter()
        .map(|file| (file.path().into(), file.content().into()))
        .collect()
}

#[test]
fn rust_target_snapshots_recursive_wire_models_and_full_operation_chains() {
    let source = include_str!("../../bdd/features/data/versioned_adapters_rust_business.idl");
    let files = render(source);
    assert_eq!(files, render(source));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "api.adapters.rs");
    insta::assert_snapshot!("independent_rust_business_adapters", &files[0].1);
}

#[test]
fn rust_transforms_target_snapshots_every_change_shape_of_the_catalogue() {
    let source = include_str!("../../bdd/features/data/versioned_transforms_rust.idl");
    let files = render(source);
    assert_eq!(files, render(source));
    assert_eq!(files.len(), 1);
    assert!(
        files[0]
            .1
            .contains("route_protocol_downcast_to_v_2026_04_01")
    );
    assert!(files[0].1.contains("profile_upcast_from_v_2026_01_01"));
    insta::assert_snapshot!("independent_rust_transforms_adapters", &files[0].1);
}

#[test]
fn rust_events_target_snapshots_the_split_event_surface() {
    let source = include_str!("../../bdd/features/data/versioned_events_rust.idl");
    let files = render(source);
    assert_eq!(files, render(source));
    assert_eq!(files.len(), 1);
    assert!(files[0].1.contains("event_kind_downcast_to_v_2026_04_01"));
    assert!(files[0].1.contains("event_kind_upcast_from_v_2026_04_01"));
    insta::assert_snapshot!("independent_rust_events_adapters", &files[0].1);
}

#[test]
fn rust_chain_target_snapshots_storage_backed_retirement() {
    let source = include_str!("../../bdd/features/data/versioned_chain_rust.idl");
    let files = render(source);
    assert_eq!(files, render(source));
    assert_eq!(files.len(), 1);
    let binding = r#"@version(in = "header", name = "X-Booking-Version")"#;
    assert_eq!(source.matches(binding).count(), 1);
    for declaration in [
        r#"@version(in = "header", name = "X-Application-Version")"#,
        r#"@version(in = "query", name = "api_revision")"#,
        r#"@version(in = "cookie", name = "api_revision")"#,
    ] {
        let contract = source.replacen(binding, declaration, 1);
        assert_eq!(files, render(&contract), "{declaration}");
    }
    for policy in [
        "X-Booking-Version",
        "HeaderMap",
        "headers.get(",
        "resolve_version(",
    ] {
        assert!(!files[0].1.contains(policy), "application policy: {policy}");
    }
    let code = files[0].1.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(code.contains("pub fn decode_and_upcast_scheduler_book( version: Version,"));
    let dispatch = code
        .split("pub async fn dispatch_scheduler_book<")
        .nth(1)
        .expect("the booking dispatcher is generated");
    let (_, parameters) = dispatch.split_once(">(").expect("dispatcher parameters");
    assert!(parameters.starts_with(" version: Version,"));
    insta::assert_snapshot!("independent_rust_chain_adapters", &files[0].1);
}

#[test]
fn single_version_rust_target_keeps_typed_decoding_and_owned_identity_outcomes() {
    let files = render(
        r#"@version enum V { only };
        struct Item { string id; @optional string note; };
        @http(400) exception Invalid { string reason; };
        interface Store { @post(path="/items") Item save(@body Item item) raises(Invalid); };"#,
    );
    insta::assert_snapshot!("single_version_rust_adapters", &files[0].1);
}

#[test]
fn inherited_models_require_explicit_adapter_decisions() {
    let files = render(
        r#"@version enum V { v1, v2 };
        struct Base {
            string id;
            @added(V::v2) @optional string note;
        };
        struct Item : Base { string role; };
        interface Store { @get(path="/item") Item read(); };"#,
    );
    assert!(
        files[0].1.contains("item_downcast_to_v_1"),
        "{}",
        files[0].1
    );
}

#[test]
fn rust_target_rejects_public_layouts_without_an_explicit_construction_contract() {
    for source in [
        r#"@version enum V { only }; @derive(Ord, PartialOrd) enum Key { visible, @skip hidden }; typedef Key Alias; interface Store { @post(path="/values") string save(@body map<Alias,string> values); };"#,
        r#"@version enum V { only }; struct Item { @skip string hidden; }; interface Store { @get(path="/items") Item read(); };"#,
        r#"@version enum V { only }; enum State { visible, @skip hidden }; interface Store { @post(path="/state") State save(@body State state); };"#,
        r#"@version enum V { v1,v2 }; enum State { visible, @skip hidden }; interface Store { @post(path="/state") State save(@body State state); };"#,
        r#"@version enum V { only }; @http union U switch(ContentType) { case ContentType::Json: string visible; case ContentType::Text: @skip string hidden; }; interface Store { @post(path="/u") U save(@body U value); };"#,
        r#"@version enum V { v1,v2 }; @http union U switch(ContentType) { case ContentType::Json: string visible; case ContentType::Text: @skip string hidden; }; interface Store { @post(path="/u") U save(@body U value); };"#,
        r#"@version enum V { v1,v2 }; struct Item { @skip string hidden; }; interface Store { @added(V::v2) @get(path="/items") Item read(); };"#,
    ] {
        let error = xidlc::driver::Generator::new("rust-axum-adapters".into())
            .generate_from_idl(source, Path::new("invalid-layout.idl"), HashMap::new())
            .err()
            .expect("skipped physical model fields must not disappear silently")
            .to_string();
        assert!(error.contains("skipped model"), "{error}");
    }
    for (source, expected) in [
        (
            r#"@version enum V { v1,V1 }; interface Store { @get(path="/items") string read(); };"#,
            "ambiguous",
        ),
        ("struct Item { string id; };", "version_map"),
        (
            r#"@version enum V { v1,v2 }; interface Store { @removed(V::v2) @get(path="/old") string read(); };"#,
            "removed operations",
        ),
        (
            r#"@version enum V { only }; union U switch(long) { case 1: case 2: string value; }; interface Store { @post(path="/u") U save(@body U value); };"#,
            "multiple or default union",
        ),
        (
            r#"@version enum V { only }; union U switch(long) { default: string value; }; interface Store { @post(path="/u") U save(@body U value); };"#,
            "multiple or default union",
        ),
        (
            r#"@version enum V { only }; interface Store { @server_stream @get(path="/events") string events(); };"#,
            "stream and upgrade",
        ),
    ] {
        let error = xidlc::driver::Generator::new("rust-axum-adapters".into())
            .generate_from_idl(source, Path::new("invalid.idl"), HashMap::new())
            .err()
            .expect("unsupported contracts must produce generation errors")
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
}
