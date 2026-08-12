use async_trait::async_trait;
use std::sync::Mutex;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyCalculator {
    history: Mutex<Vec<i32>>,
}
#[async_trait]
impl gen::Calculator for MyCalculator {
    async fn calculate<'a>(&'a self, req: gen::AddRequest, op: gen::Operation) -> Result<gen::AddResponse, xidl_jsonrpc::Error> {
        let result = match op { gen::Operation::ADD => req.a + req.b, gen::Operation::SUBTRACT => req.a - req.b };
        self.history.lock().unwrap().push(result);
        Ok(gen::AddResponse { result })
    }
    async fn get_history<'a>(&'a self) -> Result<Vec<i32>, xidl_jsonrpc::Error> { Ok(self.history.lock().unwrap().clone()) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder().with_service(gen::CalculatorServer::new(MyCalculator { history: Mutex::new(vec![]) })).with_endpoint(&format!("tcp://127.0.0.1:{}", port)).build().await?;
    server.serve().await?; Ok(())
}
