#[cfg(test)]
mod tests;

use crate::Error;
use crate::codec::Codec;
use futures_core::Stream;
use futures_util::StreamExt;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
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

/// Opens a server-side bidirectional stream using newline-delimited JSON framing.
pub fn open_bidi_server<S>(io: S) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_server_with(io, Codec::Json)
}

/// Opens a server-side bidirectional stream using length-prefixed MessagePack framing.
#[cfg(feature = "msgpack")]
pub fn open_bidi_server_msgpack<S>(io: S) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_server_with(io, Codec::Msgpack)
}

pub(crate) fn open_bidi_server_with<S>(io: S, codec: Codec) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_io(io, codec)
}

/// Opens a client-side bidirectional stream using newline-delimited JSON framing.
pub async fn open_bidi_client<S>(io: S, method: &str) -> Result<ReaderWriter<Value, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_client_with(io, method, Codec::Json).await
}

/// Opens a client-side bidirectional stream using length-prefixed MessagePack framing.
#[cfg(feature = "msgpack")]
pub async fn open_bidi_client_msgpack<S>(
    io: S,
    method: &str,
) -> Result<ReaderWriter<Value, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_bidi_client_with(io, method, Codec::Msgpack).await
}

async fn open_bidi_client_with<S>(
    mut io: S,
    method: &str,
    codec: Codec,
) -> Result<ReaderWriter<Value, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    write_request_line(&mut io, method, Value::Null, codec).await?;
    let (read_half, write_half) = tokio::io::split(io);
    let mut reader = BufReader::new(read_half);
    read_handshake(&mut reader, codec).await?;
    let writer = spawn_stream_writer(write_half, codec);
    let reader = value_reader_buf(reader, codec);
    Ok(ReaderWriter::new(writer, reader))
}

/// Opens a client-side server stream using newline-delimited JSON framing.
pub async fn open_server_stream_client<S>(
    io: S,
    method: &str,
    params: Value,
) -> Result<Reader<'static, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_server_stream_client_with(io, method, params, Codec::Json).await
}

/// Opens a client-side server stream using length-prefixed MessagePack framing.
#[cfg(feature = "msgpack")]
pub async fn open_server_stream_client_msgpack<S>(
    io: S,
    method: &str,
    params: Value,
) -> Result<Reader<'static, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    open_server_stream_client_with(io, method, params, Codec::Msgpack).await
}

async fn open_server_stream_client_with<S>(
    mut io: S,
    method: &str,
    params: Value,
    codec: Codec,
) -> Result<Reader<'static, Value>, Error>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    write_request_line(&mut io, method, params, codec).await?;

    let (read_half, _write_half) = tokio::io::split(io);
    let mut reader = BufReader::new(read_half);
    read_handshake(&mut reader, codec).await?;
    Ok(value_reader_buf(reader, codec))
}

fn open_bidi_io<S>(io: S, codec: Codec) -> ReaderWriter<Value, Value>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (read_half, write_half) = tokio::io::split(io);
    let writer = spawn_stream_writer(write_half, codec);
    let reader = value_reader(read_half, codec);
    ReaderWriter::new(writer, reader)
}

async fn write_request_line<W>(
    writer: &mut W,
    method: &str,
    params: Value,
    codec: Codec,
) -> Result<(), Error>
where
    W: AsyncWrite + Unpin,
{
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1u64,
        "method": method,
        "params": params,
    });
    codec.write(writer, &request).await
}

fn value_reader<R>(reader: R, codec: Codec) -> Reader<'static, Value>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    value_reader_buf(BufReader::new(reader), codec)
}

fn value_reader_buf<R>(mut reader: R, codec: Codec) -> Reader<'static, Value>
where
    R: AsyncBufRead + Unpin + Send + 'static,
{
    let reader_stream = boxed(async_stream::try_stream! {
        loop {
            let Some(value) = codec.read::<_, Value>(&mut reader).await? else {
                break;
            };
            yield value;
        }
    });
    Reader::new(reader_stream)
}

/// Spawns the background task that serializes stream writes onto `write_half`.
fn spawn_stream_writer<W>(write_half: W, codec: Codec) -> ClientStreamWriter<Value, ()>
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    let (tx, mut rx) = mpsc::channel::<Result<Value, Error>>(32);
    let writer_task = tokio::spawn(async move {
        let mut writer = BufWriter::new(write_half);
        while let Some(item) = rx.recv().await {
            let value = item?;
            codec.write(&mut writer, &value).await?;
        }
        writer.shutdown().await?;
        Ok(())
    });
    ClientStreamWriter::new(tx, writer_task)
}

/// Reads and validates the server's acknowledgement of a stream request.
///
/// The server acknowledges an id-bearing stream request with a JSON-RPC
/// response whose `id` matches and whose `error` is absent, so this validates
/// the handshake before the caller sees any stream values.
async fn read_handshake<R>(reader: &mut R, codec: Codec) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
{
    let Some(response) = codec.read::<_, crate::RpcResponse>(reader).await? else {
        return Err(Error::Protocol("missing stream handshake response"));
    };
    if let Some(error) = response.error {
        return Err(Error::Rpc {
            code: crate::ErrorCode::from(error.code),
            message: error.message,
            data: error.data,
        });
    }
    if response.id.as_ref() != Some(&Value::from(1u64)) {
        return Err(Error::Protocol("unexpected stream handshake id"));
    }
    Ok(())
}
