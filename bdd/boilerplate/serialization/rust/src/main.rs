use async_trait::async_trait;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MySerializationTest;
#[async_trait]
impl gen::SerializationTest for MySerializationTest {
    async fn get_string<'a>(&'a self) -> Result<String, xidl_rust_axum::Error> { Ok("hello".to_string()) }
    async fn get_int<'a>(&'a self) -> Result<i32, xidl_rust_axum::Error> { Ok(42) }
    async fn get_bool<'a>(&'a self) -> Result<bool, xidl_rust_axum::Error> { Ok(true) }
    async fn get_struct<'a>(&'a self) -> Result<gen::Item, xidl_rust_axum::Error> { Ok(gen::Item { name: "world".to_string() }) }
    async fn echo_string<'a>(&'a self, value: String) -> Result<String, xidl_rust_axum::Error> { Ok(value) }
    async fn echo_struct<'a>(&'a self, value: gen::Item) -> Result<gen::Item, xidl_rust_axum::Error> { Ok(value) }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = gen::SerializationTestServer::new(MySerializationTest);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
