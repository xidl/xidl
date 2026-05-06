use xidlc_examples::city_rest::SmartCityRestApiServer;
use xidlc_examples::city_rest::SmartCityRestService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3001";
    println!("smart city http server listening on {addr}");

    xidl_rust_axum::Server::builder()
        .with_service(SmartCityRestApiServer::new(SmartCityRestService))
        .serve(addr)
        .await?;

    Ok(())
}
