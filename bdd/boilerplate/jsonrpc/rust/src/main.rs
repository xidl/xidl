use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyCalculator;
#[async_trait]
impl gen::Calculator for MyCalculator {
    async fn add<'a>(&'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> { Ok(a + b) }
    async fn subtract<'a>(&'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> { Ok(a - b) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator)).with_endpoint(&format!("tcp://127.0.0.1:{}", port)).build().await?;
    server.serve().await?; Ok(())
}
