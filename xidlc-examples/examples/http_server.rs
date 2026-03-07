use xidlc_examples::city_http::SmartCityHttpApiServer;
use xidlc_examples::example_services::SmartCityHttpService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3001";
    println!("smart city http server listening on {addr}");

    xidl_rust_axum::Server::builder()
        .with_service(SmartCityHttpApiServer::new(SmartCityHttpService))
        .serve(addr)
        .await?;

    Ok(())
}
