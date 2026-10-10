use super::ContentType;

#[test]
fn shared_discriminators_preserve_wire_values() {
    for (value, name, media) in [
        (ContentType::Json, "Json", "application/json"),
        (
            ContentType::OctetStream,
            "OctetStream",
            "application/octet-stream",
        ),
        (ContentType::Text, "Text", "text/plain"),
    ] {
        assert_eq!(value.idl_name(), name);
        assert_eq!(ContentType::from_idl_name(name), Some(value));
        assert_eq!(value.as_str(), media);
        let encoded = serde_json::to_value(value).expect("serialize shared discriminator");
        assert_eq!(encoded, media);
        assert_eq!(
            serde_json::from_value::<ContentType>(encoded).expect("restore discriminator"),
            value,
        );
    }
}

#[test]
fn unknown_representation_is_not_silently_coerced() {
    assert_eq!(ContentType::from_idl_name("Xml"), None);
    assert!(serde_json::from_str::<ContentType>(r#""application/xml""#).is_err());
}
