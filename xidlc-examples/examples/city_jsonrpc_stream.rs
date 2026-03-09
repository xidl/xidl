use clap::Parser;
use xidlc_examples::city_jsonrpc_stream::{CityJsonrpcStreamApiServer, CityJsonrpcStreamService};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4002")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("city jsonrpc stream server listening on {}", args.addr);
    xidl_jsonrpc::Server::builder()
        .with_service(CityJsonrpcStreamApiServer::new(CityJsonrpcStreamService))
        .serve_on(&args.addr)
        .await?;

    Ok(())
}
