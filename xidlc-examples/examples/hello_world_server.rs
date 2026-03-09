use xidlc_examples::hello_world::HelloWorld;
use xidlc_examples::hello_world::HelloWorldSayHelloRequest;
use xidlc_examples::hello_world::HelloWorldServer;

struct HelloWorldImpl;

#[async_trait::async_trait]
impl HelloWorld for HelloWorldImpl {
    async fn sayHello(
        &self,
        req: xidl_rust_axum::Request<HelloWorldSayHelloRequest>,
    ) -> Result<(), xidl_rust_axum::Error> {
        let HelloWorldSayHelloRequest { name } = req.data;
        println!("Hello, {}!", name);
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
