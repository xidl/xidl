use super::*;

#[test]
fn builtin_annotations_reparse_with_their_original_semantics() {
    let source = r#"
        @extensibility(FINAL) struct A {};
        @extensibility(APPENDABLE) struct B {};
        @extensibility(MUTABLE) struct C {};
        @autoid(SEQUENTIAL) struct D {};
        @data_representation(XML) struct E {};
        @verbatim("plain") struct G {};
        @verbatim(C, "language") struct H {};
        @verbatim(BEGIN_FILE, "placement") struct I {};
        @verbatim(C, BEGIN_FILE, "both") struct J {};
        @service(DDS) interface S {};
        @service(CORBA) interface T {};
        @service(*) interface U {};
        @topic struct K {};
        @topic("events") struct L {};
        @topic(DDS) struct M {};
        @topic("events", DDS) struct N {};
        @topic(*) struct O {};
    "#;
    for contract in source
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let typed =
            crate::parser::parser_text(contract).unwrap_or_else(|err| panic!("{contract}: {err}"));
        let spec = Specification::from(typed);
        let printed = IdlPrinter::render(&spec);
        let typed = crate::parser::parser_text(&printed)
            .unwrap_or_else(|err| panic!("rendered {printed}: {err}"));
        let reparsed = Specification::from(typed);
        assert_eq!(
            serde_json::to_value(&reparsed).unwrap(),
            serde_json::to_value(&spec).unwrap(),
            "{printed}"
        );
    }
}

#[test]
fn printer_renders_all_definition_and_interface_forms() {
    let source = r#"
        #pragma xidlc package "printer.coverage"
        #pragma custom value
        module outer {
            struct Parent { long base; };
            struct Forward;
            union ForwardUnion;
            enum State { @rename("ready_state") Ready, Idle };
            bitset Bits { bitfield<1, boolean> enabled; bitfield<2, octet> raw;
                bitfield<3, short> signed_value; bitfield<4, unsigned short> unsigned_value; };
            bitmask Flags { Read, @position(4) Write, };
            typedef long Count, Matrix[2][3];
            typedef string<8> Bounded;
            typedef sequence<long, 4> Numbers;
            typedef map<string, long, 8> Lookup;
            typedef fixed<10, 2> Money;
            typedef Pair<long, string> Generic;
            typedef struct Inline : Parent { string value; } InlineAlias;
            typedef enum InlineState { One, Two } InlineStateAlias;
            typedef union InlineChoice switch(long) { case 1: long value; default: string text; } InlineChoiceAlias;
            typedef bitset InlineBits : Bits { bitfield<1> bit; } InlineBitsAlias;
            typedef bitmask InlineFlags { First, @position(3) Last, } InlineFlagsAlias;
            native Handle;
            const long MAX = (1 + 2);
            const long NEGATIVE = -(-MAX);
            const long COMPLEMENT = ~(~MAX);
            const double RATIO = -1.25;
            const char LETTER = 'a';
            const wchar WIDE = L'w';
            const boolean ENABLED = true;
            const octet BYTE = 1;
            const string<8> LABEL = "ok";
            const wstring<8> WIDE_LABEL = L"wide";
            const fixed PRICE = 1.25;
            const long LIMIT = MAX;
            const sequence<long> VALUES = 0;
            exception Failure { @optional string detail; };
            interface Base;
            interface Service : ::outer::Base {
                @get(path = "/items/{id}") @rename("fetch")
                string fetch(@path long id, @query @optional string filter) raises(Failure);
                void ping(in long count, out string result, inout boolean flag);
                readonly attribute string status raises(Failure);
                attribute long value getraises(Failure) setraises(Failure);
                attribute long write_only setraises(Failure);
                typedef long Local;
                const long ANSWER = 42;
                exception Nested { long code; };
            };
        };
    "#;
    assert_semantic_roundtrip(&lower(source));
}

#[test]
fn escaped_annotation_and_pragma_values_preserve_lexical_text() {
    let spec = lower(
        r#"
        #pragma xidlc package "demo\\pkg"
        @rename("a\\b") struct A { string value; };
        struct B { @rename("a\\b[0]") string value; @rename("a\"[0]") string quoted; };
    "#,
    );
    assert_semantic_roundtrip(&spec);
}

#[test]
fn opaque_arguments_preserve_tokens_and_strings_but_ignore_trivia() {
    let spec = lower(
        r#"@rust-derive(Debug /* derive */, Clone)
        @relay_audit(data = [ "x\"[0]", 'y', "中文 😀", "a\\b" ], expr = a + +b)
        struct Item { string id; };"#,
    );
    assert_semantic_roundtrip(&spec);
    let printed = IdlPrinter::render(&spec);
    assert!(!printed.contains("derive */"), "{printed}");
    assert!(printed.contains("a + +b"), "{printed}");
    assert!(
        printed.contains(r#"["x\"[0]",'y',"中文 😀","a\\b"]"#),
        "{printed}"
    );
}

#[test]
fn pruning_a_recursive_back_edge_recomputes_projected_type_metadata() {
    let spec = lower(
        "@version enum V { v1, v2 }; struct A { B b; }; \
         struct B { @removed(V::v2) A a; };",
    );
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .unwrap()
        .unwrap();
    for (id, _) in record.timeline().iter() {
        assert_semantic_roundtrip(&record.project(&spec, id).unwrap());
    }
}

#[test]
fn version_diff_fixture_preserves_every_version_and_effective_field_id() {
    let spec = lower(include_str!(
        "../../../../../bdd/features/data/version_diff.idl"
    ));
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .unwrap()
        .unwrap();
    for (id, _) in record.timeline().iter() {
        assert_semantic_roundtrip(&record.project(&spec, id).unwrap());
    }
}

#[test]
fn five_version_messaging_contract_preserves_semantics_and_printing_is_idempotent() {
    let spec = lower(include_str!(
        "../../../../../bdd/features/data/relay_messaging/relay_messaging.idl"
    ));
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .unwrap()
        .unwrap();
    assert_eq!(record.timeline().iter().count(), 5);
    for (id, _) in record.timeline().iter() {
        assert_semantic_roundtrip(&record.project(&spec, id).unwrap());
    }
}

#[test]
fn removed_leading_members_preserve_effective_field_ids() {
    let spec = lower(
        r#"
        @version enum V { v1, v2 };
        struct S { @removed(V::v2) long old; long kept; };
        union U switch(long) { case 1: @removed(V::v2) long old; default: string kept; };
        bitmask B { @removed(V::v2) Old, Kept, };
    "#,
    );
    let record = crate::versioning::VersioningRecord::analyze(&spec)
        .unwrap()
        .unwrap();
    for (id, _) in record.timeline().iter() {
        let projected = record.project(&spec, id).unwrap();
        assert_semantic_roundtrip(&projected);
        if id.index() == 1 {
            let printed = IdlPrinter::render(&projected);
            assert!(printed.contains("@id(2)"), "{printed}");
            assert!(printed.contains("@position(1)"), "{printed}");
        }
    }
}

fn assert_semantic_roundtrip(spec: &Specification) {
    let printed = IdlPrinter::render(spec);
    let reparsed = lower(&printed);
    let mut expected = serde_json::to_value(spec).unwrap();
    let mut actual = serde_json::to_value(&reparsed).unwrap();
    remove_redundant_field_annotations(&mut expected);
    remove_redundant_field_annotations(&mut actual);
    assert_eq!(actual, expected, "{printed}");
    assert_eq!(IdlPrinter::render(&reparsed), printed);
}

fn remove_redundant_field_annotations(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(serde_json::Value::String(raw)) = object.get_mut("Raw") {
                *raw = crate::parser::raw::RawArguments::canonical(raw);
            }
            let id = object.get("field_id").and_then(serde_json::Value::as_u64);
            let position = object.get("position").and_then(json_u64);
            if let Some(serde_json::Value::Array(annotations)) = object.get_mut("annotations") {
                annotations.retain(|annotation| {
                    let explicit_position = annotation
                        .get("Position")
                        .and_then(|id| id.get("value"))
                        .and_then(json_u64);
                    if explicit_position.is_some() && explicit_position == position {
                        return false;
                    }
                    let explicit = annotation
                        .get("Id")
                        .and_then(|id| id.get("value"))
                        .and_then(serde_json::Value::as_str)
                        .and_then(|id| id.parse::<u64>().ok());
                    explicit.is_none() || explicit != id
                });
            }
            for value in object.values_mut() {
                remove_redundant_field_annotations(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                remove_redundant_field_annotations(value);
            }
        }
        _ => {}
    }
}

fn json_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse::<u64>().ok())
}
