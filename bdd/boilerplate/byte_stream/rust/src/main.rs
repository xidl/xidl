use async_trait::async_trait;
use futures_util::StreamExt;
use std::env;
use xidl_rust_axum::stream::{ByteStream, ByteReader};

mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/byte_stream.rs"));
}

struct ByteStreamServiceImpl;

#[async_trait]
impl gen::ByteStreamService for ByteStreamServiceImpl {
    async fn download_bytes<'a>(
        &'a self,
        _req: xidl_rust_axum::Request<()>,
    ) -> Result<ByteStream, xidl_rust_axum::Error> {
        let stream = futures_util::stream::iter(vec![
            Ok(axum::body::Bytes::from("hello ")),
            Ok(axum::body::Bytes::from("world")),
        ]);
        Ok(xidl_rust_axum::stream::boxed_bytes(stream))
    }

    async fn upload_bytes<'a>(
        &'a self,
        req: xidl_rust_axum::Request<ByteStream>,
    ) -> Result<String, xidl_rust_axum::Error> {
        let mut reader = ByteReader::new(req.into_inner());
        let mut result = String::new();
        while let Some(bytes) = reader.read().await {
            result.push_str(std::str::from_utf8(&bytes.unwrap()).unwrap());
        }
        Ok(result)
    }
}

#[tokio::main]
async fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("127.0.0.1:{}", port);

    let app = gen::ByteStreamServiceServer::new(ByteStreamServiceImpl).router();

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
