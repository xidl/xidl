use crate::{BoxFuture, Error, ErrorCode, RpcError, RpcRequestOwned, RpcResponse, JSONRPC_VERSION};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
#[cfg(feature = "tokio-net")]
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;

pub trait Handler: Send + Sync {
    fn handle<'a>(&'a self, method: &'a str, params: Value) -> BoxFuture<'a, Result<Value, Error>>;
}

impl<T> Handler for Arc<T>
where
    T: Handler,
{
    fn handle<'a>(&'a self, method: &'a str, params: Value) -> BoxFuture<'a, Result<Value, Error>> {
        (**self).handle(method, params)
    }
}

pub struct Io<R, W> {
    pub reader: R,
    pub writer: W,
}

impl<R, W> Io<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self { reader, writer }
    }
}

type BoxedReader = Box<dyn AsyncBufRead + Unpin + Send>;
type BoxedWriter = Box<dyn AsyncWrite + Unpin + Send>;
type BoxedIo = Io<BoxedReader, BoxedWriter>;

pub trait Listener: Send {
    fn accept<'a>(&'a mut self) -> BoxFuture<'a, Result<Option<BoxedIo>, Error>>;
}

pub struct MuxSender {
    tx: mpsc::UnboundedSender<BoxedIo>,
}

impl MuxSender {
    pub fn push<R, W>(&self, io: Io<R, W>) -> Result<(), Error>
    where
        R: AsyncBufRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        self.tx
            .send(box_io(io))
            .map_err(|_| Error::Protocol("listener channel closed"))
    }
}

pub struct MuxListener {
    rx: mpsc::UnboundedReceiver<BoxedIo>,
}

impl MuxListener {
    pub fn channel() -> (MuxSender, Self) {
        let (tx, rx) = mpsc::unbounded_channel();
        (MuxSender { tx }, Self { rx })
    }

    pub fn from_io<R, W>(io: Io<R, W>) -> Self
    where
        R: AsyncBufRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel();
        let _ = tx.send(box_io(io));
        Self { rx }
    }
}

impl Listener for MuxListener {
    fn accept<'a>(&'a mut self) -> BoxFuture<'a, Result<Option<BoxedIo>, Error>> {
        Box::pin(async move { Ok(self.rx.recv().await) })
    }
}

#[cfg(feature = "tokio-net")]
impl Listener for TcpListener {
    fn accept<'a>(&'a mut self) -> BoxFuture<'a, Result<Option<BoxedIo>, Error>> {
        Box::pin(async move {
            let (stream, _peer) = TcpListener::accept(self).await?;
            stream.set_nodelay(true)?;
            let (rx, tx) = tokio::io::split(stream);
            let reader = BufReader::new(rx);
            Ok(Some(box_io(Io::new(reader, tx))))
        })
    }
}

struct MultiHandler {
    services: Vec<Box<dyn Handler>>,
}

impl Handler for MultiHandler {
    fn handle<'a>(&'a self, method: &'a str, params: Value) -> BoxFuture<'a, Result<Value, Error>> {
        Box::pin(async move {
            for service in &self.services {
                match service.handle(method, params.clone()).await {
                    Ok(value) => return Ok(value),
                    Err(err) => {
                        if err.is_method_not_found() {
                            continue;
                        }
                        return Err(err);
                    }
                }
            }
            Err(Error::method_not_found(method))
        })
    }
}

pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    services: Vec<Box<dyn Handler>>,
}

pub struct Server {
    listener: Box<dyn Listener>,
    services: Vec<Box<dyn Handler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            listener: None,
            services: Vec::new(),
        }
    }

    pub async fn serve(mut self) -> Result<(), Error> {
        let handler = Arc::new(MultiHandler {
            services: self.services,
        });
        loop {
            let io = match self.listener.accept().await? {
                Some(io) => io,
                None => break,
            };
            let mut session = ServerSession::new(io.reader, io.writer, handler.clone());
            session.run().await?;
        }
        Ok(())
    }
}

impl ServerBuilder {
    pub fn with_io<R, W>(mut self, io: Io<R, W>) -> Self
    where
        R: AsyncBufRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        self.listener = Some(Box::new(MuxListener::from_io(io)));
        self
    }

    pub fn with_listener<L>(mut self, listener: L) -> Self
    where
        L: Listener + 'static,
    {
        self.listener = Some(Box::new(listener));
        self
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub async fn serve(self) -> Result<(), Error> {
        let listener = self.listener.ok_or(Error::Protocol("missing listener"))?;
        let server = Server {
            listener,
            services: self.services,
        };
        server.serve().await
    }

    #[cfg(feature = "tokio-net")]
    pub async fn serve_on<A>(self, addr: A) -> Result<(), Error>
    where
        A: ToSocketAddrs,
    {
        if self.listener.is_some() {
            return Err(Error::Protocol("listener already set"));
        }
        let listener = TcpListener::bind(addr).await?;
        self.with_listener(listener).serve().await
    }
}

pub async fn serve<R, W, H>(mut reader: R, mut writer: W, handler: H) -> Result<(), Error>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
    H: Handler,
{
    let mut session = ServerSession::new(&mut reader, &mut writer, handler);
    session.run().await
}

fn box_io<R, W>(io: Io<R, W>) -> BoxedIo
where
    R: AsyncBufRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    Io::new(Box::new(io.reader), Box::new(io.writer))
}

struct ServerSession<R, W, H> {
    reader: R,
    writer: W,
    handler: H,
}

impl<R, W, H> ServerSession<R, W, H>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
    H: Handler,
{
    fn new(reader: R, writer: W, handler: H) -> Self {
        Self {
            reader,
            writer,
            handler,
        }
    }

    async fn run(&mut self) -> Result<(), Error> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self.reader.read_line(&mut line).await?;
            if bytes == 0 {
                break;
            }
            self.handle_line(&line).await?;
        }
        Ok(())
    }

    async fn handle_line(&mut self, line: &str) -> Result<(), Error> {
        let request: RpcRequestOwned = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(err) => {
                self.write_error(None, Error::Json(err)).await?;
                return Ok(());
            }
        };
        let id = request.id;
        let method = match request.method {
            Some(method) => method,
            None => {
                self.write_error(id, Error::Protocol("missing method"))
                    .await?;
                return Ok(());
            }
        };
        let params = request.params.unwrap_or(Value::Null);

        match self.handler.handle(&method, params).await {
            Ok(value) => self.write_result(id, value).await,
            Err(err) => self.write_error(id, err).await,
        }
    }

    async fn write_result(&mut self, id: Option<u64>, result: Value) -> Result<(), Error> {
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: Some(result),
            error: None,
        };
        self.write_response(response).await
    }

    async fn write_error(&mut self, id: Option<u64>, error: Error) -> Result<(), Error> {
        let rpc_error = match error {
            Error::Rpc {
                code,
                message,
                data,
            } => RpcError {
                code: code.code(),
                message,
                data,
            },
            Error::Json(err) => RpcError {
                code: ErrorCode::ParseError.code(),
                message: err.to_string(),
                data: None,
            },
            Error::Protocol(message) => RpcError {
                code: ErrorCode::InvalidRequest.code(),
                message: message.to_string(),
                data: None,
            },
            Error::Io(err) => RpcError {
                code: ErrorCode::InternalError.code(),
                message: err.to_string(),
                data: None,
            },
        };
        let response = RpcResponse {
            jsonrpc: Some(JSONRPC_VERSION.to_string()),
            id,
            result: None,
            error: Some(rpc_error),
        };
        self.write_response(response).await
    }

    async fn write_response(&mut self, response: RpcResponse) -> Result<(), Error> {
        let payload = serde_json::to_string(&response)?;
        self.writer.write_all(payload.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
}
