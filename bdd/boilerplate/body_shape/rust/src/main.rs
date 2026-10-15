use async_trait::async_trait;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/{{MODULE_NAME}}.rs"));
}

struct MyStore;

#[async_trait]
impl gen::Store for MyStore {
    async fn create<'a>(&'a self, item: gen::Item) -> Result<gen::Item, xidl_rust_axum::Error> {
        Ok(item)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let svc = gen::StoreServer::new(MyStore);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
