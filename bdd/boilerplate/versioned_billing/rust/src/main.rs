use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
    Router,
};
use std::sync::Arc;
use tower::ServiceExt;

pub mod gen {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/versioned_billing.rs"));
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/versioned_billing.adapters.rs"
    ));
}
mod chain;
mod ledger;
mod projection;
mod route;
pub mod sdk;

use gen::version_adapters::Version;
use ledger::Ledger;

#[derive(Clone, Copy)]
enum VersionedMode {
    Chain,
    Route,
}

impl VersionedMode {
    fn router(self) -> Router {
        let ledger = Arc::new(Ledger::new());
        let first = gen::v1::InvoicesServer::new(route::VersionRoute(ledger.clone())).router();
        let current = gen::v2::InvoicesServer::new(route::VersionRoute(ledger.clone())).router();
        let chain = chain::Chain::router(ledger);
        Router::new()
            .fallback(move |mut request: Request<Body>| {
                let version = match request.headers().get("X-Billing-Version") {
                    None => Ok(Version::V2),
                    Some(value) if value == "2025-01-01" => Ok(Version::V1),
                    Some(value) if value == "2025-04-01" => Ok(Version::V2),
                    Some(_) => Err(xidl_rust_axum::Error::new(
                        400,
                        "unsupported billing version",
                    )),
                };
                let router = match (&self, &version) {
                    (Self::Chain, _) => chain.clone(),
                    (Self::Route, Ok(Version::V1)) => first.clone(),
                    (Self::Route, _) => current.clone(),
                };
                async move {
                    let version = match version {
                        Ok(version) => version,
                        Err(error) => return error.into_response(),
                    };
                    request.extensions_mut().insert(version);
                    let mut response = router
                        .oneshot(request)
                        .await
                        .expect("infallible Axum router");
                    response.headers_mut().insert(
                        "X-Billing-Version",
                        version.value().parse().expect("static version header"),
                    );
                    response
                }
            })
            .route(
                "/ready",
                axum::routing::get(|| async { StatusCode::NO_CONTENT }),
            )
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("expected chain, route or sdk")?;
    if mode == "sdk" {
        sdk::exercise(args.next().ok_or("expected SDK base URL")?).await?;
        return Ok(());
    }
    let mode = match mode.as_str() {
        "chain" => VersionedMode::Chain,
        "route" => VersionedMode::Route,
        _ => return Err("unknown billing mode".into()),
    };
    let listener =
        tokio::net::TcpListener::bind(format!("127.0.0.1:{}", std::env::var("PORT")?)).await?;
    axum::serve(listener, mode.router()).await?;
    Ok(())
}
