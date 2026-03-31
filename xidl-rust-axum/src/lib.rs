mod error;
pub use error::{Error, ErrorBody, Result};

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::{ApiKeyAuth, Client, ClientApiKeyLocation, ClientAuth, ClientAuthRequirement};

mod server;
pub use server::{Server, Service};

mod request;
pub use request::Request;

pub mod http;
mod serde_factory;
pub use serde_factory::{DeserializeFactory, SerializeFactory};
pub mod stream;

pub mod auth;

pub use axum;
pub use axum_extra;
pub use futures_util;
#[cfg(feature = "client")]
pub use reqwest;
pub use serde;
pub use serde_json;
pub use serde_urlencoded;
pub use tokio;
pub use tokio_tungstenite;
