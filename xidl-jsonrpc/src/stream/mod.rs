use crate::Error;
use futures_core::Stream;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send + 'a>>;

pub fn boxed<'a, T, S>(stream: S) -> BoxStream<'a, T>
where
    T: Send + 'a,
    S: Stream<Item = Result<T, Error>> + Send + 'a,
{
    Box::pin(stream)
}

pub fn polling<'a, T, F, Fut>(mut fetch: F, interval: Duration) -> BoxStream<'a, T>
where
    T: Send + 'a,
    F: FnMut() -> Fut + Send + 'a,
    Fut: Future<Output = Result<T, Error>> + Send + 'a,
{
    boxed(async_stream::try_stream! {
        loop {
            let value = fetch().await?;
            yield value;
            tokio::time::sleep(interval).await;
        }
    })
}
