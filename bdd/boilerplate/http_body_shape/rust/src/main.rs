use async_trait::async_trait;
use xidl_rust_axum::Service;

pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}

struct Documents;

#[async_trait]
impl gen::BodyShape for Documents {
    async fn document(&self) -> xidl_rust_axum::Result<gen::BodyShapeDocumentResponse> {
        Ok(gen::BodyShapeDocumentResponse {
            r#return: gen::Document { title: "Guide".into() },
            etag: "document-v1".into(),
        })
    }

    async fn scalar(&self) -> xidl_rust_axum::Result<gen::BodyShapeScalarResponse> {
        Ok(gen::BodyShapeScalarResponse {
            r#return: "Guide".into(),
            etag: "scalar-v1".into(),
            count: 7,
            cached: false,
        })
    }

    async fn rename(&self, document: gen::Document) -> xidl_rust_axum::Result<gen::BodyShapeRenameResponse> {
        Ok(gen::BodyShapeRenameResponse { r#return: document, etag: "renamed-v1".into() })
    }

    async fn output(&self) -> xidl_rust_axum::Result<gen::BodyShapeOutputResponse> {
        Ok(gen::BodyShapeOutputResponse {
            document: gen::Document { title: "Guide".into() },
            etag: "output-v1".into(),
        })
    }

    async fn upload(
        &self,
        media_type: Option<String>,
        payload: Vec<u8>,
    ) -> xidl_rust_axum::Result<gen::UploadResult> {
        Ok(gen::UploadResult {
            media_type: media_type.unwrap_or_default(),
            size: payload.len() as u32,
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let router = gen::BodyShapeServer::new(Documents)
        .into_router()
        .layer(axum::extract::DefaultBodyLimit::max(64));
    let test_router = router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, test_router).await });
    let client = gen::BodyShapeClient::new(format!("http://{address}"));
    let document = client.document().await?;
    assert_eq!(document.r#return.title, "Guide");
    assert_eq!(document.etag, "document-v1");
    let scalar = client.scalar().await?;
    assert_eq!(scalar.r#return, "Guide");
    assert_eq!(scalar.etag, "scalar-v1");
    assert_eq!(scalar.count, 7);
    assert!(!scalar.cached);
    let renamed = client.rename(gen::Document { title: "Revised".into() }).await?;
    assert_eq!(renamed.r#return.title, "Revised");
    let output = client.output().await?;
    assert_eq!(output.document.title, "Guide");
    assert_eq!(output.etag, "output-v1");
    let uploaded = client.upload(Some("text/markdown".into()), b"# Guide".to_vec()).await?;
    assert_eq!(uploaded.media_type, "text/markdown");
    assert_eq!(uploaded.size, 7);
    let oversized = client.upload(None, vec![b'x'; 65]).await;
    assert_eq!(oversized.err().map(|error| error.code), Some(413));
    server.abort();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
