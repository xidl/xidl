use serde_json::json;
use std::collections::BTreeMap;
use xidlc_examples::union_serde::{BoolTagValue, CharTagValue, IntTagValue, Tag, UnionValue};

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
    .err()
    .expect("payload/tag mismatch should fail");
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
