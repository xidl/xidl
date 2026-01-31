mod rpc;

use clap::Parser;
use rpc::{HelloWorld, HelloWorldClient};
use std::net::TcpStream;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:4000")]
    addr: String,
    #[arg(long, default_value = "World")]
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let stream = TcpStream::connect(&args.addr)?;
    stream.set_nodelay(true)?;

    let reader = stream.try_clone()?;
    let writer = stream;

    let client = HelloWorldClient::new(reader, writer);
    client.sayHello(args.name)?;
    eprintln!("request sent");

    Ok(())
}
