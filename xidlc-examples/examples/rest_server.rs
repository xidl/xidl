use xidlc_examples::rest_server::{RestServerServer, SimpleRestServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:8081";
    println!("axum hello_world server listening on {addr}");

    xidl_rust_axum::Server::builder()
        .with_service(RestServerServer::new(SimpleRestServer::new()))
        .serve(addr)
        .await?;

    Ok(())
}
