mod session;

use crate::{Error, Listener};
use serde_json::Value;
use session::ServerSession;
use std::sync::Arc;
#[cfg(feature = "tokio-net")]
use tokio::net::{TcpListener, ToSocketAddrs};

#[async_trait::async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error>;
}

#[async_trait::async_trait]
impl<T> Handler for Arc<T>
where
    T: Handler,
{
    async fn handle(&self, method: &str, params: Value) -> Result<Value, Error> {
        (**self).handle(method, params).await
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

            let mut session = ServerSession::new(stream, handler.clone());
            session.run().await?;
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
