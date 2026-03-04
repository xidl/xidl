use super::*;

pub trait Service: Send + Sync + 'static {
    fn into_router(self) -> axum::Router;
}

pub struct ServerBuilder {
    router: axum::Router,
}

pub struct Server;

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            router: axum::Router::new(),
        }
    }
}

impl ServerBuilder {
    pub fn with_service<T: Service>(mut self, svc: T) -> Self {
        self.router = self.router.merge(svc.into_router());
        self
    }

    pub async fn serve(self, addr: impl tokio::net::ToSocketAddrs) -> Result<()> {
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|err| Error::new(500, err.to_string()))?;
        axum::serve(listener, self.router)
            .await
            .map_err(|err| Error::new(500, err.to_string()))
    }

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
