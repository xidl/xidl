use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyMath;
#[async_trait] impl gen::Math for MyMath { async fn add<'a>(&'a self, a: i32, b: i32) -> Result<i32, xidl_jsonrpc::Error> { Ok(a + b) } }
struct MyStore { last: std::sync::Mutex<String> }
#[async_trait] impl gen::Store for MyStore { async fn save<'a>(&'a self, value: String) -> Result<(), xidl_jsonrpc::Error> { *self.last.lock().unwrap() = value; Ok(()) } async fn last_value<'a>(&'a self) -> Result<String, xidl_jsonrpc::Error> { Ok(self.last.lock().unwrap().clone()) } }
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let server = xidl_jsonrpc::Server::builder()
        .with_service(gen::MathServer::new(MyMath))
        .with_service(gen::StoreServer::new(MyStore { last: std::sync::Mutex::new("".into()) }))
        .with_endpoint(&format!("tcp://127.0.0.1:{}", port))
        .build().await?;
    server.serve().await?; Ok(())
}
