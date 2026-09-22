use async_trait::async_trait;
use std::env;

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/websocket_upgrade.rs"));
}

struct MyWsControl;

#[async_trait]
impl gen::WsControl for MyWsControl {
    async fn control(
        &self,
        req: xidl_rust_axum::Request<
            xidl_rust_axum::stream::BidiServerStream<
                gen::WsControlControlRequest,
                gen::WsControlControlResponse,
            >,
        >,
    ) -> Result<(), xidl_rust_axum::Error> {
        let mut stream = req.into_inner();
        while let Some(item) = stream.read().await {
            let item = item?;
            stream
                .write(gen::WsControlControlResponse {
                    event: format!("{}:ack", item.opcode),
                    data: item.payload,
                })
                .await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);
    let svc = gen::WsControlServer::new(MyWsControl);
    println!("Rust websocket server starting on {}", addr);
    xidl_rust_axum::Server::builder()
        .with_service(svc)
        .serve(&addr)
        .await?;
    Ok(())
}
