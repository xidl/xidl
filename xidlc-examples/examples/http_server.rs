use xidlc_examples::http_server::{HttpServerServer, SimpleHttpServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8081";
    println!("axum hello_world server listening on {addr}");

    xidl_rust_axum::Server::builder()
        .with_service(HttpServerServer::new(SimpleHttpServer::new()))
        .serve(addr)
        .await?;

    Ok(())
}
