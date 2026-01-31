mod rpc;

use clap::Parser;
use rpc::{HelloWorld, HelloWorldServer};
use std::io::BufReader;
use std::net::TcpListener;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    eprintln!("jsonrpc hello_world server listening on {}", args.addr);
    let listener = TcpListener::bind(&args.addr)?;
    let (stream, peer) = listener.accept()?;
    eprintln!("client connected: {peer}");
    stream.set_nodelay(true)?;

    let reader = BufReader::new(stream.try_clone()?);
    let writer = stream;

    xidl_jsonrpc::Server::builder()
        .with_io(xidl_jsonrpc::Io::new(reader, writer))
        .with_service(HelloWorldServer::new(HelloWorldImpl))
        .serve()?;

    Ok(())
}
