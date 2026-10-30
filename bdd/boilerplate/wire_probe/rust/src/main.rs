use async_trait::async_trait;
use futures_util::{stream};
use xidl_rust_axum::stream::{boxed_sse, SseStream, StreamError, StreamOutcome};

#[allow(non_camel_case_types)]
pub mod gen {
    include!("../{{MODULE_NAME}}.rs");
}
use gen::catalog::{Availability, Change, Item};
struct Store;

fn item() -> Item {
    Item {
        sku: "sku-雪".into(),
        internal_note: "private".into(),
        memo: None,
        availability: Availability::in_stock,
        stock_grid: [[0, 1, 2], [3, 4, 5]],
        thumbnail: vec![0, 127, 255],
        revision: 9_007_199_254_740_991,
        initial: 'A',
        mark: '🚀',
        letters: vec!['A', '雪', '🚀'],
    }
}
fn conflict() -> gen::Conflict {
    gen::Conflict {
        sku: "locked".into(),
        reason: "inventory locked".into(),
    }
}

#[async_trait]
impl gen::Store for Store {
    async fn echo(&self, item: Item) -> Result<Item, gen::StoreEchoError> {
        if item.sku == "locked" {
            return Err(gen::StoreEchoError::Conflict(conflict()));
        }
        Ok(item)
    }
    async fn apply(&self, change: Change) -> xidl_rust_axum::Result<Change> {
        Ok(change.clone())
    }
    async fn label(&self, change: gen::catalog::LabelChange) -> xidl_rust_axum::Result<gen::catalog::LabelChange> {
        Ok(change.clone())
    }
    async fn bindings(
        &self,
        count: i32,
        initial: char,
        session: String,
    ) -> xidl_rust_axum::Result<gen::StoreBindingsResponse> {
        Ok(gen::StoreBindingsResponse {
            r#return: format!("{count}:{initial}:{session}"),
            count_reply: count,
            receipt: session,
        })
    }
    async fn feed(
        &self,
        req: xidl_rust_axum::Request<gen::StoreFeedRequest>,
    ) -> Result<SseStream<Change, gen::StoreFeedError>, gen::StoreFeedError> {
        if req.data.state == "locked" {
            return Err(gen::StoreFeedError::Conflict(conflict()));
        }
        let tail = if req.data.state == "partial" {
            Err(gen::StoreFeedError::Conflict(conflict()))
        } else {
            Ok(Change::new_reason("retired".into()))
        };
        Ok(boxed_sse(stream::iter(vec![
            Ok(Change::new_item(item())),
            tail,
        ])))
    }
}

async fn sdk(base: String) -> Result<(), Box<dyn std::error::Error>> {
    let client = gen::StoreClient::new(base);
    let value = client.echo(item()).await?;
    assert_eq!(value.sku, "sku-雪");
    assert_eq!(value.internal_note, "");
    assert_eq!(value.thumbnail, [0, 127, 255]);
    assert_eq!(value.mark, '🚀');
    assert_eq!(value.stock_grid, [[0, 1, 2], [3, 4, 5]]);
    assert_eq!(value.revision, 9_007_199_254_740_991);
    assert!(value.memo.is_none());
    let mut locked = item();
    locked.sku = "locked".into();
    assert!(matches!(
        client.echo(locked).await,
        Err(gen::StoreEchoError::Conflict(_))
    ));
    let changed = client.apply(Change::new_item(item())).await?;
    assert!(changed.is_item());
    assert_eq!(changed.as_item().mark, '🚀');
    let batch = client.apply(Change::new_batch(vec![item()])).await?;
    assert_eq!(batch.as_batch()[0].sku, "sku-雪");
    let retired = client.apply(Change::new_reason("retired".into())).await?;
    assert!(retired.is_reason());
    assert_eq!(retired.tag(), None);
    assert_eq!(retired.into_reason(), "retired");
    let label = client.label(gen::catalog::LabelChange::new_reason("retired".into())).await?;
    assert_eq!(label.tag(), Some(&'A'));
    assert_eq!(label.into_reason(), "retired");
    let bindings = client.bindings(7, 'A', "%2F+ %".into()).await?;
    assert_eq!(bindings.r#return, "7:A:%2F+ %");
    assert_eq!(bindings.count_reply, 7);
    assert_eq!(bindings.receipt, "%2F+ %");
    let mut feed = client.feed("ready".into()).await?;
    assert!(feed.read().await.unwrap()?.is_item());
    assert!(feed.read().await.unwrap()?.is_reason());
    assert!(feed.read().await.is_none());
    assert_eq!(feed.outcome(), Some(StreamOutcome::Completed));
    let mut partial = client.feed("partial".into()).await?;
    assert!(partial.read().await.unwrap()?.is_item());
    assert!(matches!(
        partial.read().await.unwrap(),
        Err(StreamError::Failed(gen::StoreFeedError::Conflict(_)))
    ));
    assert!(partial.read().await.is_none());
    assert_eq!(partial.outcome(), Some(StreamOutcome::Failed));
    Ok(())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(base) = std::env::args().nth(1) {
        return sdk(base).await;
    }
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    xidl_rust_axum::Server::builder()
        .with_service(gen::StoreServer::new(Store))
        .serve(&format!("127.0.0.1:{port}"))
        .await?;
    Ok(())
}
