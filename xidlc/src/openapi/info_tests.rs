use crate::openapi::Contact;

#[test]
fn contact_new() {
    let contact = Contact::new();
    assert!(contact.name.is_none());
    assert!(contact.url.is_none());
    assert!(contact.email.is_none());
}
