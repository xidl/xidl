use async_trait::async_trait;

pub mod gen { include!("../{{MODULE_NAME}}.rs"); }
use gen::*;

struct Files;
#[async_trait]
impl EnvelopeApi for Files {
    async fn get_file(&self, key: String) -> Result<EnvelopeApiGetFileResponse, EnvelopeApiGetFileError> {
        let value = match key.as_str() {
            "missing" => return Err(EnvelopeApiGetFileError::NotFound(NotFound { code: 404, msg: "no such file".into() })),
            "raw" => FileResponse::OctetStream(b"envelope-bytes".to_vec()),
            "text" => FileResponse::Text("envelope-text".into()),
            _ => FileResponse::Json(FileMeta { id: key, etag: "meta-v1".into() }),
        };
        Ok(EnvelopeApiGetFileResponse { r#return: value, etag: "meta-v1".into(), tags: vec!["001".into(), "\"quoted\"".into()], cached: false })
    }
    async fn get_fresh(&self) -> Result<String, EnvelopeApiGetFreshError> {
        Err(EnvelopeApiGetFreshError::NotModified(NotModified { etag: "same-tag".into() }))
    }
    async fn delete_file(&self, _if_match: String) -> Result<(), EnvelopeApiDeleteFileError> {
        Err(EnvelopeApiDeleteFileError::PreconditionFailed(PreconditionFailed { etag: "current-tag".into(), code: 412, msg: "stale revision".into() }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder().with_service(EnvelopeApiServer::new(Files)).serve_with_listener(listener).await
    });
    let client = EnvelopeApiClient::new(format!("http://{address}"));
    let meta = client.get_file("meta".into()).await?;
    assert!(matches!(meta.r#return, FileResponse::Json(FileMeta { id, .. }) if id == "meta"));
    assert_eq!(meta.etag, "meta-v1");
    assert_eq!(meta.tags, ["001", "\"quoted\""]);
    assert!(!meta.cached);
    let raw = client.get_file("raw".into()).await?;
    assert!(matches!(raw.r#return, FileResponse::OctetStream(bytes) if bytes == b"envelope-bytes"));
    let text = client.get_file("text".into()).await?;
    assert!(matches!(text.r#return, FileResponse::Text(text) if text == "envelope-text"));
    assert!(matches!(client.get_file("missing".into()).await, Err(EnvelopeApiGetFileError::NotFound(_))));
    server.abort();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    xidl_rust_axum::Server::builder().with_service(EnvelopeApiServer::new(Files)).serve_with_listener(listener).await?;
    Ok(())
}
