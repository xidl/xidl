mod session;

use crate::Error;
use crate::transport::{IoListener, Listener};
use serde_json::Value;
use session::ServerSession;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;

    fn accepts_bidi(&self, _method: &str) -> bool {
        false
    }

    async fn handle_bidi(
        &self,
        method: &str,
        _params: Value,
        _stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        Err(Error::method_not_found(method))
    }
}

#[async_trait::async_trait]
impl<T> Handler for Arc<T>
where
    T: Handler,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        (**self).handle(method, params).await
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        (**self).accepts_bidi(method)
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        (**self).handle_bidi(method, params, stream).await
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

struct MultiHandler {
    services: Vec<Box<dyn Handler>>,
}

#[async_trait::async_trait]
impl Handler for MultiHandler {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
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
    }

    fn accepts_bidi(&self, method: &str) -> bool {
        self.services
            .iter()
            .any(|service| service.accepts_bidi(method))
    }

    async fn handle_bidi(
        &self,
        method: &str,
        params: Value,
        stream: crate::stream::ReaderWriter<Value, Value>,
    ) -> Result<(), Error> {
        for service in &self.services {
            if service.accepts_bidi(method) {
                return service.handle_bidi(method, params, stream).await;
            }
        }
        Err(Error::method_not_found(method))
    }
}

pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    endpoint: Option<String>,
    services: Vec<Box<dyn Handler>>,
}

pub struct Server {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
    services: Vec<Box<dyn Handler>>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            listener: None,
            endpoint: None,
            services: Vec::new(),
        }
    }

    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    pub async fn serve(self) -> Result<(), Error> {
        let handler = Arc::new(MultiHandler {
            services: self.services,
        });
        loop {
            let (stream, _peer) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                Err(err) => return Err(err.into()),
            };
            let handler = handler.clone();
            tokio::spawn(async move {
                let mut session = ServerSession::new(stream, handler);
                let _ = session.run().await;
            });
        }
    }
}

impl ServerBuilder {
    pub fn with_listener<L>(mut self, listener: L) -> Self
    where
        L: Listener + 'static,
    {
        self.listener = Some(Box::new(listener));
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub fn with_io<R, W>(self, io: Io<R, W>) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        self.with_listener(IoListener::from_io(io))
    }

    pub async fn build(self) -> Result<Server, Error> {
        if self.listener.is_some() && self.endpoint.is_some() {
            return Err(Error::Protocol("listener already set"));
        }

        let (listener, endpoint) = if let Some(listener) = self.listener {
            let endpoint = listener.endpoint();
            (listener, endpoint)
        } else if let Some(endpoint) = self.endpoint {
            let (listener, endpoint) = crate::transport::bind(&endpoint).await?.into_parts();
            (listener, Some(endpoint))
        } else {
            return Err(Error::Protocol("missing listener"));
        };

        Ok(Server {
            listener,
            endpoint,
            services: self.services,
        })
    }

    pub async fn build_on<S>(self, endpoint: S) -> Result<Server, Error>
    where
        S: AsRef<str>,
    {
        self.with_endpoint(endpoint.as_ref()).build().await
    }

    pub async fn serve(self) -> Result<(), Error> {
        self.build().await?.serve().await
    }

    pub async fn serve_on<S>(self, endpoint: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        self.build_on(endpoint).await?.serve().await
    }
}
