use serde_json::json;
use std::collections::BTreeMap;
use xidlc_examples::union_serde::{
    BareTagValue, BoolTagValue, CharTagValue, IntTagValue, Tag, UnionValue,
};

#[test]
fn generated_union_serializes_with_string_tag() {
    let value = UnionValue::new_case2(vec!["a".to_string(), "b".to_string()]);
    let encoded = serde_json::to_value(&value).expect("serialize generated union");
    assert_eq!(encoded, json!({"tag": "V2", "data": ["a", "b"]}));
}

#[test]
fn generated_union_deserializes_string_tag() {
    let decoded: UnionValue = serde_json::from_value(json!({
        "tag": "V3",
        "data": {
            "k": "v"
        }
    }))
    .expect("deserialize generated union");

    assert_eq!(decoded.tag(), &Tag::V3);
    assert_eq!(decoded.as_case3().get("k"), Some(&"v".to_string()));
}

#[test]
fn generated_union_rejects_mismatched_payload() {
    let err = serde_json::from_value::<UnionValue>(json!({
        "tag": "V1",
        "data": ["not", "a", "u8"]
    }))
    .expect_err("payload/tag mismatch should fail");
    assert!(err.to_string().contains("payload decode failed"));
}

#[test]
fn generated_union_roundtrips_map_case() {
    let mut data = BTreeMap::new();
    data.insert("lang".to_string(), "rust".to_string());
    let value = UnionValue::new_case3(data);
    let encoded = serde_json::to_string(&value).expect("serialize generated map case");
    let decoded: UnionValue =
        serde_json::from_str(&encoded).expect("deserialize generated map case");
    assert_eq!(decoded.tag(), &Tag::V3);
    assert_eq!(decoded.as_case3().get("lang"), Some(&"rust".to_string()));
}

#[test]
fn generated_int_tag_union_serializes_numeric_tags_as_strings() {
    let value = IntTagValue::new_two_or_three(7);
    let encoded = serde_json::to_value(&value).expect("serialize int tag union");
    assert_eq!(encoded, json!({"tag": "2", "data": 7}));
}

#[test]
fn generated_int_tag_union_deserializes_multi_label_tag() {
    let decoded: IntTagValue = serde_json::from_value(json!({
        "tag": "3",
        "data": 9
    }))
    .expect("deserialize int tag union");
    assert_eq!(decoded.tag(), &3);
    assert_eq!(decoded.as_two_or_three(), &9);
    assert_eq!(
        serde_json::to_value(&decoded).expect("re-serialize multi label tag"),
        json!({"tag": "3", "data": 9})
    );
}

#[test]
fn generated_int_tag_union_uses_default_tag_string() {
    let decoded: IntTagValue = serde_json::from_value(json!({
        "tag": "default",
        "data": true
    }))
    .expect("deserialize int tag default case");
    assert_eq!(decoded.tag(), &0);
    assert_eq!(decoded.as_fallback(), &true);
    assert_eq!(
        serde_json::to_value(&decoded).expect("serialize default case"),
        json!({"tag": "default", "data": true})
    );
}

#[test]
fn generated_bool_tag_union_uses_boolean_strings() {
    let yes = BoolTagValue::new_yes("ok".to_string());
    let no = BoolTagValue::new_no("nope".to_string());
    assert_eq!(
        serde_json::to_value(&yes).expect("serialize true case"),
        json!({"tag": "true", "data": "ok"})
    );
    assert_eq!(
        serde_json::to_value(&no).expect("serialize false case"),
        json!({"tag": "false", "data": "nope"})
    );
}

#[test]
fn generated_char_tag_union_roundtrips_char_string_tag() {
    let value = CharTagValue::new_why("because".to_string());
    let encoded = serde_json::to_string(&value).expect("serialize char tag union");
    let decoded: CharTagValue = serde_json::from_str(&encoded).expect("deserialize char tag union");
    assert_eq!(decoded.tag(), &'y');
    assert_eq!(decoded.as_why(), "because");
}

#[test]
fn generated_char_tag_union_uses_default_tag_string() {
    let decoded: CharTagValue = serde_json::from_value(json!({
        "tag": "default",
        "data": "fallback"
    }))
    .expect("deserialize char default case");
    assert_eq!(decoded.tag(), &'\0');
    assert_eq!(decoded.as_other(), "fallback");
}

#[test]
fn generated_bare_label_union_constructs_each_case_under_its_own_tag() {
    let first = BareTagValue::new_first("one".to_string());
    assert_eq!(first.tag(), &Tag::V1);
    assert!(first.is_first() && !first.is_second() && !first.is_third());
    assert_eq!(first.as_first(), "one");

    let second = BareTagValue::new_second(2);
    assert_eq!(second.tag(), &Tag::V2);
    assert!(second.is_second() && !second.is_first() && !second.is_third());
    assert_eq!(second.as_second(), &2);

    let third = BareTagValue::new_third(true);
    assert_eq!(third.tag(), &Tag::V3);
    assert!(third.is_third() && !third.is_first() && !third.is_second());
    assert_eq!(third.as_third(), &true);
}

#[test]
fn generated_bare_label_union_roundtrips_every_case_through_serde() {
    for (value, tag, data) in [
        (
            BareTagValue::new_first("one".to_string()),
            "V1",
            json!("one"),
        ),
        (BareTagValue::new_second(2), "V2", json!(2)),
        (BareTagValue::new_third(true), "V3", json!(true)),
    ] {
        let encoded = serde_json::to_value(&value).expect("serialize bare label case");
        assert_eq!(encoded, json!({"tag": tag, "data": data}));
        let decoded: BareTagValue =
            serde_json::from_value(encoded).expect("deserialize bare label case");
        assert_eq!(decoded.tag(), value.tag());
        assert_eq!(decoded, value);
    }
}

#[test]
fn generated_bare_label_union_clone_and_eq_follow_the_tag() {
    let second = BareTagValue::new_second(7);
    let cloned = second.clone();
    assert_eq!(cloned.tag(), &Tag::V2);
    assert_eq!(cloned.as_second(), &7);
    assert_eq!(cloned, second);
    assert_ne!(cloned, BareTagValue::new_second(8));
    assert_ne!(cloned, BareTagValue::new_first("7".to_string()));
}

#[test]
#[should_panic(expected = "assertion failed: self.is_first()")]
fn generated_bare_label_union_accessor_rejects_another_case() {
    let second = BareTagValue::new_second(2);
    let _ = second.as_first();
}
