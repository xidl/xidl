use clap::Parser;
use xidlc_examples::city_jsonrpc::{SmartCityRpcApiServer, SmartCityRpcService};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4001")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let server = xidl_jsonrpc::Server::builder()
        .with_service(SmartCityRpcApiServer::new(SmartCityRpcService::default()))
        .with_endpoint(format!("tcp://{}", args.addr))
        .build()
        .await?;
    eprintln!(
        "smart city jsonrpc server listening on {}",
        server.endpoint().unwrap_or("")
    );
    server.serve().await?;

    Ok(())
}
