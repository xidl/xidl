use super::SequenceWire;
use serde::{Deserialize, Serialize};

#[test]
fn maximum_is_inclusive_and_never_pads() {
    for length in 0..=8 {
        let input = (0..length).collect::<Vec<_>>();
        let json = serde_json::to_string(&input).unwrap();
        let decoded = serde_json::from_str::<SequenceWire<usize, 2>>(&json);
        assert_eq!(decoded.is_ok(), length <= 2);
        let wire: SequenceWire<_, 2> = input.into_iter().collect();
        let encoded = serde_json::to_string(&wire);
        assert_eq!(encoded.is_ok(), length <= 2);
        if length <= 2 {
            assert_eq!(encoded.unwrap(), json);
            assert_eq!(decoded.unwrap(), wire);
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    batches: SequenceWire<SequenceWire<u32, 2>, 2>,
}

#[test]
fn nested_bounds_hold_in_both_directions() {
    assert!(serde_json::from_str::<Envelope>(r#"{"batches":[[],[1,2]]}"#).is_ok());
    for json in [r#"{"batches":[[1,2,3]]}"#, r#"{"batches":[[],[],[]]}"#] {
        assert!(serde_json::from_str::<Envelope>(json).is_err(), "{json}");
    }
    let inner: SequenceWire<_, 2> = [1, 2, 3].into_iter().collect();
    let invalid = Envelope {
        batches: [inner].into_iter().collect(),
    };
    assert!(serde_json::to_string(&invalid).is_err());
    assert_eq!(SequenceWire::<u8, 2>::default().len(), 0);
    let error = SequenceWire::<_, 2>::try_from(vec![1, 2, 3]).unwrap_err();
    assert_eq!(error.actual, 3);
    assert_eq!(error.maximum, 2);
    assert!(error.to_string().contains("sequence length 3"));
}

#[test]
fn wire_collections_preserve_element_order_for_projection_and_empty_defaults() {
    let wire = SequenceWire::<_, 3>::try_from(vec!["invoice-b", "invoice-a"]).unwrap();
    let borrowed = (&wire).into_iter().copied().collect::<Vec<_>>();
    let owned = wire.into_iter().collect::<Vec<_>>();
    assert_eq!(borrowed, ["invoice-b", "invoice-a"]);
    assert_eq!(owned, borrowed);
    let empty = SequenceWire::<u8, 0>::default();
    assert_eq!(serde_json::to_string(&empty).unwrap(), "[]");
    assert!(SequenceWire::<_, 0>::try_from(vec![1]).is_err());
}
