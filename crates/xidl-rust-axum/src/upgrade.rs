//! HTTP connection upgrade helper.
//!
//! Provides a two-stage upgrade lifecycle to validate handshake parameters
//! before upgrading the connection.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use std::future::Future;

/// Helper to handle HTTP connection upgrades.
///
/// Contains the original request parts and target protocol, allowing handshakes
/// to be checked before sending the 101 Switching Protocols response.
pub struct Upgrade {
    request: Request<Body>,
    protocol: String,
}

impl Upgrade {
    /// Creates a new `Upgrade` helper.
    pub fn new(request: Request<Body>, protocol: String) -> Self {
        Self { request, protocol }
    }

    /// Upgrades the connection and calls the provided callback with the upgraded stream.
    ///
    /// This method returns a 101 Switching Protocols response and spawns a background
    /// task to handle the connection upgrade.
    pub fn on_upgrade<F, Fut>(mut self, callback: F) -> Response
    where
        F: FnOnce(hyper_util::rt::TokioIo<hyper::upgrade::Upgraded>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let on_upgrade = match self
            .request
            .extensions_mut()
            .remove::<hyper::upgrade::OnUpgrade>()
        {
            Some(on) => on,
            None => {
                let mut err_response = Response::new(Body::empty());
                *err_response.status_mut() = StatusCode::BAD_REQUEST;
                return err_response;
            }
        };

        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;

        if let Ok(val) = header::HeaderValue::from_str(&self.protocol) {
            response.headers_mut().insert(header::UPGRADE, val);
        }
        response.headers_mut().insert(
            header::CONNECTION,
            header::HeaderValue::from_static("upgrade"),
        );

        tokio::spawn(async move {
            if let Ok(upgraded) = on_upgrade.await {
                callback(hyper_util::rt::TokioIo::new(upgraded)).await;
            }
        });

        response
    }
}
