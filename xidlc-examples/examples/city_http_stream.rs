use clap::Parser;
use xidlc_examples::city_http_stream::{CityHttpStreamApiServer, CityHttpStreamService};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3002")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("city http stream server listening on {}", args.addr);
    xidl_rust_axum::Server::builder()
        .with_service(CityHttpStreamApiServer::new(CityHttpStreamService))
        .serve(&args.addr)
        .await?;

    Ok(())
}
