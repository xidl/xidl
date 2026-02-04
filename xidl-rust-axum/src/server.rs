use super::*;

pub struct Io {
    listener: tokio::net::TcpListener,
}

impl Io {
    pub fn new(listener: tokio::net::TcpListener) -> Self {
        Self { listener }
    }
}

pub trait Service: Send + Sync + 'static {
    fn into_router(self) -> axum::Router;
}

pub struct ServerBuilder {
    io: Option<Io>,
    router: axum::Router,
}

pub struct Server;

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder {
            io: None,
            router: axum::Router::new(),
        }
    }
}

impl ServerBuilder {
    pub fn with_io(mut self, io: Io) -> Self {
        self.io = Some(io);
        self
    }

    pub fn with_service<T: Service>(mut self, svc: T) -> Self {
        self.router = self.router.merge(svc.into_router());
        self
    }

    pub async fn serve(self, addr: impl tokio::net::ToSocketAddrs) -> Result<()> {
        let listener = match self.io {
            Some(io) => io.listener,
            None => tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|err| Error::new(500, err.to_string()))?,
        };
        axum::serve(listener, self.router)
            .await
            .map_err(|err| Error::new(500, err.to_string()))
    }
}
