use crate::Error;
use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = Result<T, Error>> + Send + 'a>>;
pub type Writer<T, R> = ClientStreamWriter<T, R>;
pub type ReaderWriter<TIn, TOut> = DuplexStream<TIn, TOut>;

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

pub struct Reader<'a, T> {
    inner: BoxStream<'a, T>,
}

impl<'a, T> Reader<'a, T> {
    pub fn new(inner: BoxStream<'a, T>) -> Self {
        Self { inner }
    }

    pub async fn read(&mut self) -> Option<Result<T, Error>> {
        self.inner.next().await
    }

    pub fn into_inner(self) -> BoxStream<'a, T> {
        self.inner
    }
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
    reader: Reader<'static, TOut>,
}

impl<TIn, TOut> DuplexStream<TIn, TOut> {
    pub fn new(writer: ClientStreamWriter<TIn, ()>, reader: Reader<'static, TOut>) -> Self {
        Self { writer, reader }
    }

    pub async fn write(&mut self, item: TIn) -> Result<(), Error> {
        self.writer.write(item).await
    }

    pub async fn read(&mut self) -> Option<Result<TOut, Error>> {
        self.reader.read().await
    }

    pub async fn close(self) -> Result<(), Error> {
        self.writer.close().await.map(|_| ())
    }

    pub async fn cancel(self) -> Result<(), Error> {
        self.writer.cancel().await
    }

    pub fn into_parts(self) -> (ClientStreamWriter<TIn, ()>, Reader<'static, TOut>) {
        (self.writer, self.reader)
    }
}

pub fn open_bidi_server<S>(io: S) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_io(io)
}

pub async fn open_bidi_client<S>(
    mut io: S,
    method: &str,
) -> Result<ReaderWriter<Value, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1u64,
        "method": method,
        "params": Value::Null,
    });
    let payload = serde_json::to_string(&request)?;
    io.write_all(payload.as_bytes()).await?;
    io.write_all(b"\n").await?;
    io.flush().await?;
    Ok(open_bidi_io(io))
}

pub async fn open_server_stream_client<S>(
    mut io: S,
    method: &str,
    params: Value,
) -> Result<Reader<'static, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1u64,
        "method": method,
        "params": params,
    });
    let payload = serde_json::to_string(&request)?;
    io.write_all(payload.as_bytes()).await?;
    io.write_all(b"\n").await?;
    io.flush().await?;

    let (read_half, _write_half) = tokio::io::split(io);
    let reader_stream = boxed(async_stream::try_stream! {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).await?;
            if bytes == 0 {
                break;
            }
            let value: Value = serde_json::from_str(&line)?;
            yield value;
        }
    });
    Ok(Reader::new(reader_stream))
}

fn open_bidi_io<S>(io: S) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read_half, write_half) = tokio::io::split(io);
    let (tx, mut rx) = mpsc::channel::<Result<Value, Error>>(32);
    let writer_task = tokio::spawn(async move {
        let mut writer = BufWriter::new(write_half);
        while let Some(item) = rx.recv().await {
            let value = item?;
            let payload = serde_json::to_string(&value)?;
            writer.write_all(payload.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
        writer.shutdown().await?;
        Ok(())
    });
    let writer = Writer::new(tx, writer_task);

    let reader_stream = boxed(async_stream::try_stream! {
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).await?;
            if bytes == 0 {
                break;
            }
            let value: Value = serde_json::from_str(&line)?;
            yield value;
        }
    });
    let reader = Reader::new(reader_stream);
    ReaderWriter::new(writer, reader)
}
