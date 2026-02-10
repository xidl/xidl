use clap::Parser;
use xidlc_examples::hello_world_jsonrpc::{HelloWorld, HelloWorldServer};

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4000")]
    addr: String,
}

struct HelloWorldImpl;

impl HelloWorld for HelloWorldImpl {
    fn sayHello<'a>(
        &'a self,
        name: String,
    ) -> xidl_jsonrpc::BoxFuture<'a, Result<(), xidl_jsonrpc::Error>> {
        Box::pin(async move {
            eprintln!("Hello, {name}!");
            Ok(())
        })
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
