mod gen {
    include!("../versioned_inheritance.rs");
    include!("../versioned_inheritance.adapters.rs");
}
mod chain;
mod hooks;
mod route;
mod sdk;
mod store;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch},
    Json, Router,
};
use gen::{v1, v2, v3, version_adapters::Version};
use serde_json::json;
use std::sync::Arc;
use store::Store;
use tower::ServiceExt;

#[derive(Clone, Copy)]
enum VersionedMode {
    Chain,
    Route,
}

#[derive(Clone)]
struct Application {
    mode: VersionedMode,
    chain: Router,
    routes: [Router; 3],
    transaction: Arc<tokio::sync::Mutex<()>>,
}

impl Application {
    async fn serve(State(app): State<Self>, request: Request<Body>) -> Response {
        let Some(version) = chain::selected(request.headers()) else {
            return xidl_rust_axum::Error::new(400, "unsupported address version").into_response();
        };
        let tenant = request.uri().path().split('/').nth(2);
        let credential = request
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok());
        let authorized = match credential {
            Some("Bearer test-acme") => Some("acme"),
            Some("Bearer test-other") => Some("other"),
            _ => None,
        };
        let mut response = if tenant.is_none() || tenant != authorized {
            (
                StatusCode::FORBIDDEN,
                Json(json!({"reason":"tenant access denied"})),
            )
                .into_response()
        } else {
            // The example application serializes read/convert/write/project as
            // one transaction, including the snapshot of historical facts.
            let _transaction = app.transaction.lock().await;
            let router = match app.mode {
                VersionedMode::Route => app.routes[match version {
                    Version::V1 => 0,
                    Version::V2 => 1,
                    Version::V3 => 2,
                }]
                .clone(),
                VersionedMode::Chain => app.chain.clone(),
            };
            router.oneshot(request).await.unwrap()
        };
        response.headers_mut().insert(
            "x-address-version",
            HeaderValue::from_static(version.value()),
        );
        response
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("sdk") {
        sdk::run(&args[2]).await;
        return;
    }
    let mode = match args.get(1).map(String::as_str) {
        Some("chain") => VersionedMode::Chain,
        Some("route") => VersionedMode::Route,
        _ => panic!("choose chain or route"),
    };
    let store = Arc::new(Store::seeded());
    let app = Application {
        mode,
        chain: Router::new()
            .route(
                "/tenants/{tenant}/addresses/{id}",
                get(chain::read).merge(patch(chain::update)),
            )
            .with_state(store.clone()),
        transaction: Arc::new(tokio::sync::Mutex::new(())),
        routes: [
            v1::shipping::AddressesServer::new(route::Facade(store.clone())).router(),
            v2::shipping::AddressesServer::new(route::Facade(store.clone())).router(),
            v3::shipping::AddressesServer::new(route::Facade(store.clone())).router(),
        ],
    };
    let audit = store.clone();
    let router = Router::new()
        .route("/ready", get(|| async { StatusCode::NO_CONTENT }))
        .route(
            "/audit",
            get(move || {
                let audit = audit.clone();
                async move { Json(audit.audit()) }
            }),
        )
        .fallback(Application::serve)
        .with_state(app);
    let listener =
        tokio::net::TcpListener::bind(format!("127.0.0.1:{}", std::env::var("PORT").unwrap()))
            .await
            .unwrap();
    axum::serve(listener, router).await.unwrap();
}
