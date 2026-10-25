use super::super::analyze;
use crate::versioning::map::{ChangeKind, VersionMap};

fn map(source: &str) -> VersionMap {
    let (spec, record) = analyze(source);
    crate::versioning::map::from_spec(&spec, &record, "replacement.idl").expect("version map")
}

#[test]
fn unchanged_scoped_alias_inputs_ignore_projection_module_names() {
    let map = map(r#"
        @version enum V { v1, v2, v3 };
        module filters {
            typedef string MediaType;
            typedef sequence<MediaType> MediaTypes;
            typedef MediaTypes Selection;
        };
        interface Catalog {
            @get(path="/catalog") void search(@query filters::Selection media);
        };
    "#);
    assert!(map.steps.iter().all(|step| step.operations.is_empty()));
}

#[test]
fn identical_endpoints_keep_distinct_member_and_parameter_declarations() {
    let source = r#"
        @version enum V { v1, v2, v3 };
        struct Item {
            string stable;
            @removed(V::v2) @id(7) string label;
            @added(V::v2) @id(7) string label;
            @removed(V::v2) @optional string note;
            @added(V::v2) @optional string note;
        };
        @http(400) exception Failure {
            @removed(V::v2) string reason;
            @added(V::v2) string reason;
        };
        interface Items {
            @post(path = "/items")
            Item send(@removed(V::v2) @query string token,
                      @added(V::v2) @query string token) raises(Failure);
        };
    "#;
    let map = map(source);
    assert_eq!(map.format, 2);
    let step = &map.steps[0];
    for (owner, name) in [("Item", "label"), ("Item", "note"), ("Failure", "reason")] {
        let member = &step.types[owner].members[name];
        assert_eq!(member.change, ChangeKind::Changed);
        let old = member.from.as_ref().expect("old declaration");
        let new = member.to.as_ref().expect("new declaration");
        assert_ne!(old["declaration_id"], new["declaration_id"]);
        let mut old_shape = old.clone();
        let mut new_shape = new.clone();
        old_shape.remove("declaration_id");
        new_shape.remove("declaration_id");
        assert_eq!(
            old_shape, new_shape,
            "the replacement is declaration identity alone"
        );
    }
    assert!(!step.types["Item"].members.contains_key("stable"));
    let operation = &step.operations["Items.send"];
    let old = operation.from.as_ref().expect("old operation");
    let new = operation.to.as_ref().expect("new operation");
    assert_eq!(
        old["declaration_id"], new["declaration_id"],
        "the operation itself survived"
    );
    assert_ne!(
        old["parameters"]["query:token"]["declaration_id"],
        new["parameters"]["query:token"]["declaration_id"]
    );
    assert!(
        map.steps[1].types.is_empty(),
        "a surviving declaration is unchanged"
    );
    assert!(map.steps[1].operations.is_empty());
    insta::assert_snapshot!(crate::versioning::map::to_json(&map).expect("canonical map"));
}

#[test]
fn owner_replacement_enum_union_and_attributes_preserve_source_identity() {
    let map = map(r#"
        @version enum V { v1, v2 };
        @removed(V::v2) struct Item { string value; };
        @added(V::v2) struct Item { string value; };
        enum State { stable, @removed(V::v2) changed, @added(V::v2) changed };
        union Choice switch(long) {
            case 0: @removed(V::v2) string text;
            case 0: @added(V::v2) string text;
        };
        interface Items {
            @removed(V::v2) attribute string status;
            @added(V::v2) attribute string status;
        };
    "#);
    let step = &map.steps[0];
    for (owner, member) in [("Item", "value"), ("State", "changed"), ("Choice", "text")] {
        let change = &step.types[owner].members[member];
        assert_eq!(change.change, ChangeKind::Changed);
        assert_ne!(
            change.from.as_ref().expect("from")["declaration_id"],
            change.to.as_ref().expect("to")["declaration_id"]
        );
    }
    for operation in ["Items.get_attribute_status", "Items.set_attribute_status"] {
        let change = &step.operations[operation];
        assert_ne!(
            change.from.as_ref().expect("from")["declaration_id"],
            change.to.as_ref().expect("to")["declaration_id"]
        );
    }
}

#[test]
fn declaration_tokens_ignore_trivia_and_cover_multideclarators_and_wire_names() {
    let source = r#"
        @version enum V { v1, v2 };
        module nested {
            struct Item {
                string stable, values[2];
                @removed(V::v2) @rename("old") string value;
                @added(V::v2) @rename("new") string value;
                @rename("left") string simultaneous;
                @rename("right") string simultaneous;
                @skip string visible;
                string visible;
            };
        };
    "#;
    let first = map(source);
    let second = map(&source.replace(';', "; /* comment */\n"));
    assert_eq!(first, second);
    assert_eq!(first.steps[0].types["nested::Item"].members.len(), 2);
}

#[test]
fn parameters_with_equal_raw_names_use_effective_http_bindings() {
    let cases = [
        (
            "@get(path=\"/x\")",
            "@query @rename(\"left\") string value, @query @rename(\"right\") string value",
            ["query:left", "query:right"],
        ),
        (
            "@get(path=\"/x\")",
            "@query string value, @header string value",
            ["query:value", "header:value"],
        ),
        (
            "@post(path=\"/x\")",
            "string value, @query string value",
            ["body:value", "query:value"],
        ),
        (
            "@get(path=\"/x/{value}\")",
            "string value, @query string value",
            ["path:value", "query:value"],
        ),
        (
            "@post(path=\"/x{?value}\")",
            "string value, @header string value",
            ["query:value", "header:value"],
        ),
        (
            "@get(path=\"/x\")",
            "inout string value, @query string value",
            ["body:value", "query:value"],
        ),
        (
            "@post(path=\"/x\")",
            "@query @optional string value, @body string value",
            ["query:value", "body:value"],
        ),
    ];
    for (route, parameters, keys) in cases {
        let source = format!(
            "@version enum V {{ v1, v2 }}; interface I {{ {route} void f({parameters}, @added(V::v2) @cookie string audit); }};"
        );
        let map = map(&source);
        let operation = map.steps[0].operations["I.f"].from.as_ref().expect("from");
        let parameters = operation["parameters"].as_object().expect("parameters");
        assert_ne!(
            parameters[keys[0]]["declaration_id"], parameters[keys[1]]["declaration_id"],
            "{source}"
        );
        if source.contains("@optional") {
            assert_eq!(parameters["query:value"]["optional"], true);
            assert_eq!(parameters["body:value"]["optional"], false);
        }
    }
}

#[test]
fn method_and_attribute_with_equal_names_keep_distinct_sources() {
    let map = map(r#"
        @version enum V { v1, v2 };
        interface I {
            attribute string status;
            @get(path="/custom") string status(@added(V::v2) @query string audit);
        };
    "#);
    assert_eq!(map.steps[0].operations.len(), 1);
    assert!(map.steps[0].operations.contains_key("I.status"));
}

#[test]
fn response_parameters_keep_identity_separate_from_equal_named_inputs() {
    let map = map(r#"
        @version enum V { v1, v2, v3 };
        interface I {
            @post(path="/x") void split(
                in string token,
                @removed(V::v2) out string token,
                @added(V::v2) out string token
            );
            @get(path="/y") void both(
                @removed(V::v2) inout string value,
                @added(V::v2) inout string value
            );
        };
    "#);
    let change = &map.steps[0].operations["I.split"];
    let old = change.from.as_ref().expect("old");
    let new = change.to.as_ref().expect("new");
    assert_eq!(
        old["parameters"]["body:token"]["declaration_id"],
        new["parameters"]["body:token"]["declaration_id"]
    );
    assert_ne!(
        old["response_parameters"]["body:token"][0]["declaration_id"],
        new["response_parameters"]["body:token"][0]["declaration_id"]
    );
    assert_ne!(
        old["parameters"]["body:token"]["declaration_id"],
        old["response_parameters"]["body:token"][0]["declaration_id"]
    );
    let both = map.steps[0].operations["I.both"]
        .from
        .as_ref()
        .expect("inout");
    assert_eq!(
        both["parameters"]["body:value"]["declaration_id"],
        both["response_parameters"]["body:value"][0]["declaration_id"]
    );
    assert!(map.steps[1].operations.is_empty());
}

#[test]
fn bitfield_source_identity_inherits_owner_lifecycle() {
    let map = map(r#"
        @version enum V { v1, v2, v3 };
        @removed(V::v2) bitset Flags { bitfield<1> enabled; };
        @added(V::v2) bitset Flags { bitfield<1> enabled; };
        bitset Stable { bitfield<1> visible; };
    "#);
    let change = &map.steps[0].types["Flags"].members["enabled"];
    assert_ne!(
        change.from.as_ref().expect("from")["declaration_id"],
        change.to.as_ref().expect("to")["declaration_id"]
    );
    assert!(!map.steps[0].types.contains_key("Stable"));
    assert!(map.steps[1].types.is_empty());
}

#[test]
fn repeated_response_headers_preserve_all_declarations_in_source_order() {
    let map = map(r#"
        @version enum V { v1, v2 };
        interface I {
            @post(path="/x") void collect(
                @header @rename("X-Tag") out sequence<string> z,
                @header @rename("X-Tag") out sequence<string> a,
                @added(V::v2) @query string audit
            );
        };
    "#);
    let descriptor = map.steps[0].operations["I.collect"]
        .from
        .as_ref()
        .expect("old");
    let headers = descriptor["response_parameters"]["header:X-Tag"]
        .as_array()
        .expect("multiple bindings");
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0]["name"], "z");
    assert_eq!(headers[1]["name"], "a");
    assert_ne!(headers[0]["declaration_id"], headers[1]["declaration_id"]);
}
