use async_trait::async_trait;

pub mod gen {
    include!("../wire_strict.rs");
}
use gen::{Probe, Probes, ProbesClient, ProbesServer};

struct Echo;

#[async_trait]
impl Probes for Echo {
    async fn check(&self, probe: Probe) -> xidl_rust_axum::Result<Probe> {
        Ok(probe)
    }
    async fn find(&self, deep: bool, count: i32) -> xidl_rust_axum::Result<Probe> {
        Ok(Probe { id: "query".into(), locked: deep, minutes: count })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(ProbesServer::new(Echo))
            .serve_with_listener(listener)
            .await
    });
    let client = ProbesClient::new(format!("http://{address}"));
    assert_eq!(
        client.check(Probe { id: "p1".into(), locked: false, minutes: 7 }).await?,
        Probe { id: "p1".into(), locked: false, minutes: 7 }
    );
    assert_eq!(
        client.find(true, 5).await?,
        Probe { id: "query".into(), locked: true, minutes: 5 }
    );
    let raw = xidl_rust_axum::reqwest::Client::new();
    let refused = raw
        .post(format!("http://{address}/probe"))
        .header("content-type", "application/json")
        .body(r#"{"id":"p1","locked":"false","minutes":1}"#)
        .send()
        .await?;
    assert_eq!(refused.status().as_u16(), 400, "a string never becomes a boolean");
    server.abort();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    xidl_rust_axum::Server::builder()
        .with_service(ProbesServer::new(Echo))
        .serve_with_listener(listener)
        .await?;
    Ok(())
}
