mod imp;

use imp::HelloWorld;

use crate::imp::HelloWorldServer;

struct HelloWorldImpl;

#[async_trait::async_trait]
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
        .with_service(HelloWorldServer::new(HelloWorldImpl))
        .serve(addr)
        .await?;

    Ok(())
}
