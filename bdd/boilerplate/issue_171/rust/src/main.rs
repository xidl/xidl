use async_trait::async_trait;
pub mod issue_171 {
    pub use crate::gen::issue_171::*;
}
mod gen {
    include!("../{{MODULE_NAME}}.rs");
}
struct MyRepro;
#[async_trait]
impl issue_171::ReproService for MyRepro {
    async fn flattenAny<'a>(&'a self, payload: xidl_rust_axum::serde_json::Value) -> Result<(), xidl_rust_axum::Error> {
        if payload.get("foo").and_then(|v| v.as_str()) == Some("bar") {
            Ok(())
        } else {
            Err(xidl_rust_axum::Error::bad_request())
        }
    }
    async fn flattenStructWithAny<'a>(&'a self, payload: issue_171::StructWithAny) -> Result<(), xidl_rust_axum::Error> {
        if payload.field.get("foo").and_then(|v| v.as_str()) == Some("bar") {
            Ok(())
        } else {
            Err(xidl_rust_axum::Error::bad_request())
        }
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = issue_171::ReproServiceServer::new(MyRepro);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
