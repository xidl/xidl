use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyHelloWorld;
#[async_trait]
impl gen::HelloWorldService for MyHelloWorld {
    async fn hello<'a>(&'a self) -> Result<String, xidl_rust_axum::Error> { Ok("Hello BDD".into()) }
    async fn echo<'a>(&'a self, msg: String) -> Result<String, xidl_rust_axum::Error> { Ok(msg) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = gen::HelloWorldServer::new(MyHelloWorld);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
