use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use axum;
pub use reqwest;
pub use serde;
pub use serde_json;

#[derive(Debug, Clone, Copy)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ParamSource {
    Path,
    Query,
    Body,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteParam {
    pub name: &'static str,
    pub ty: &'static str,
    pub source: ParamSource,
}

#[derive(Debug, Clone, Copy)]
pub struct RouteInfo {
    pub name: &'static str,
    pub method: HttpMethod,
    pub path: &'static str,
    pub params: &'static [RouteParam],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: i32,
    pub msg: String,
}

#[derive(Debug, Clone, Error)]
#[error("{message}")]
pub struct Error {
    pub code: i32,
    pub message: String,
}

impl Error {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn message(message: impl Into<String>) -> Self {
        Self::new(500, message)
    }
}

impl From<Error> for ErrorBody {
    fn from(err: Error) -> Self {
        Self {
            code: err.code,
            msg: err.message,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let body: ErrorBody = self.into();
        (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Client {
    base_url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn url(&self, path: &str) -> String {
        if self.base_url.ends_with('/') && path.starts_with('/') {
            format!("{}{}", self.base_url.trim_end_matches('/'), path)
        } else if !self.base_url.ends_with('/') && !path.starts_with('/') {
            format!("{}/{}", self.base_url, path)
        } else {
            format!("{}{}", self.base_url, path)
        }
    }
}

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

    pub fn with_service<S>(mut self, svc: S) -> Self
    where
        S: Service,
    {
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
            .map_err(|err| Error::new(500, err.to_string()))?;
        Ok(())
    }
}
