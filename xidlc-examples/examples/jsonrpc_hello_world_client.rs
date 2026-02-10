use clap::Parser;
use tokio::io::split;
use tokio::net::TcpStream;
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

    let stream = TcpStream::connect(&args.addr).await?;
    stream.set_nodelay(true)?;
    let (reader, writer) = split(stream);

    let client = HelloWorldClient::new(reader, writer);
    client.sayHello(args.name).await?;
    eprintln!("request sent");

    Ok(())
}
