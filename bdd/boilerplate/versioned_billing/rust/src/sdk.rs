pub mod first {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/selected/v1/versioned_billing.rs"
    ));
}
pub mod current {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/selected/v2/versioned_billing.rs"
    ));
}

use current::v2;
use first::v1;

/// Exercise independently selected SDK artifacts against the running application.
pub async fn exercise(base: String) -> Result<(), Box<dyn std::error::Error>> {
    let http = xidl_rust_axum::reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let first = v1::InvoicesClient::with_http(&base, http.clone());
    let current = v2::InvoicesClient::with_http(&base, http);
    assert_eq!(v1::InvoicesClient::API_VERSION, "2025-01-01");
    assert_eq!(v2::InvoicesClient::API_VERSION, "2025-04-01");
    let before = current.audit("acme".into()).await?;
    let old = v1::Capture {
        invoice_id: "inv_sdk".into(),
        payment: v1::Payment {
            payment_intent: "pi_sdk_1".into(),
            amount: 400,
        },
    };
    let new = v2::Capture {
        invoice_id: "inv_sdk".into(),
        payment: v2::Payment {
            payment_intent: "pi_sdk_1".into(),
            amount: 400,
        },
    };
    let (original, replay) = tokio::join!(
        first.pay(old.clone(), "sdk-1".into(), "acme".into()),
        current.pay(new.clone(), "sdk-1".into(), "acme".into()),
    );
    let original = original?;
    let replay = replay?;
    assert_eq!(original.payment_intent.as_deref(), Some("pi_sdk_1"));
    assert_eq!(replay.amount_paid, 400);
    assert_eq!(replay.payments.len(), 1);
    assert_eq!(
        current.audit("acme".into()).await?.commits,
        before.commits + 1
    );
    let next = v1::Capture {
        invoice_id: "inv_sdk".into(),
        payment: v1::Payment {
            payment_intent: "pi_sdk_2".into(),
            amount: 600,
        },
    };
    for _ in 0..2 {
        let error = first
            .pay(next.clone(), "sdk-2".into(), "acme".into())
            .await
            .unwrap_err();
        assert!(
            matches!(error, v1::InvoicesPayError::Rejected(value) if value.type_path == "Invoice.payment_intent")
        );
    }
    let committed = current
        .pay(
            v2::Capture {
                invoice_id: "inv_sdk".into(),
                payment: v2::Payment {
                    payment_intent: "pi_sdk_2".into(),
                    amount: 600,
                },
            },
            "sdk-2".into(),
            "acme".into(),
        )
        .await?;
    assert_eq!(committed.amount_paid, 1000);
    assert_eq!(committed.payments.len(), 2);
    assert!(
        matches!(first.get("inv_sdk".into(), "acme".into()).await, Err(v1::InvoicesGetError::Rejected(value)) if value.type_path == "Invoice.payment_intent")
    );
    // Replaying the first payment uses its committed snapshot, not today's invoice.
    assert_eq!(
        first.pay(old, "sdk-1".into(), "acme".into()).await?,
        original
    );
    assert_eq!(
        current
            .pay(new.clone(), "sdk-1".into(), "acme".into())
            .await?,
        replay
    );
    let mut changed = new.clone();
    changed.payment.amount = 401;
    assert!(matches!(
        current.pay(changed, "sdk-1".into(), "acme".into()).await,
        Err(v2::InvoicesPayError::Conflict(_))
    ));
    let other_before = current.audit("other".into()).await?;
    let isolated = current.pay(new, "sdk-1".into(), "other".into()).await?;
    assert_eq!(isolated.amount_paid, 400);
    assert_eq!(
        current.audit("other".into()).await?.commits,
        other_before.commits + 1
    );
    assert!(first.list(0, "acme".into()).await?.is_empty());
    assert!(matches!(
        first.list(100, "acme".into()).await,
        Err(v1::InvoicesListError::Rejected(_))
    ));
    assert_eq!(
        current.audit("acme".into()).await?.commits,
        before.commits + 2
    );
    println!("PASS: independent Rust SDKs preserve replayed and refused committed outcomes");
    Ok(())
}
