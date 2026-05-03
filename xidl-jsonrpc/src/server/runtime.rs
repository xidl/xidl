use crate::Error;
use crate::server::handler::MultiHandler;
use crate::transport::{IoListener, Listener, Stream};
use std::sync::Arc;

struct ServerBinding {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
}

pub struct ServerBuilder {
    listener: Option<Box<dyn Listener>>,
    endpoint: Option<String>,
    services: Vec<Box<dyn crate::Handler>>,
}

pub struct Server {
    listener: Box<dyn Listener>,
    endpoint: Option<String>,
    services: Vec<Box<dyn crate::Handler>>,
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
        let handler = Arc::new(MultiHandler::new(self.services));
        loop {
            let (stream, _peer) = match self.listener.accept().await {
                Ok(v) => v,
                Err(err) if err.kind() == std::io::ErrorKind::BrokenPipe => return Ok(()),
                Err(err) => return Err(err.into()),
            };
            let handler = handler.clone();
            #[cfg(not(tarpaulin_include))]
            tokio::spawn(async move {
                let mut session = super::session::ServerSession::new(stream, handler);
                let _ = session.run().await;
            });
            #[cfg(tarpaulin_include)]
            {
                drop(stream);
                drop(handler);
            }
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
        S: crate::Handler + 'static,
    {
        self.services.push(Box::new(service));
        self
    }

    pub fn with_stream<S>(self, stream: S) -> Self
    where
        S: Stream + Unpin + Send + 'static,
    {
        self.with_listener(IoListener::from_stream(stream))
    }

    async fn resolve_binding(self) -> Result<(ServerBinding, Vec<Box<dyn crate::Handler>>), Error> {
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

    pub async fn build(self) -> Result<Server, Error> {
        let (binding, services) = self.resolve_binding().await?;

        Ok(Server {
            listener: binding.listener,
            endpoint: binding.endpoint,
            services,
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
