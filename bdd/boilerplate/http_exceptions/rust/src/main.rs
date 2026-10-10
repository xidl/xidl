use async_trait::async_trait;

pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}
use gen::*;

struct FileService;

#[async_trait]
impl Files for FileService {
    async fn get_file(&self, id: String) -> Result<FileMeta, FilesGetFileError> {
        match id.as_str() {
            "cached" => Err(FilesGetFileError::NotModified(NotModified {
                etag: "\"v2\"".into(),
            })),
            "stale" | "stale-minimal" => {
                let full = id == "stale";
                Err(FilesGetFileError::PreconditionFailed(PreconditionFailed {
                    etag: "\"v2\"".into(),
                    hints: full.then(|| vec!["reload".into(), "retry".into()]),
                    session_id: "renewed".into(),
                    retry_token: full.then(|| "001/;%".into()),
                    attempts: full.then(|| vec!["first".into(), "second".into()]),
                    code: 41201,
                    msg: "stale revision".into(),
                    detail: full.then(|| "reload before retry".into()),
                }))
            }
            "missing" => Err(FilesGetFileError::NotFound(NotFound {
                code: 40401,
                msg: "not found".into(),
            })),
            "framework-typed" => Err(xidl_rust_axum::Error::new(412, "framework conflict").into()),
            "framework" => Err(xidl_rust_axum::Error::new(500, "storage unavailable").into()),
            _ => Ok(FileMeta {
                id,
                etag: "v2".into(),
            }),
        }
    }
}

#[async_trait]
impl Naming for FileService {
    async fn read(&self, id: String) -> Result<(), NamingReadError> {
        match id.as_str() {
            "client" => Err(NamingReadError::Http409FilesClient(collision::FilesClient { reason: "client name".into() })),
            _ => Err(NamingReadError::Http502Framework(collision::Framework { reason: "declared framework".into() })),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(FilesServer::new(FileService))
        .with_service(NamingServer::new(FileService))
            .serve_with_listener(listener)
            .await
    });
    let client = FilesClient::new(format!("http://{address}"));
    assert_eq!(client.get_file("ok".into()).await?.id, "ok");
    let FilesGetFileError::NotModified(cached) =
        client.get_file("cached".into()).await.unwrap_err()
    else {
        panic!("expected typed 304")
    };
    assert_eq!(cached.etag, "\"v2\"");
    for full in [true, false] {
        let id = if full { "stale" } else { "stale-minimal" };
        let FilesGetFileError::PreconditionFailed(error) =
            client.get_file(id.into()).await.unwrap_err()
        else {
            panic!("expected typed 412")
        };
        assert_eq!(error.etag, "\"v2\"");
        assert_eq!(error.session_id, "renewed");
        assert_eq!(error.code, 41201);
        assert_eq!(error.msg, "stale revision");
        assert_eq!(error.retry_token, full.then(|| "001/;%".into()));
        assert_eq!(
            error.hints,
            full.then(|| vec!["reload".into(), "retry".into()])
        );
        assert_eq!(
            error.attempts,
            full.then(|| vec!["first".into(), "second".into()])
        );
        assert_eq!(error.detail, full.then(|| "reload before retry".into()));
    }
    assert!(matches!(
        client.get_file("missing".into()).await,
        Err(FilesGetFileError::NotFound(NotFound { code: 40401, .. }))
    ));
    assert!(matches!(
        client.get_file("framework".into()).await,
        Err(FilesGetFileError::Framework(_))
    ));
    let FilesGetFileError::Framework(error) = client.get_file("framework-typed".into()).await.unwrap_err() else { panic!("expected framework 412") };
    assert_eq!(error.http_status().as_u16(), 412);
    let raised = client.get_file("missing".into()).await.unwrap_err();
    assert_eq!(raised.to_string(), "HTTP 404: NotFound");
    assert!(std::error::Error::source(&raised).is_none());
    let framework = client.get_file("framework".into()).await.unwrap_err();
    assert!(std::error::Error::source(&framework).is_some());
    let naming = NamingClient::new(format!("http://{address}"));
    assert!(matches!(naming.read("client".into()).await,
        Err(NamingReadError::Http409FilesClient(error)) if error.reason == "client name"));
    assert!(matches!(naming.read("framework".into()).await,
        Err(NamingReadError::Http502Framework(error)) if error.reason == "declared framework"));
    println!("generated client exception round trips passed");
    server.abort();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    xidl_rust_axum::Server::builder()
        .with_service(FilesServer::new(FileService))
        .with_service(NamingServer::new(FileService))
        .serve_with_listener(listener)
        .await?;
    Ok(())
}
