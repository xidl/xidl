mod rpc;

use rpc::{service, HelloWorld};

struct HelloWorldImpl;

impl HelloWorld for HelloWorldImpl {
    async fn sayHello(&self, name: String) -> Result<(), xidl_rust_axum::Error> {
        println!("Hello, {name}!");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:3000";
    println!("axum hello_world server listening on {addr}");

    xidl_rust_axum::Server::builder()
        .with_service(service(HelloWorldImpl))
        .serve(addr)
        .await?;

    Ok(())
}
