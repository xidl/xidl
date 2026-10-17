use crate::gen::{v3, version_adapters::*};
use crate::hooks::Hooks;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn parcel() -> Value {
    json!({"id":"p1", "reference":"TRACK-1", "address":{"zip_code":"10001"},
        "stops":[{"zip_code":"20002"}], "by_tag":{"home":{"zip_code":"30003"}}, "fixed":[4,5]})
}

pub async fn run() {
    let hooks = Hooks::default();
    let input = decode_and_upcast_depot_ship(Version::V1, json!({"parcel":parcel()}), &hooks).unwrap();
    assert_eq!(input.parcel.tracking, "TRACK-1");
    assert_eq!(input.parcel.carrier, "postal");
    assert_eq!(input.parcel.audit, "checked");
    assert_eq!(input.parcel.fixed, [4,5]);
    assert_eq!(input.parcel.by_tag["home"].zip, "30003");
    assert!(input.parcel.note.is_none() && input.parcel.inspected.is_none() && input.parcel.address.zone.is_none());
    let mut invalid = Vec::new();
    let mut value = parcel(); value["carrier"] = json!("future"); invalid.push(value);
    let mut value = parcel(); value["address"]["zone"] = json!("future"); invalid.push(value);
    let mut value = parcel(); value["address"] = json!({"zip":"model-name"}); invalid.push(value);
    let mut value = parcel(); value["by_tag"]["home"]["zone"] = json!("future"); invalid.push(value);
    let mut value = parcel(); value["fixed"] = json!([1]); invalid.push(value);
    let mut value = parcel(); value["fixed"] = json!([1,"2"]); invalid.push(value);
    let mut value = parcel(); value["stops"][0] = json!({}); invalid.push(value);
    invalid.push(Value::Null);
    let hook_calls = hooks.calls.load(Ordering::SeqCst);
    let handler_calls = AtomicUsize::new(0);
    for value in invalid {
        let result = dispatch_depot_ship(Version::V1, json!({"parcel":value}), |_, _| async {
            handler_calls.fetch_add(1, Ordering::SeqCst); Ok(v3::State::pending)
        }, &hooks).await;
        assert!(matches!(result, Err(AdaptationError::Validation(_))));
    }
    assert_eq!(hooks.calls.load(Ordering::SeqCst), hook_calls);
    assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    let result = dispatch_depot_verify(Version::V1, json!({"parcel":parcel(), "actor":7}), |_, _| async {
        handler_calls.fetch_add(1, Ordering::SeqCst); Ok(v3::State::pending)
    }, &hooks).await;
    assert!(matches!(result, Err(AdaptationError::Validation(_))));
    assert_eq!(hooks.calls.load(Ordering::SeqCst), hook_calls);
    assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    let mut rejected = parcel(); rejected["reference"] = json!("reject-v2");
    let result = dispatch_depot_ship(Version::V1, json!({"parcel":rejected}), |_, _| async {
        handler_calls.fetch_add(1, Ordering::SeqCst); Ok(v3::State::pending)
    }, &hooks).await;
    assert!(matches!(result, Err(AdaptationError::Unsupported(_))));
    assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    assert!(decode_and_upcast_depot_state(Version::V1, json!({"state":"delivered"}), &hooks).is_err());
    let choice = decode_and_upcast_depot_choose(Version::V2, json!({"choice":{"tag":"1", "data":7}}), &hooks).unwrap();
    assert_eq!(*choice.choice.as_count(), 7);
    assert!(decode_and_upcast_depot_choose(Version::V1, json!({"choice":{"tag":"1", "data":7}}), &hooks).is_err());
    assert!(decode_and_upcast_depot_choose(Version::V2, json!({"choice":{"tag":"1", "data":7, "extra":1}}), &hooks).is_err());
    let contact = decode_and_upcast_depot_contact(Version::V1, json!({"contact":{"zip_code":"123", "team":"ops"}}), &hooks).unwrap();
    assert_eq!(contact.contact.address.zip, "123"); assert_eq!(contact.contact.tags["team"], "ops");
    assert!(decode_and_upcast_depot_contact(Version::V1, json!({"contact":{"zip_code":"123", "zone":"future"}}), &hooks).is_err());
    let extra = decode_and_upcast_depot_extra(Version::V1, json!({"extra":{"fixed":"x", "tag":"ok"}}), &hooks).unwrap();
    assert_eq!(extra.extra.metadata, json!({"tag":"ok"}));
    assert!(decode_and_upcast_depot_extra(Version::V1, json!({"extra":{"fixed":"x", "future":"v3"}}), &hooks).is_err());
    let node = decode_and_upcast_depot_inspect(Version::V1, json!({"node":{"name":"root", "children":[{"name":"leaf"}]}}), &hooks).unwrap();
    assert_eq!(node.node.children.unwrap()[0].name, "leaf");
    assert!(decode_and_upcast_depot_inspect(Version::V1, json!({"node":{"name":"root", "children":[{"name":"leaf", "unknown":1}]}}), &hooks).is_err());
    let index = decode_and_upcast_depot_index(Version::V1, json!({"index":{"addresses":{"-7":{"zip_code":"xyz"}}}}), &hooks).unwrap();
    assert_eq!(index.index.addresses[&-7].zip, "xyz");
    assert!(decode_and_upcast_depot_index(Version::V1, json!({"index":{"addresses":{"bad":{"zip_code":"xyz"}}}}), &hooks).is_err());
    assert!(decode_and_upcast_depot_index(Version::V1, json!({"index":{"addresses":{"1":{"zip_code":"a"}, "01":{"zip_code":"b"}}}}), &hooks).is_err());
    // Strictly matching the second untagged variant must preserve its extra field.
    let representation = decode_and_upcast_depot_representation(Version::V1, json!({"value":{"id":"x", "extra":"keep"}}), &hooks).unwrap();
    assert!(matches!(representation.value, v3::Representation::Text(v3::FullPayload { ref extra, .. }) if extra == "keep"));
    assert!(decode_and_upcast_depot_ambiguous(Version::V1, json!({"value":{"id":"ambiguous"}}), &hooks).is_err());
    let bytes = decode_and_upcast_depot_blob(Version::V1, json!({"data":[0,127,255]}), &hooks).unwrap();
    assert_eq!(bytes.data, [0,127,255]);
    assert!(decode_and_upcast_depot_blob(Version::V1, json!({"data":[256]}), &hooks).is_err());
    let bindings = decode_and_upcast_depot_bindings(Version::V1, json!({"q":"query", "h":"header"}), &hooks).unwrap();
    assert_eq!((bindings.q, bindings.h), ("query".into(), "header".into()));
    assert!(decode_and_upcast_depot_bindings(Version::V1, json!({"x":"ambiguous"}), &hooks).is_err());
    let optional = decode_and_upcast_depot_optional_change(Version::V1, json!({"value":{}}), &hooks).unwrap();
    assert!(optional.value.note.is_none());
    let head_calls = AtomicUsize::new(0);
    let result = dispatch_depot_head_only(Version::V1, json!({}), |_, _| async {
        head_calls.fetch_add(1, Ordering::SeqCst); Ok("head".into())
    }, &hooks).await;
    assert!(matches!(result, Err(AdaptationError::Unsupported(_))));
    assert_eq!(head_calls.load(Ordering::SeqCst), 0);
}
