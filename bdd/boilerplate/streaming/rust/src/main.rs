use async_trait::async_trait;
use futures_util::stream::StreamExt;
mod gen { include!("../{{MODULE_NAME}}.rs"); }
struct MyStream;
#[async_trait]
impl gen::StreamingService for MyStream {
    async fn ticks<'a>(&'a self, req: xidl_rust_axum::Request<gen::StreamingServiceTicksRequest>) -> Result<xidl_rust_axum::stream::SseStream<i32>, xidl_rust_axum::Error> {
        let count = req.data.count;
        let s = futures_util::stream::iter(0..count).map(|i| Ok::<_, xidl_rust_axum::Error>(i));
        Ok(xidl_rust_axum::stream::boxed_sse(s))
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let svc = gen::StreamingServiceServer::new(MyStream);
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder().with_service(svc).serve(&format!("127.0.0.1:{}", port)).await?; Ok(())
}
