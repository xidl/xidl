use clap::Parser;
use xidlc_examples::example_services::SmartCityRpcService;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4001")]
    addr: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("smart city jsonrpc server listening on {}", args.addr);
    xidl_jsonrpc::Server::builder()
        .with_service(SmartCityRpcService)
        .serve_on(&args.addr)
        .await?;

    Ok(())
}
