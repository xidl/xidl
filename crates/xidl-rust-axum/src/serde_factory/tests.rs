use super::*;

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Payload {
    name: String,
    age: u32,
}

#[test]
fn json_factory_round_trip() {
    let payload = Payload {
        name: "alice".to_string(),
        age: 3,
    };
    let bytes = SerializeFactory::new("application/json")
        .to_vec(&payload)
        .unwrap();
    let decoded: Payload = DeserializeFactory::new("application/json")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn form_factory_round_trip() {
    let payload = Payload {
        name: "alice".to_string(),
        age: 3,
    };
    let bytes = SerializeFactory::new("application/x-www-form-urlencoded")
        .to_vec(&payload)
        .unwrap();
    let decoded: Payload = DeserializeFactory::new("application/x-www-form-urlencoded")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, payload);
}

#[test]
fn plain_factory_round_trip() {
    let payload = "hello world".to_string();
    let bytes = SerializeFactory::new("text/plain")
        .to_vec(&payload)
        .unwrap();
    assert_eq!(bytes, b"hello world");
    let decoded: String = DeserializeFactory::new("text/plain")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, payload);

    let age = 42u32;
    let bytes = SerializeFactory::new("text/plain").to_vec(&age).unwrap();
    assert_eq!(bytes, b"42");
    let decoded: u32 = DeserializeFactory::new("text/plain")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, age);

    let opt_age: Option<u32> = None;
    let bytes = SerializeFactory::new("text/plain")
        .to_vec(&opt_age)
        .unwrap();
    assert_eq!(bytes, b"");
    let decoded: Option<u32> = DeserializeFactory::new("text/plain")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, None);
}

#[cfg(feature = "msgpack")]
#[test]
fn msgpack_factory_round_trip() {
    let payload = Payload {
        name: "alice".to_string(),
        age: 3,
    };
    let bytes = SerializeFactory::new("application/msgpack")
        .to_vec(&payload)
        .unwrap();
    let decoded: Payload = DeserializeFactory::new("application/msgpack")
        .from_slice(&bytes)
        .unwrap();
    assert_eq!(decoded, payload);
}
