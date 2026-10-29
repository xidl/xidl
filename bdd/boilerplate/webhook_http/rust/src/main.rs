use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::Mutex;

pub mod gen {
    include!("../webhook_http.rs");
}
use gen::notifications::{Event, Receipt, Rejected};
use gen::{Receiver, ReceiverClient, ReceiverReceiveError, ReceiverServer};

#[derive(Default)]
struct Invoices {
    deliveries: Mutex<BTreeMap<String, Event>>,
}

#[async_trait]
impl Receiver for Invoices {
    async fn accepted(&self) -> xidl_rust_axum::Result<i32> {
        let deliveries = self.deliveries.lock()
            .map_err(|_| xidl_rust_axum::Error::new(500, "delivery store unavailable"))?;
        i32::try_from(deliveries.len()).map_err(|_| xidl_rust_axum::Error::new(500, "delivery count exceeds long"))
    }

    async fn receive(&self, delivery_id: String, event: Event) -> Result<Receipt, ReceiverReceiveError> {
        let reject = |reason: &str| ReceiverReceiveError::Rejected(Rejected { reason: reason.into() });
        if event.invoice_id.is_empty() || event.amount < 0 {
            return Err(reject("invalid invoice"));
        }
        let mut deliveries = self.deliveries.lock()
            .map_err(|_| xidl_rust_axum::Error::new(500, "delivery store unavailable"))?;
        if deliveries.get(&delivery_id).is_some_and(|previous| previous != &event) {
            return Err(reject("delivery payload conflict"));
        }
        let receipt = Receipt {
            event_type: event.event_type.clone(),
            invoice_id: event.invoice_id.clone(),
            accepted: true,
        };
        deliveries.insert(delivery_id, event);
        Ok(receipt)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(ReceiverServer::new(Invoices::default()))
            .serve_with_listener(listener).await
    });
    let client = ReceiverClient::new(format!("http://{address}"));
    let event = Event { event_type: "arbitrary.sdk.value".into(), invoice_id: "sdk".into(), amount: 1, note: None };
    for _ in 0..2 {
        let receipt = client.receive("sdk-delivery".into(), event.clone()).await?;
        assert!(receipt.accepted);
        assert_eq!(receipt.event_type, "arbitrary.sdk.value");
    }
    assert_eq!(client.accepted().await?, 1);
    let refused = client.receive("sdk-delivery".into(), Event { amount: 2, ..event }).await;
    assert!(matches!(refused, Err(ReceiverReceiveError::Rejected(error)) if error.reason == "delivery payload conflict"));
    server.abort();

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    xidl_rust_axum::Server::builder()
        .with_service(ReceiverServer::new(Invoices::default()))
        .serve_with_listener(listener).await?;
    Ok(())
}
