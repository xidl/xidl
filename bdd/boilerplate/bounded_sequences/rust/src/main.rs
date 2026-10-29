use async_trait::async_trait;
use std::collections::BTreeMap;
use std::future::IntoFuture;
use std::sync::atomic::{AtomicI32, Ordering};

pub mod gen {
    include!("../bounded_sequences.rs");
}
use gen::{
    billing::{Batch, BatchRejected, Export, InvoiceIds, Region, Regions, Total},
    codes::RetryState,
    Batches, BatchesClient, BatchesExportedResponse, BatchesMetadataResponse, BatchesRejectedError,
    BatchesServer,
};

#[derive(Default)]
struct Billing {
    accepted: AtomicI32,
}

impl Billing {
    fn ids(values: Vec<i32>) -> xidl_rust_axum::Result<InvoiceIds> {
        InvoiceIds::try_from(values)
            .map_err(|error| xidl_rust_axum::Error::new(500, error.to_string()))
    }
}

#[async_trait]
impl Batches for Billing {
    async fn rejected(&self, fault: i32) -> Result<Total, BatchesRejectedError> {
        let values = |channel| {
            Billing::ids(if fault == channel {
                vec![1, 2, 3]
            } else {
                vec![1, 2]
            })
        };
        Err(BatchesRejectedError::BatchRejected(BatchRejected {
            ids: values(1)?,
            retry: values(2)?,
            tokens: values(3)?,
            regions: BTreeMap::from([(Region::Eu, values(4)?)])
                .try_into()
                .map_err(|error: xidl_rust_axum::BoundExceeded| {
                    xidl_rust_axum::Error::new(500, error.to_string())
                })?,
            detail: Total { count: 2 },
            state: RetryState::Blocked,
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
    async fn query(&self, values: InvoiceIds) -> xidl_rust_axum::Result<InvoiceIds> {
        Ok(values)
    }
    async fn headers(&self, values: InvoiceIds) -> xidl_rust_axum::Result<InvoiceIds> {
        Ok(values)
    }
    async fn cookies(&self, values: InvoiceIds) -> xidl_rust_axum::Result<InvoiceIds> {
        Ok(values)
    }
    async fn upload(
        &self,
        bytes: xidl_rust_axum::BoundedVec<u8, 2>,
    ) -> xidl_rust_axum::Result<Total> {
        Ok(Total {
            count: bytes.len() as i32,
        })
    }
    async fn metadata(
        &self,
        invalid: bool,
        groups: Regions,
        _backup: Regions,
    ) -> xidl_rust_axum::Result<BatchesMetadataResponse> {
        Ok(BatchesMetadataResponse {
            value: if invalid {
                Regions::try_from_iter([("eu".into(), Self::ids(vec![1, 2, 3])?)])
                    .map_err(|error| xidl_rust_axum::Error::new(500, error.to_string()))?
            } else {
                groups
            },
        })
    }
    async fn exported(&self, invalid: bool) -> xidl_rust_axum::Result<BatchesExportedResponse> {
        let ids = Self::ids(if invalid { vec![1, 2, 3] } else { vec![1, 2] })?;
        Ok(BatchesExportedResponse {
            ids: ids.clone(),
            tokens: ids,
        })
    }
    async fn download(&self, raw: bool, invalid: bool) -> xidl_rust_axum::Result<Export> {
        let ids = Self::ids(if invalid { vec![1, 2, 3] } else { vec![1, 2] })?;
        Ok(if raw {
            Export::OctetStream(ids.map(|id| id as u8))
        } else {
            Export::Json(ids)
        })
    }
    async fn oversized(&self) -> xidl_rust_axum::Result<Batch> {
        let value = Self::ids(vec![1, 2, 3])?;
        Ok(Batch {
            invoices: InvoiceIds::new(),
            windows: Default::default(),
            regions: Regions::try_from_iter([("eu".into(), value)])
                .map_err(|error| xidl_rust_axum::Error::new(500, error.to_string()))?,
            days: Default::default(),
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
        let ids = InvoiceIds::try_from(values.clone())?;
        let returned = client
            .reserve(Batch {
                invoices: ids.clone(),
                windows: vec![ids.clone()].try_into()?,
                regions: Regions::try_from_iter([("eu".into(), ids.clone())])?,
                days: [ids.clone(), InvoiceIds::new()],
                retry: Some(ids),
            })
            .await?;
        assert_eq!(returned.invoices.as_ref(), values.as_slice());
    }
    assert_eq!(client.query(vec![1].try_into()?).await?.as_ref(), &[1]);
    assert_eq!(
        client.headers(vec![1, 2].try_into()?).await?.as_ref(),
        &[1, 2]
    );
    assert_eq!(
        client.cookies(vec![1, 2].try_into()?).await?.as_ref(),
        &[1, 2]
    );
    assert_eq!(client.upload(vec![1, 2].try_into()?).await?.count, 2);
    assert!(
        matches!(client.rejected(0).await, Err(BatchesRejectedError::BatchRejected(error))
        if error.ids.as_ref() == [1, 2] && error.retry.as_ref() == [1, 2] && error.tokens.as_ref() == [1, 2])
    );
    for fault in 1..=4 {
        assert!(
            matches!(client.rejected(fault).await, Err(BatchesRejectedError::Framework(error)) if error.code == 500)
        );
    }
    assert!(
        InvoiceIds::try_from(vec![1, 2, 3]).is_err(),
        "invalid SDK input cannot be constructed"
    );
    assert!(xidl_rust_axum::BoundedVec::<u8, 2>::try_from(vec![1, 2, 3]).is_err());
    let groups = Regions::try_from_iter([("eu".into(), vec![1, 2].try_into()?)])?;
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
    assert!(
        Regions::try_from_iter([
            ("eu".into(), InvoiceIds::new()),
            ("us".into(), InvoiceIds::new()),
            ("jp".into(), InvoiceIds::new())
        ])
        .is_err(),
        "outer map bound survives the typedef"
    );
    assert_eq!(client.exported(false).await?.ids.as_ref(), &[1, 2]);
    assert!(client.exported(true).await.is_err());
    assert!(
        matches!(client.download(false, false).await?, Export::Json(ids) if ids.as_ref() == [1, 2])
    );
    assert!(
        matches!(client.download(true, false).await?, Export::OctetStream(ids) if ids.as_ref() == [1, 2])
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
