// Each include is generated with --api-version, independently of server generation.
mod first {
    include!("../selected/v1/versioned_inheritance.rs");
}
mod middle {
    include!("../selected/v2/versioned_inheritance.rs");
}
mod head {
    include!("../selected/v3/versioned_inheritance.rs");
}
use serde_json::json;

pub async fn run(base: &str) {
    use xidl_rust_axum::reqwest::{
        header::{HeaderMap, HeaderValue},
        Client,
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        HeaderValue::from_static("Bearer test-acme"),
    );
    let http = Client::builder().default_headers(headers).build().unwrap();
    let first = first::v1::shipping::AddressesClient::with_http(base, http.clone());
    let middle = middle::v2::shipping::AddressesClient::with_http(base, http.clone());
    let head = head::v3::shipping::AddressesClient::with_http(base, http);
    let old = first.read("acme".into(), "a2".into()).await.unwrap();
    assert_eq!(old.base.base.id, "a2");
    let coordinates: first::v1::geo::MapPosition = old.base.coordinates.unwrap();
    assert_eq!(coordinates, [12.5, -7.25]);
    assert!(old.base.base.base.previous.as_ref().unwrap()[0]
        .previous
        .is_none());
    let parent = old.base.base.base.parent.as_deref().unwrap();
    assert_eq!(parent.checkpoint.as_deref(), Some("handover"));
    assert_eq!(
        parent.parent.as_deref().unwrap().checkpoint.as_deref(),
        Some("packed")
    );
    assert!(parent.parent.as_deref().unwrap().parent.is_none());
    let patch = serde_json::from_value(json!({"coordinates":[0,-90],"deliveryReference":"R-SDK","deliveryNote":"","line":"SDK street","countryCode":"CA","priority":0,"parent":{"checkpoint":"Rust reroute","parent":{"checkpoint":"Rust original"}}})).unwrap();
    let written = first
        .update("acme".into(), "a2".into(), patch)
        .await
        .unwrap();
    assert_eq!(written.base.base.delivery_reference, "R-SDK");
    assert_eq!(written.base.base.delivery_note.as_deref(), Some(""));
    assert_eq!(written.priority, 0);
    assert_eq!(written.base.coordinates, Some([0.0, -90.0]));
    assert_eq!(
        written.base.base.base.parent.as_deref().unwrap().checkpoint.as_deref(),
        Some("Rust reroute")
    );
    let current = head.read("acme".into(), "a2".into()).await.unwrap();
    assert_eq!(current.base.base.signature_required, Some(true));
    assert_eq!(current.base.street, "SDK street");
    assert_eq!(current.base.coordinates, written.base.coordinates);
    assert_eq!(
        current.base.base.base.parent.as_deref().unwrap().parent.as_deref().unwrap().checkpoint.as_deref(),
        Some("Rust original")
    );
    let patch =
        serde_json::from_value(json!({"street":"Head street","signatureRequired":false})).unwrap();
    head.update("acme".into(), "a2".into(), patch)
        .await
        .unwrap();
    let latest = head.read("acme".into(), "a2".into()).await.unwrap();
    let view = middle.read("acme".into(), "a2".into()).await.unwrap();
    assert_eq!(view.base.base.delivery_reference, "R-SDK");
    assert_eq!(view.base.base.delivery_note.as_deref(), Some(""));
    // Restating a narrow view preserves the invisible Head field and the facts.
    let patch = serde_json::from_value(
        json!({"street":view.base.street,"countryCodeV2":"CA","priority":view.priority}),
    )
    .unwrap();
    middle
        .update("acme".into(), "a2".into(), patch)
        .await
        .unwrap();
    assert_eq!(head.read("acme".into(), "a2".into()).await.unwrap(), latest);
    // A simultaneous narrow write cannot undo the newer caller's choice.
    let (newer, older) = tokio::join!(
        head.update(
            "acme".into(),
            "a2".into(),
            serde_json::from_value(json!({"signatureRequired":true})).unwrap()
        ),
        first.update(
            "acme".into(),
            "a2".into(),
            serde_json::from_value(json!({"priority":0})).unwrap()
        ),
    );
    newer.unwrap();
    older.unwrap();
    assert_eq!(
        head.read("acme".into(), "a2".into())
            .await
            .unwrap()
            .base
            .base
            .signature_required,
        Some(true)
    );
    let (left, right) = tokio::join!(
        first.update(
            "acme".into(),
            "a2".into(),
            serde_json::from_value(json!({"line":"Left street","deliveryReference":"LEFT"}))
                .unwrap()
        ),
        first.update(
            "acme".into(),
            "a2".into(),
            serde_json::from_value(json!({"line":"Right street","deliveryReference":"RIGHT"}))
                .unwrap()
        ),
    );
    let left = left.unwrap();
    let right = right.unwrap();
    assert_eq!(
        (
            left.base.line.as_str(),
            left.base.base.delivery_reference.as_str()
        ),
        ("Left street", "LEFT")
    );
    assert_eq!(
        (
            right.base.line.as_str(),
            right.base.base.delivery_reference.as_str()
        ),
        ("Right street", "RIGHT")
    );
    let latest = head.read("acme".into(), "a2".into()).await.unwrap();
    let patch = serde_json::from_value(json!({"countryCode":"Atlantis"})).unwrap();
    assert!(matches!(
        first.update("acme".into(), "a2".into(), patch).await,
        Err(first::v1::shipping::AddressesUpdateError::Projection(_))
    ));
    assert_eq!(head.read("acme".into(), "a2".into()).await.unwrap(), latest);
    assert!(matches!(
        first.read("acme".into(), "orphan".into()).await,
        Err(first::v1::shipping::AddressesReadError::Projection(_))
    ));
    assert!(head.read("acme".into(), "orphan".into()).await.is_ok());
    assert!(matches!(
        first.read("other".into(), "a1".into()).await,
        Err(first::v1::shipping::AddressesReadError::Forbidden(_))
    ));
    println!("PASS: independent Rust address SDKs");
}
