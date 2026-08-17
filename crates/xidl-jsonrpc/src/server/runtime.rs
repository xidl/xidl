use crate::Error;
use crate::codec::Codec;
use crate::server::handler::MultiHandler;
use crate::transport::{IoListener, Listener, Stream};
use std::sync::Arc;
use tokio::sync::Semaphore;

struct ServerBinding {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
}

const DEFAULT_MAX_IN_FLIGHT: usize = 256;

/// Builder for composing one or more JSON-RPC handlers into a server.
pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    endpoint: Option<String>,
    services: Vec<Arc<dyn crate::Handler>>,
    codec: Codec,
    max_in_flight: usize,
}

/// JSON-RPC server bound to a transport listener.
pub struct Server {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
    services: Vec<Arc<dyn crate::Handler>>,
    codec: Codec,
    max_in_flight: usize,
}

impl Server {
    /// Creates a new server builder.
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            listener: None,
            endpoint: None,
            services: Vec::new(),
            codec: Codec::Json,
            max_in_flight: DEFAULT_MAX_IN_FLIGHT,
        }
    }

    /// Returns the bound endpoint, when available.
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    /// Serves incoming connections until the listener stops accepting streams.
    pub async fn serve(self) -> Result<(), Error> {
        let handler = Arc::new(MultiHandler::new(self.services));
        let global_in_flight = Arc::new(Semaphore::new(self.max_in_flight));
        self.listener.set_frame_kind(self.codec.frame_kind());
        loop {
            let (stream, _peer) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                Err(err) => return Err(err.into()),
            };
            let handler = handler.clone();
            let global_in_flight = global_in_flight.clone();
            tokio::spawn(async move {
                let mut session = super::session::ServerSession::with_limits(
                    stream,
                    handler,
                    self.codec,
                    global_in_flight,
                );
                if let Err(error) = session.run().await {
                    eprintln!("xidl-jsonrpc session failed: {error}");
                }
            });
        }
    }
}

impl ServerBuilder {
    /// Uses an existing listener for the server.
    pub fn with_listener<L>(mut self, listener: L) -> Self
    where
        L: Listener + 'static,
    {
        self.listener = Some(Box::new(listener));
        self
    }

    /// Binds the server to an endpoint string handled by `xidl_jsonrpc`.
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Adds a handler service to the server.
    pub fn with_service<S>(mut self, service: S) -> Self
    where
        S: crate::Handler + 'static,
    {
        self.services.push(Arc::new(service));
        self
    }

    /// Sets the maximum number of in-flight requests across all connections.
    pub fn with_max_in_flight(mut self, max_in_flight: usize) -> Self {
        self.max_in_flight = max_in_flight.max(1);
        self
    }

    /// Serves requests over an existing stream by wrapping it in a listener.
    pub fn with_stream<S>(self, stream: S) -> Self
    where
        S: Stream + Unpin + Send + 'static,
    {
        self.with_listener(IoListener::from_stream(stream))
    }

    /// Serves requests using length-prefixed MessagePack framing.
    #[cfg(feature = "msgpack")]
    pub fn with_msgpack(mut self) -> Self {
        self.codec = Codec::Msgpack;
        self
    }

    async fn resolve_binding(self) -> Result<(ServerBinding, Vec<Arc<dyn crate::Handler>>), Error> {
        if self.listener.is_some() && self.endpoint.is_some() {
            return Err(Error::Protocol("listener already set"));
        }

        let binding = if let Some(listener) = self.listener {
            ServerBinding {
                endpoint: listener.endpoint(),
                listener,
            }
        } else if let Some(endpoint) = self.endpoint {
            let (listener, endpoint) = crate::transport::bind(&endpoint).await?.into_parts();
            ServerBinding {
                listener,
                endpoint: Some(endpoint),
            }
        } else {
            return Err(Error::Protocol("missing listener"));
        };

        Ok((binding, self.services))
    }

    /// Builds a server from the configured listener or endpoint.
    pub async fn build(self) -> Result<Server, Error> {
        let codec = self.codec;
        let max_in_flight = self.max_in_flight;
        let (binding, services) = self.resolve_binding().await?;

        Ok(Server {
            listener: binding.listener,
            endpoint: binding.endpoint,
            services,
            codec,
            max_in_flight,
        })
    }

    /// Binds and builds a server on the given endpoint.
    pub async fn build_on<S>(self, endpoint: S) -> Result<Server, Error>
    where
        S: AsRef<str>,
    {
        self.with_endpoint(endpoint.as_ref()).build().await
    }

    /// Builds and immediately serves the configured server.
    pub async fn serve(self) -> Result<(), Error> {
        self.build().await?.serve().await
    }

    /// Binds, builds, and immediately serves the server on the endpoint.
    pub async fn serve_on<S>(self, endpoint: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        self.build_on(endpoint).await?.serve().await
    }
}
