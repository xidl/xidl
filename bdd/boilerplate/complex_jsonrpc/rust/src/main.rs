use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyCalculator;
#[async_trait]
impl gen::Calculator for MyCalculator {
    async fn calculate<'a>(&'a self, req: gen::AddRequest, op: gen::Operation) -> Result<gen::AddResponse, xidl_jsonrpc::Error> {
        let result = match op { gen::Operation::ADD => req.a + req.b, gen::Operation::SUBTRACT => req.a - req.b };
        Ok(gen::AddResponse { result })
    }
    async fn get_history<'a>(&'a self) -> Result<Vec<i32>, xidl_jsonrpc::Error> { Ok(vec![]) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator)).with_endpoint(&format!("tcp://127.0.0.1:{}", port)).build().await?;
    server.serve().await?; Ok(())
}
