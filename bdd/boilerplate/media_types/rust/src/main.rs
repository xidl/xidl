use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyForm;
#[async_trait]
impl gen::FormService for MyForm {
    async fn submit<'a>(&'a self, name: String, age: i32) -> Result<String, xidl_rust_axum::Error> { Ok(format!("Received {name} age {age}")) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = gen::FormServiceServer::new(MyForm);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
