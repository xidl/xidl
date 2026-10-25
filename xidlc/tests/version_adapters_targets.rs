#![cfg(feature = "gen-typescript-rest-adapters")]

use std::collections::HashMap;
use std::path::Path;

fn render(source: &str, name: &str) -> Vec<(String, String)> {
    xidlc::driver::Generator::new("typescript-rest-adapters".into())
        .generate_from_idl(source, Path::new(name), HashMap::new())
        .expect("independent adapter generation")
        .into_iter()
        .map(|file| (file.path().into(), file.content().into()))
        .collect()
}

#[test]
fn independent_typescript_target_snapshots_wire_validation_and_business_decisions() {
    let source = include_str!("../../bdd/features/data/versioned_adapters_business.idl");
    let files = render(source, "business.idl");
    assert_eq!(files, render(source, "business.idl"));
    assert!(files[0].1.contains("optionalChangeDowncastToV1"));
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "business.adapters.ts");
    insta::assert_snapshot!("independent_typescript_business_adapters", &files[0].1);
}

#[test]
fn transforms_target_snapshots_every_change_shape_of_the_catalogue() {
    let source = include_str!("../../bdd/features/data/versioned_transforms.idl");
    let files = render(source, "transforms.idl");
    assert_eq!(files, render(source, "transforms.idl"));
    assert_eq!(files.len(), 1);
    assert!(files[0].1.contains("routeProtocolDowncastToV20260401"));
    assert!(files[0].1.contains("profileUpcastFromV20260101"));
    insta::assert_snapshot!("independent_typescript_transforms_adapters", &files[0].1);
}

#[test]
fn events_target_snapshots_the_split_event_surface() {
    let source = include_str!("../../bdd/features/data/versioned_events.idl");
    let files = render(source, "events.idl");
    assert_eq!(files, render(source, "events.idl"));
    assert_eq!(files.len(), 1);
    assert!(files[0].1.contains("eventKindDowncastToV20260401"));
    assert!(files[0].1.contains("eventKindUpcastFromV20260401"));
    insta::assert_snapshot!("independent_typescript_events_adapters", &files[0].1);
}

#[test]
fn chain_target_snapshots_the_retirement_surface_of_the_booking_corpus() {
    let source = include_str!("../../bdd/features/data/versioned_chain.idl");
    let files = render(source, "chain.idl");
    assert_eq!(files, render(source, "chain.idl"));
    assert_eq!(files.len(), 1);
    insta::assert_snapshot!("independent_typescript_chain_adapters", &files[0].1);
}

#[test]
fn single_version_target_has_valid_identity_entries_without_adjacent_conversions() {
    let files = render(
        "@version enum V { only }; @http(400) exception Invalid { string reason; }; interface Store { @post(path=\"/items\") string save(@body string value) raises(Invalid); };",
        "single.idl",
    );
    insta::assert_snapshot!("single_version_typescript_adapters", &files[0].1);
}

#[test]
fn changed_source_declarations_require_decisions_even_with_identical_wire_shapes() {
    let files = render(
        r#"
        @version enum V { v1, v2 };
        struct Item { @removed(V::v2) string value; @added(V::v2) string value; };
        interface Store { @post(path="/items") Item save(@body Item item); };
        "#,
        "replacement.idl",
    );
    assert!(files[0].1.contains("itemUpcastFromV1"));
    assert!(files[0].1.contains("ignored$value"));
    assert!(files[0].1.contains("itemDowncastToV1"));
    insta::assert_snapshot!("identical_declaration_typescript_adapters", &files[0].1);
}

#[test]
fn inherited_requests_and_responses_share_member_decisions_and_storage_needs() {
    let source = include_str!("../../bdd/features/data/versioned_inheritance.idl");
    let files = render(source, "inheritance.idl");
    assert_eq!(
        files,
        render(
            &source
                .replace("geo::Identifier id", "Identifier id")
                .replace("geo::Anchor previous", "Anchor previous"),
            "inheritance.idl"
        )
    );
    insta::assert_snapshot!("inherited_typescript_address_adapters", &files[0].1);
}

#[test]
fn adapter_generation_rejects_unversioned_retired_and_incomplete_projected_inputs() {
    for (source, expected) in [
        ("struct Item { string id; };", "version_map"),
        (
            "@version enum V { v1,v2 }; interface Store { @removed(V::v2) @get(path=\"/retired\") string retired(); };",
            "removed operations",
        ),
        (
            "@version enum V { v1,v2 }; @removed(V::v2) interface Store { @get(path=\"/old\") string get(); }; @added(V::v2) interface Store { @get(path=\"/new\") string get(); };",
            "replaced operation",
        ),
        (
            "@version enum V { only }; interface Store { @server_stream @get(path=\"/events\") string events(); };",
            "stream and upgrade",
        ),
    ] {
        let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
            .generate_from_idl(source, Path::new("invalid.idl"), HashMap::new());
        let error = result
            .err()
            .expect("invalid adapter contract must fail")
            .to_string();
        assert!(error.contains(expected), "{expected}: {error}");
    }
    let source =
        "@version enum V { v1,v2 }; interface Store { @get(path=\"/items\") string get(); };";
    let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
        .generate_from_idl(
            source,
            Path::new("selected.idl"),
            HashMap::from([("api_version".into(), serde_json::json!("v1"))]),
        );
    assert!(
        result.is_err(),
        "incomplete HIR cannot consume a complete version map"
    );
}

#[test]
fn generated_symbols_reject_normalized_versions_scopes_and_cross_category_collisions() {
    for source in [
        r#"@version enum V { v1, V1 };
        interface Store { @get(path="/save") string save(); };"#,
        r#"@version enum V { only };
        module A_B { interface Store { @get(path="/one") string save(); }; };
        module A { module B { interface Store { @get(path="/two") long save(); }; }; };"#,
        r#"@version enum V { only };
        module A { struct B { string id; }; };
        struct A_B { long id; };
        interface Store { @get(path="/one") A::B first(); @get(path="/two") A_B second(); };"#,
        r#"@version enum V { only };
        struct StoreSaveResultToOnly { string id; };
        interface Store { @get(path="/save") StoreSaveResultToOnly save(); };"#,
    ] {
        let result = xidlc::driver::Generator::new("typescript-rest-adapters".into())
            .generate_from_idl(source, Path::new("collision.idl"), HashMap::new());
        let error = result
            .err()
            .expect("ambiguous symbols must fail")
            .to_string();
        assert!(error.contains("generated symbol"), "{error}");
        assert!(error.contains("ambiguous"), "{error}");
        assert!(error.contains("collision"), "{error}");
    }
}

#[test]
fn replaces_links_copy_mechanically_without_decisions_or_storage() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Message {
            string id;
            @removed(V::v2) string text;
            @added(V::v2) @replaces(text) string body;
            @removed(V::v2) @optional string subject;
        };
        interface Messages {
            @get(path = "/messages/{id}") Message get(@path string id);
            @post(path = "/messages") Message send(@body Message message);
        };"#;
    let files = render(source, "renamed.idl");
    let adapters = &files[0].1;
    assert!(
        adapters.contains("\"body\"]: converted1.value"),
        "the upcast copies the earlier value into the successor: {}",
        adapters
    );
    assert!(
        adapters.contains("\"text\"]: converted1.value"),
        "the downcast restores the earlier wire name: {}",
        adapters
    );
    assert!(
        !adapters.contains("MessageUpcastFromV1Decision"),
        "a rename needs no decision: {adapters}"
    );
    assert!(
        adapters.contains("\"Message\": { \"v2\": [], \"v1\": [\"subject\"] }"),
        "storage needs cover only the retired member: {adapters}"
    );
    assert!(
        !adapters.contains("messageText"),
        "a renamed member is not a storage candidate: {adapters}"
    );
}

#[test]
fn defaulted_additions_require_the_implied_value_in_the_upcast_decision() {
    let source = r#"
        @version enum V { v1, v2 };
        struct SendMessageRequest {
            string to;
            @added(V::v2) @optional @default(false) boolean fallback_to_sms;
            @added(V::v2) @optional string note;
        };
        struct Message { string id; };
        interface Messages {
            @post(path = "/messages") Message send(@body SendMessageRequest request);
        };"#;
    let files = render(source, "defaults.idl");
    let adapters = &files[0].1;
    assert!(
        adapters.contains("fallback_to_sms: boolean;"),
        "a defaulted addition is a required decision: {adapters}"
    );
    assert!(
        !adapters.contains("fallback_to_sms?:"),
        "the decision must not fall back to the new default: {adapters}"
    );
    assert!(
        !adapters.contains("note?: string;"),
        "an optional addition without a default stays absent: {adapters}"
    );
    assert!(
        adapters.contains("[\"fallback_to_sms\"]: decision.value[\"fallback_to_sms\"]"),
        "the conversion assigns the stated value: {adapters}"
    );
    // A removed member keeps deciding through OMIT, never through an implied
    // value: the defaulted rendering belongs to upcast additions only.
    let removals = render(
        r#"
        @version enum V { v1, v2 };
        struct SendMessageRequest {
            string to;
            @removed(V::v2) @optional @default(false) boolean legacy_flag;
        };
        struct Message { string id; };
        interface Messages {
            @post(path = "/messages") Message send(@body SendMessageRequest request);
            @get(path = "/messages/{id}") SendMessageRequest get(@path string id);
        };"#,
        "removals.idl",
    );
    let adapters = &removals[0].1;
    assert!(
        adapters.contains("legacy_flag: boolean | Omitted"),
        "the downcast decision keeps the OMIT escape hatch: {adapters}"
    );
    assert!(
        adapters.contains(
            "sendMessageRequestLegacyFlag(source: types.v2.SendMessageRequest): boolean | undefined;"
        ),
        "the stored reader keeps its absent case for a defaulted removal: {adapters}"
    );
    assert!(
        !adapters.contains("legacy_flag: boolean;"),
        "a removed member never states an implied value: {adapters}"
    );
}

#[test]
fn update_request_conversions_must_stay_operation_disjoint() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Item { @removed(V::v2) @optional string note; };
        struct ItemPatch { @removed(V::v2) @optional string note; };
        interface Store {
            @patch(path = "/a") Item first(@body ItemPatch patch);
            @patch(path = "/b") Item second(@body ItemPatch patch);
        };"#;
    let error = render_error(source, "shared.idl");
    assert!(error.contains("share the request conversion"), "{error}");
    assert!(error.contains("ItemPatch"), "{error}");
}

#[test]
fn update_current_never_reaches_paired_conversions_through_wrappers() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Item { @removed(V::v2) @optional string note; };
        struct Patch { @optional sequence<Item> items; };
        interface Store { @patch(path = "/a") Item update(@body Patch patch); };"#;
    let error = render_error(source, "wrapped.idl");
    assert!(
        error.contains("through a sequence, map, or optional member"),
        "{error}"
    );
}

#[test]
fn update_current_never_routes_through_alias_conversions() {
    let source = r#"
        @version enum V { v1, v2 };
        struct Item { @removed(V::v2) @optional string note; };
        typedef Item PatchItem;
        interface Store { @patch(path = "/a") Item update(@body PatchItem patch); };"#;
    let error = render_error(source, "alias.idl");
    assert!(error.contains("routes current through"), "{error}");
    assert!(error.contains("alias"), "{error}");
}

/// Render one source with the adapter target and return its generation error.
fn render_error(source: &str, name: &str) -> String {
    xidlc::driver::Generator::new("typescript-rest-adapters".into())
        .generate_from_idl(source, Path::new(name), HashMap::new())
        .err()
        .expect("invalid update contract must fail generation")
        .to_string()
}

#[test]
fn ordinary_targets_match_the_master_artifacts_byte_for_byte() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut targets = vec!["typescript-rest"];
    if cfg!(feature = "gen-rust-axum") {
        targets.push("rust-axum");
    }
    for target in targets {
        for fixture in [
            "versioned_adapters",
            "versioned_adapters_business",
            "versioned_rest",
            "wire_contract",
            "http_body_shape",
        ] {
            let baseline = root.join(format!("tests/adapter_baseline/{target}.{fixture}.json"));
            let expected: std::collections::BTreeMap<String, String> = serde_json::from_str(
                &std::fs::read_to_string(&baseline).expect("master baseline snapshot"),
            )
            .expect("baseline artifact manifest");
            let source = root.join(format!("../bdd/features/data/{fixture}.idl"));
            let generated = xidlc::driver::Generator::new(target.into())
                .generate_from_idl(
                    &std::fs::read_to_string(&source).expect("contract"),
                    &source,
                    HashMap::from([
                        ("enable_client".into(), serde_json::json!(true)),
                        ("enable_server".into(), serde_json::json!(true)),
                    ]),
                )
                .expect("ordinary generation")
                .into_iter()
                .map(|file| (file.path().to_string(), file.content().to_string()))
                .collect::<std::collections::BTreeMap<_, _>>();
            assert_eq!(
                generated, expected,
                "{target} artifacts for {fixture} differ from the pinned baseline"
            );
        }
    }
}
