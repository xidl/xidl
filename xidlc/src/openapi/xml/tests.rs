use super::Xml;

#[test]
fn xml_new() {
    let xml = Xml::new();

    assert!(xml.name.is_none());
    assert!(xml.namespace.is_none());
    assert!(xml.prefix.is_none());
    assert!(xml.attribute.is_none());
    assert!(xml.wrapped.is_none());
}
