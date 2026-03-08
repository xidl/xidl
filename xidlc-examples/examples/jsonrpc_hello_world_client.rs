use clap::Parser;
use xidlc_examples::hello_world_jsonrpc::{HelloWorld, HelloWorldClient};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4000")]
    addr: String,
    #[arg(long, default_value = "World")]
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let client = HelloWorldClient::builder()
        .with_endpoint(format!("tcp://{}", args.addr))
        .build()
        .await?;
    client.sayHello(args.name).await?;
    eprintln!("request sent");

    Ok(())
}
