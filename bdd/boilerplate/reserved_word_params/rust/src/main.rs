use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/reserved_word_params.rs"
    ));
}

struct MyReservedWordService;

#[async_trait]
impl gen::ReservedWordService for MyReservedWordService {
    async fn get_monitor<'a>(
        &'a self,
        id: String,
        r#type: String,
    ) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("monitor:{id}:{type}"))
    }
    async fn search<'a>(&'a self, r#type: String) -> Result<String, xidl_rust_axum::Error> {
        Ok(format!("search:{type}"))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let svc = gen::ReservedWordServiceServer::new(MyReservedWordService);
    println!("Rust server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&addr)
        .await?;
    Ok(())
}
