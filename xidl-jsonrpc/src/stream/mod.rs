use crate::Error;
use futures_core::Stream;
use futures_util::StreamExt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

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

pub struct ClientStreamWriter<T, R> {
    tx: Option<mpsc::Sender<Result<T, Error>>>,
    response: Option<JoinHandle<Result<R, Error>>>,
}

impl<T, R> ClientStreamWriter<T, R> {
    pub fn new(tx: mpsc::Sender<Result<T, Error>>, response: JoinHandle<Result<R, Error>>) -> Self {
        Self {
            tx: Some(tx),
            response: Some(response),
        }
    }

    pub async fn write(&mut self, item: T) -> Result<(), Error> {
        let tx = self
            .tx
            .as_mut()
            .ok_or(Error::Protocol("stream writer is already closed"))?;
        tx.send(Ok(item))
            .await
            .map_err(|_| Error::Protocol("stream writer is closed"))
    }

    pub async fn close(mut self) -> Result<R, Error> {
        let _ = self.tx.take();
        let response = self
            .response
            .take()
            .ok_or(Error::Protocol("stream writer is already closed"))?;
        response
            .await
            .map_err(|_| Error::Protocol("stream response task failed"))?
    }

    pub async fn cancel(mut self) -> Result<(), Error> {
        let _ = self.tx.take();
        if let Some(response) = self.response.take() {
            response.abort();
        }
        Ok(())
    }
}

impl<T, R> Drop for ClientStreamWriter<T, R> {
    fn drop(&mut self) {
        let _ = self.tx.take();
    }
}

pub struct DuplexStream<TIn, TOut> {
    writer: ClientStreamWriter<TIn, ()>,
    reader: BoxStream<'static, TOut>,
}

impl<TIn, TOut> DuplexStream<TIn, TOut> {
    pub fn new(writer: ClientStreamWriter<TIn, ()>, reader: BoxStream<'static, TOut>) -> Self {
        Self { writer, reader }
    }

    pub async fn write(&mut self, item: TIn) -> Result<(), Error> {
        self.writer.write(item).await
    }

    pub async fn read(&mut self) -> Option<Result<TOut, Error>> {
        self.reader.next().await
    }

    pub async fn close(self) -> Result<(), Error> {
        self.writer.close().await.map(|_| ())
    }

    pub async fn cancel(self) -> Result<(), Error> {
        self.writer.cancel().await
    }
}
