use async_trait::async_trait;
use futures_util::{stream, StreamExt};
use xidl_rust_axum::stream::{boxed_sse, StreamError, StreamOutcome};

pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}
struct Ledger;

fn conflict(job: &str, reason: &str) -> gen::Conflict {
    gen::Conflict {
        detail: gen::ConflictDetail {
            job: job.into(),
            reason: reason.into(),
        },
    }
}

#[async_trait]
impl gen::Ledger for Ledger {
    async fn export_rows(
        &self,
        req: xidl_rust_axum::Request<gen::LedgerExportRowsRequest>,
    ) -> Result<
        xidl_rust_axum::stream::SseStream<gen::ExportRow, gen::LedgerExportRowsError>,
        gen::LedgerExportRowsError,
    > {
        let job = req.data.job;
        let rows = match job.as_str() {
            "locked" => vec![Err(gen::LedgerExportRowsError::Conflict(conflict(
                &job, "locked",
            )))],
            "retired" => vec![Err(gen::LedgerExportRowsError::Retired(
                gen::Retired::default(),
            ))],
            "empty" => vec![],
            _ => {
                let mut rows = vec![Ok(gen::ExportRow {
                    id: "zero".into(),
                    quantity: 0,
                })];
                if job == "partial" {
                    rows.push(Err(gen::LedgerExportRowsError::Conflict(conflict(
                        &job, "locked",
                    ))));
                } else {
                    rows.push(Ok(gen::ExportRow {
                        id: "雪".into(),
                        quantity: 2,
                    }));
                }
                rows
            }
        };
        Ok(boxed_sse(stream::iter(rows)))
    }
    async fn import_rows(
        &self,
        req: xidl_rust_axum::Request<
            xidl_rust_axum::stream::NdjsonStream<gen::LedgerImportRowsRequest>,
        >,
    ) -> Result<gen::ImportResult, gen::LedgerImportRowsError> {
        let mut input = req.data;
        let mut ids = std::collections::BTreeSet::new();
        let mut total = 0;
        while let Some(value) = input.next().await {
            let row = value?.row;
            if row.quantity < 0 || !ids.insert(row.id.clone()) {
                return Err(gen::LedgerImportRowsError::Conflict(conflict(
                    &row.id,
                    "duplicate or negative quantity",
                )));
            }
            total += row.quantity;
        }
        Ok(gen::ImportResult {
            accepted: ids.len() as i32,
            total,
        })
    }
}

async fn sdk(base: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = gen::LedgerClient::new(base);
    let mut reader = client.export_rows("ready".into()).await?;
    let mut rows = vec![];
    while let Some(row) = reader.read().await {
        rows.push(row?);
    }
    assert_eq!(
        rows.iter().map(|row| row.quantity).collect::<Vec<_>>(),
        [0, 2]
    );
    assert_eq!(rows[1].id, "雪");
    assert_eq!(reader.outcome(), Some(StreamOutcome::Completed));
    assert!(
        matches!(client.export_rows("locked".into()).await,Err(gen::LedgerExportRowsError::Conflict(value)) if value.detail.job=="locked")
    );
    assert!(matches!(
        client.export_rows("retired".into()).await,
        Err(gen::LedgerExportRowsError::Retired(_))
    ));
    let mut partial = client.export_rows("partial".into()).await?;
    assert_eq!(partial.read().await.unwrap()?.quantity, 0);
    assert!(
        matches!(partial.read().await.unwrap(),Err(StreamError::Failed(gen::LedgerExportRowsError::Conflict(value))) if value.detail.job=="partial")
    );
    assert_eq!(partial.outcome(), Some(StreamOutcome::Failed));
    assert!(partial.read().await.is_none());
    let mut empty = client.export_rows("empty".into()).await?;
    assert!(empty.read().await.is_none());
    assert_eq!(empty.outcome(), Some(StreamOutcome::Completed));
    let mut cancelled = client.export_rows("ready".into()).await?;
    cancelled.read().await.unwrap()?;
    cancelled.cancel();
    assert_eq!(cancelled.outcome(), Some(StreamOutcome::Cancelled));
    let mut upload = client.import_rows().await?;
    upload
        .write(gen::LedgerImportRowsRequest {
            row: gen::ExportRow {
                id: "zero".into(),
                quantity: 0,
            },
        })
        .await?;
    upload
        .write(gen::LedgerImportRowsRequest {
            row: gen::ExportRow {
                id: "雪".into(),
                quantity: 2,
            },
        })
        .await?;
    let result = upload.close().await?;
    assert_eq!((result.accepted, result.total), (2, 2));
    let mut upload = client.import_rows().await?;
    for _ in 0..2 {
        upload
            .write(gen::LedgerImportRowsRequest {
                row: gen::ExportRow {
                    id: "duplicate".into(),
                    quantity: 1,
                },
            })
            .await?;
    }
    assert!(
        matches!(upload.close().await,Err(gen::LedgerImportRowsError::Conflict(value)) if value.detail.job=="duplicate")
    );
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(base) = std::env::args().nth(1) {
        return sdk(base).await;
    }
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder()
        .with_service(gen::LedgerServer::new(Ledger))
        .serve(&format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
