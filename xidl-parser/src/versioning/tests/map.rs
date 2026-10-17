use super::{analyze, parse};

#[test]
fn version_map_snapshot_contains_only_adjacent_direct_facts() {
    let source = include_str!("../../../../bdd/features/data/versioned_rest.idl");
    let (spec, record) = analyze(source);
    let map = crate::versioning::map::from_spec(&spec, &record, "versioned_rest.idl")
        .expect("map should compute");
    insta::assert_snapshot!(crate::versioning::map::to_json(&map).expect("serialize map"));
}

#[test]
fn version_map_rejects_unversioned_contracts_at_the_call_boundary() {
    let spec = parse("struct Item { string id; };");
    assert!(
        crate::versioning::VersioningRecord::analyze(&spec)
            .expect("analysis should succeed")
            .is_none()
    );
}

#[test]
fn version_map_keeps_body_moves_and_declaration_details() {
    let source = r#"
        @version
        enum Versions { v1, v2 };

        @removed(Versions::v2) typedef long Count;
        @added(Versions::v2) typedef string Total;
        @added(Versions::v2) native Opaque;
        struct Item { string id; };

        interface Items {
            @post(path = "/items")
            string save(
                @removed(Versions::v2) @body string payload,
                @added(Versions::v2) @query string query
            );
        };
    "#;
    let (spec, record) = analyze(source);
    let map =
        crate::versioning::map::from_spec(&spec, &record, "edge.idl").expect("map should compute");
    let json = crate::versioning::map::to_json(&map).expect("map should serialize");
    assert!(
        json.contains("body:payload"),
        "body parameter must be retained"
    );
    assert!(
        json.contains("query:query"),
        "query parameter must be retained"
    );
    assert!(
        json.contains("declarators"),
        "typedef details must be retained"
    );
    assert!(
        json.ends_with('\n'),
        "canonical output must end with a newline"
    );
    insta::assert_snapshot!(json);
}

#[test]
fn version_map_walks_all_type_and_http_shapes() {
    let source = r#"
        @version
        enum Versions { v1, v2 };

        @added(Versions::v2) typedef long Count, Counts[3];
        @added(Versions::v2) typedef sequence<string<16>, 4> Names;
        @added(Versions::v2) typedef fixed<8, 2> Amount;
        native Opaque;
        struct Item {
            string id;
            @added(Versions::v2) string<12> label;
            @added(Versions::v2) Names names;
            @added(Versions::v2) Amount amount;
        };
        @http(400) exception Failure { string message; };
        enum Color { red, blue };
        union Choice switch (long) {
            case 0: string text;
            default: long number;
        };
        bitset Flags {
            bitfield<1> enabled;
            bitfield<2, short> code;
        };
        bitmask Mask { A, @position(3) B };

        interface Outer {
            typedef long Local;
            struct Nested { string value; };
            @http(401) exception NestedFailure { string reason; };
        };

        interface Items {
            @post(path = "/single")
            string single(@body string payload);

            @post(path = "/object")
            string object(@body string first, @body @optional string second);

            @client_stream
            @stream_codec("ndjson")
            @post(path = "/stream")
            string stream(string asset_id, sequence<octet> chunk);

            @upgrade(protocol = "websocket", codec = "json")
            @get(path = "/socket")
            void socket(in string input, out string output);

            @post(path = "/changed")
            string changed(@removed(Versions::v2) @body string old,
                           @added(Versions::v2) @query string next);
        };
    "#;
    let (spec, record) = analyze(source);
    let map = crate::versioning::map::from_spec(&spec, &record, "shapes.idl")
        .expect("map should compute");
    let step = &map.steps[0];
    let changed = step
        .operations
        .get("Items.changed")
        .expect("changed operation");
    let from = changed.from.as_ref().expect("from descriptor");
    let to = changed.to.as_ref().expect("to descriptor");
    let from_params = from.get("parameters").expect("body parameter");
    let to_params = to.get("parameters").expect("query parameter");
    assert!(from_params.get("body:old").is_some());
    assert!(to_params.get("query:next").is_some());

    let json = crate::versioning::map::to_json(&map).expect("map should serialize");
    assert!(json.contains("sequence<string<16>, 4>"));
    assert!(json.contains("fixed<8, 2>"));
    assert!(json.contains("string<12>"));
    assert!(json.contains("\"Counts\""));
    assert!(!json.contains("Counts[3]"));
    assert!(json.contains("\"lengths\""));
    assert!(json.contains("\"3\""));
}
