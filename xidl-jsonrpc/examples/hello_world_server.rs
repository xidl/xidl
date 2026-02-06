mod rpc;

use clap::Parser;
use rpc::{HelloWorld, HelloWorldServer};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4000")]
    addr: String,
}

struct HelloWorldImpl;

impl HelloWorld for HelloWorldImpl {
    fn sayHello(&self, name: String) -> Result<(), xidl_jsonrpc::Error> {
        eprintln!("Hello, {name}!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("jsonrpc hello_world server listening on {}", args.addr);
    xidl_jsonrpc::Server::builder()
        .with_service(HelloWorldServer::new(HelloWorldImpl))
        .serve_on(&args.addr)
        .await?;

    Ok(())
}
