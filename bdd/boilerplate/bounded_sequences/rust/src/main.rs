use async_trait::async_trait;
use std::collections::BTreeMap;
use std::future::IntoFuture;
use std::sync::atomic::{AtomicI32, Ordering};

pub mod gen {
    include!("../bounded_sequences.rs");
}
use gen::{
    billing::{Batch, BatchRejected, Export, Region, Total},
    codes::RetryState,
    Batches, BatchesClient, BatchesExportedResponse, BatchesMetadataResponse, BatchesRejectedError,
    BatchesServer,
};

#[derive(Default)]
struct Billing {
    accepted: AtomicI32,
}

#[async_trait]
impl Batches for Billing {
    async fn rejected(&self, fault: i32) -> Result<Total, BatchesRejectedError> {
        let values = |channel| {
            if fault == channel {
                vec![1, 2, 3]
            } else {
                vec![1, 2]
            }
        };
        Err(BatchesRejectedError::BatchRejected(BatchRejected {
            ids: values(1),
            retry: values(2),
            tokens: values(3),
            regions: BTreeMap::from([(Region::eu, values(4))]),
            detail: Total { count: 2 },
            state: RetryState::blocked,
            tag: "123".into(),
            token: "true".into(),
        }))
    }
    async fn reserve(&self, batch: Batch) -> xidl_rust_axum::Result<Batch> {
        self.accepted.fetch_add(1, Ordering::SeqCst);
        Ok(batch)
    }
    async fn accepted(&self) -> xidl_rust_axum::Result<Total> {
        Ok(Total {
            count: self.accepted.load(Ordering::SeqCst),
        })
    }
    async fn query(&self, values: Vec<i32>) -> xidl_rust_axum::Result<Vec<i32>> {
        Ok(values)
    }
    async fn headers(&self, values: Vec<i32>) -> xidl_rust_axum::Result<Vec<i32>> {
        Ok(values)
    }
    async fn cookies(&self, values: Vec<i32>) -> xidl_rust_axum::Result<Vec<i32>> {
        Ok(values)
    }
    async fn upload(&self, bytes: Vec<u8>) -> xidl_rust_axum::Result<Total> {
        Ok(Total {
            count: bytes.len() as i32,
        })
    }
    async fn metadata(
        &self,
        invalid: bool,
        groups: BTreeMap<String, Vec<i32>>,
        _backup: BTreeMap<String, Vec<i32>>,
    ) -> xidl_rust_axum::Result<BatchesMetadataResponse> {
        Ok(BatchesMetadataResponse {
            value: if invalid {
                BTreeMap::from([("eu".into(), vec![1, 2, 3])])
            } else {
                groups
            },
        })
    }
    async fn exported(&self, invalid: bool) -> xidl_rust_axum::Result<BatchesExportedResponse> {
        let ids = if invalid { vec![1, 2, 3] } else { vec![1, 2] };
        Ok(BatchesExportedResponse {
            ids: ids.clone(),
            tokens: ids,
        })
    }
    async fn download(&self, raw: bool, invalid: bool) -> xidl_rust_axum::Result<Export> {
        let ids = if invalid { vec![1, 2, 3] } else { vec![1, 2] };
        Ok(if raw {
            Export::OctetStream(ids.into_iter().map(|id| id as u8).collect())
        } else {
            Export::Json(ids)
        })
    }
    async fn oversized(&self) -> xidl_rust_axum::Result<Batch> {
        Ok(Batch {
            invoices: vec![],
            windows: vec![],
            regions: [("eu".into(), vec![1, 2, 3])].into(),
            days: [vec![], vec![]],
            retry: None,
        })
    }
}

async fn reject_invalid_provider_responses() -> Result<(), Box<dyn std::error::Error>> {
    async fn response(request: axum::extract::Request) -> axum::response::Response {
        let query = request.uri().query().unwrap_or_default();
        if request.uri().path() == "/rejected" {
            let values = |channel| {
                if query == format!("fault={channel}") {
                    vec![1, 2, 3]
                } else {
                    vec![1, 2]
                }
            };
            return axum::response::Response::builder()
                .status(409)
                .header("content-type", "application/json")
                .header("retry", serde_json::to_string(&values(2)).unwrap())
                .header("detail", "{\"count\":2}")
                .header("tag", "123")
                .header("set-cookie", "state=\"true\"")
                .header("set-cookie", "token=true")
                .header(
                    "set-cookie",
                    format!("tokens={}", serde_json::to_string(&values(3)).unwrap()),
                )
                .body(axum::body::Body::from(
                    serde_json::json!({"ids": values(1), "regions": {"eu": values(4)}}).to_string(),
                ))
                .unwrap();
        }
        if request.uri().path() == "/download" {
            let raw = query.contains("raw=true");
            return axum::response::Response::builder()
                .header(
                    "content-type",
                    if raw {
                        "application/octet-stream"
                    } else {
                        "application/json"
                    },
                )
                .body(axum::body::Body::from(if raw {
                    vec![1, 2, 3]
                } else {
                    b"[1,2,3]".to_vec()
                }))
                .unwrap();
        }
        let bad_cookie = query.contains("invalid=true");
        let mut response = axum::response::Response::builder()
            .status(204)
            .body(axum::body::Body::empty())
            .unwrap();
        for value in if bad_cookie {
            &["1", "2"][..]
        } else {
            &["1", "2", "3"][..]
        } {
            response
                .headers_mut()
                .append("ids", axum::http::HeaderValue::from_static(value));
        }
        for value in if bad_cookie {
            &["tokens=1", "tokens=2", "tokens=3"][..]
        } else {
            &["tokens=1", "tokens=2"][..]
        } {
            response
                .headers_mut()
                .append("set-cookie", axum::http::HeaderValue::from_static(value));
        }
        response
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server =
        tokio::spawn(axum::serve(listener, axum::Router::new().fallback(response)).into_future());
    let client = BatchesClient::new(format!("http://{address}"));
    for fault in 1..=4 {
        assert!(
            matches!(
                client.rejected(fault).await,
                Err(BatchesRejectedError::Framework(_))
            ),
            "invalid exception must not become a typed business error"
        );
    }
    assert!(
        client.exported(false).await.is_err(),
        "invalid response header"
    );
    assert!(
        client.exported(true).await.is_err(),
        "invalid response cookie"
    );
    assert!(
        client.download(false, false).await.is_err(),
        "invalid JSON representation"
    );
    assert!(
        client.download(true, false).await.is_err(),
        "invalid binary representation"
    );
    server.abort();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    reject_invalid_provider_responses().await?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        xidl_rust_axum::Server::builder()
            .with_service(BatchesServer::new(Billing::default()))
            .serve_with_listener(listener)
            .await
    });
    let client = BatchesClient::new(format!("http://{address}"));
    for values in [vec![], vec![1], vec![1, 2]] {
        let returned = client
            .reserve(Batch {
                invoices: values.clone(),
                windows: vec![values.clone()],
                regions: [("eu".into(), values.clone())].into(),
                days: [values.clone(), vec![]],
                retry: Some(values.clone()),
            })
            .await?;
        assert_eq!(returned.invoices, values);
    }
    assert_eq!(client.query(vec![1]).await?, vec![1]);
    assert_eq!(client.headers(vec![1, 2]).await?, vec![1, 2]);
    assert_eq!(client.cookies(vec![1, 2]).await?, vec![1, 2]);
    assert_eq!(client.upload(vec![1, 2]).await?.count, 2);
    assert!(
        matches!(client.rejected(0).await, Err(BatchesRejectedError::BatchRejected(error)) if error.ids == vec![1, 2] && error.retry == vec![1, 2] && error.tokens == vec![1, 2])
    );
    for fault in 1..=4 {
        assert!(
            matches!(client.rejected(fault).await, Err(BatchesRejectedError::Framework(error)) if error.code == 500)
        );
    }
    assert!(client.upload(vec![1, 2, 3]).await.is_err());
    assert!(client.query(vec![1, 2, 3]).await.is_err());
    assert!(client.headers(vec![1, 2, 3]).await.is_err());
    assert!(client.cookies(vec![1, 2, 3]).await.is_err());
    let groups = BTreeMap::from([("eu".into(), vec![1, 2])]);
    assert_eq!(
        client
            .metadata(false, groups.clone(), groups.clone())
            .await?
            .value,
        groups
    );
    assert_eq!(
        client
            .metadata(true, groups.clone(), groups.clone())
            .await
            .err()
            .expect("invalid metadata output")
            .code,
        500
    );
    let invalid = BTreeMap::from([("eu".into(), vec![1, 2, 3])]);
    assert!(client
        .metadata(false, invalid.clone(), groups.clone())
        .await
        .is_err());
    assert!(client.metadata(false, groups, invalid).await.is_err());
    assert_eq!(client.exported(false).await?.ids, vec![1, 2]);
    assert!(client.exported(true).await.is_err());
    assert!(matches!(client.download(false, false).await?, Export::Json(ids) if ids == vec![1, 2]));
    assert!(
        matches!(client.download(true, false).await?, Export::OctetStream(ids) if ids == vec![1, 2])
    );
    assert!(client.download(false, true).await.is_err());
    assert!(client.download(true, true).await.is_err());
    assert!(client.oversized().await.is_err());
    assert_eq!(
        client.accepted().await?.count,
        3,
        "invalid wire data cannot mutate billing"
    );
    server.abort();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    xidl_rust_axum::Server::builder()
        .with_service(BatchesServer::new(Billing::default()))
        .serve_with_listener(listener)
        .await?;
    Ok(())
}
