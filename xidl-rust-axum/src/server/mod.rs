use super::*;

/// Trait implemented by generated Axum services.
///
/// A service knows how to convert itself into an [`axum::Router`] that can be
/// merged into a larger application.
pub trait Service: Send + Sync + 'static {
    /// Consumes the service and produces its router.
    fn into_router(self) -> axum::Router;
}

/// Builder for composing one or more generated services into an Axum server.
pub struct ServerBuilder {
    router: axum::Router,
}

/// Entry point for serving generated Axum services.
pub struct Server;

impl Server {
    /// Creates a new server builder.
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            router: axum::Router::new(),
        }
    }
}

impl ServerBuilder {
    /// Merges a generated service into the server router.
    pub fn with_service<T: Service>(mut self, svc: T) -> Self {
        self.router = self.router.merge(svc.into_router());
        self
    }

    /// Binds a TCP listener on `addr` and serves the composed router.
    #[cfg(not(tarpaulin_include))]
    pub async fn serve(self, addr: impl tokio::net::ToSocketAddrs) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|err| Error::new(500, err.to_string()))?;
        axum::serve(listener, self.router)
            .await
            .map_err(|err| Error::new(500, err.to_string()))
    }

    /// Serves the composed router using an existing Axum listener.
    #[cfg(not(tarpaulin_include))]
    pub async fn serve_with_listener<L>(self, listener: L) -> Result<()>
    where
        L: axum::serve::Listener,
        axum::serve::Serve<L, axum::Router, axum::Router>:
            std::future::IntoFuture<Output = std::io::Result<()>>,
    {
        axum::serve(listener, self.router)
            .await
            .map_err(|err| Error::new(500, err.to_string()))
    }
}

#[cfg(test)]
mod tests;
