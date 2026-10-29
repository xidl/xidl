use crate::{BoundExceeded, BoundedMap, BoundedVec};
use std::collections::BTreeMap;

#[test]
fn vector_rejects_growth_without_changing_the_value() {
    let mut values = BoundedVec::<_, 2>::try_from(vec![10]).unwrap();
    values.try_push(20).unwrap();
    let error = values.try_push(30).unwrap_err();
    assert_eq!(
        error,
        BoundExceeded {
            collection: "sequence",
            actual: 3,
            maximum: 2
        }
    );
    assert_eq!(values.as_ref(), &[10, 20]);
    assert_eq!(error.to_string(), "sequence length 3 exceeds maximum 2");
    assert_eq!(values.pop(), Some(20));
    values.try_push(30).unwrap();
    values.as_mut_slice()[0] = 40;
    assert_eq!(
        values.clone().map(|value| value.to_string()).into_inner(),
        ["40", "30"]
    );
    let rebuilt = BoundedVec::try_from_iter((&values).into_iter().copied()).unwrap();
    assert_eq!(values, rebuilt);
    assert_eq!(values.clone().into_iter().collect::<Vec<_>>(), [40, 30]);
    assert_eq!(Vec::from(values), [40, 30]);
    assert!(BoundedVec::<_, 2>::try_from_iter([1, 2, 3]).is_err());
    assert!(BoundedVec::<_, 2>::try_from(vec![1, 2, 3]).is_err());
    let mut empty = BoundedVec::<i32, 0>::default();
    assert!(empty.try_push(1).is_err());
    assert!(empty.is_empty());
    assert_eq!(empty.pop(), None);
}

#[test]
fn map_capacity_counts_keys_and_allows_replacement_and_slot_reuse() {
    let mut values = BoundedMap::<String, _, 1>::new();
    assert_eq!(values.try_insert("eu".into(), 10), Ok(None));
    assert_eq!(values.try_insert("eu".into(), 20), Ok(Some(10)));
    let error = values.try_insert("us".into(), 30).unwrap_err();
    assert_eq!(error.collection, "map");
    assert_eq!(error.actual, 2);
    assert_eq!(values.len(), 1);
    assert_eq!(values.get("eu"), Some(&20));
    *values.get_mut("eu").unwrap() += 1;
    assert_eq!(values.remove("eu"), Some(21));
    values.try_insert("us".into(), 30).unwrap();
    assert_eq!(
        values
            .clone()
            .map_values(|value| value.to_string())
            .into_inner(),
        BTreeMap::from([("us".into(), "30".into())])
    );
    let rebuilt = BoundedMap::<_, _, 1>::try_from(BTreeMap::from([("us".into(), 30)])).unwrap();
    assert_eq!(values, rebuilt);
    assert_eq!(
        (&values)
            .into_iter()
            .map(|(key, value)| (key.as_str(), *value))
            .collect::<Vec<_>>(),
        [("us", 30)]
    );
    assert_eq!(
        values.clone().into_iter().collect::<Vec<_>>(),
        [("us".into(), 30)]
    );
    assert_eq!(BTreeMap::from(values), BTreeMap::from([("us".into(), 30)]));
    assert!(BoundedMap::<_, _, 1>::try_from(BTreeMap::from([(1, 2), (3, 4)])).is_err());
    assert!(BoundedMap::<_, _, 1>::try_from_iter([(1, 2), (3, 4)]).is_err());
    let repeated = BoundedMap::<_, _, 1>::try_from_iter([(1, 2), (1, 3)]).unwrap();
    assert_eq!(repeated.get(&1), Some(&3));
    let mut empty = BoundedMap::<i32, i32, 0>::default();
    assert!(empty.try_insert(1, 2).is_err());
    assert!(empty.is_empty());
    assert_eq!(empty.get_mut(&1), None);
    assert_eq!(empty.remove(&1), None);
}

#[test]
fn serde_preserves_wire_shapes_and_checks_every_nested_bound() {
    type Groups = BoundedMap<String, BoundedVec<i32, 2>, 2>;
    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
    struct Batch {
        days: [Groups; 2],
        retry: Option<BoundedVec<i32, 2>>,
    }
    let wire = r#"{"days":[{"eu":[1,2]},{}],"retry":[]}"#;
    let value: Batch = serde_json::from_str(wire).unwrap();
    assert_eq!(serde_json::to_string(&value).unwrap(), wire);
    assert_eq!(value.days[0].get("eu").unwrap().as_ref(), &[1, 2]);
    for invalid in [
        r#"{"days":[{"eu":[1,2,3]},{}],"retry":null}"#,
        r#"{"days":[{"eu":[],"us":[],"jp":[]},{}],"retry":null}"#,
        r#"{"days":[{},{}],"retry":[1,2,3]}"#,
        r#"{"days":[{}],"retry":null}"#,
        r#"{"days":[{},{}],"retry":["1"]}"#,
    ] {
        assert!(serde_json::from_str::<Batch>(invalid).is_err(), "{invalid}");
    }
    let repeated: BoundedMap<String, i32, 1> = serde_json::from_str(r#"{"eu":1,"eu":2}"#).unwrap();
    assert_eq!(repeated.get("eu"), Some(&2));
    let error = serde_json::from_str::<BoundedVec<i32, 2>>("[1,2,3,invalid]").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("sequence length 3 exceeds maximum 2")
    );
    assert!(
        serde_json::from_str::<BoundedVec<i32, 0>>("[]")
            .unwrap()
            .is_empty()
    );
    assert!(
        serde_json::from_str::<BoundedMap<String, i32, 0>>("{}")
            .unwrap()
            .is_empty()
    );
    assert!(
        serde_json::from_str::<BoundedVec<i32, 2>>("{}")
            .unwrap_err()
            .to_string()
            .contains("a sequence with at most 2 elements")
    );
    assert!(
        serde_json::from_str::<BoundedMap<String, i32, 2>>("[]")
            .unwrap_err()
            .to_string()
            .contains("a map with at most 2 entries")
    );
}
