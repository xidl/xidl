use super::*;
use serde_json::json;

#[test]
fn extensions_builder() {
    let expected = json!("value");
    let extensions = ExtensionsBuilder::new()
        .add("x-some-extension", expected.clone())
        .add("another-extension", expected.clone())
        .build();

    let value = serde_json::to_value(&extensions).unwrap();
    assert_eq!(value.get("x-some-extension"), Some(&expected));
    assert_eq!(value.get("x-another-extension"), Some(&expected));
}

#[test]
fn extensions_from_iter() {
    let expected = json!("value");
    let extensions: Extensions = [
        ("x-some-extension", expected.clone()),
        ("another-extension", expected.clone()),
    ]
    .into_iter()
    .collect();

    assert_eq!(extensions.get("x-some-extension"), Some(&expected));
    assert_eq!(extensions.get("another-extension"), Some(&expected));
}
