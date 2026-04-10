use super::*;
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

struct EchoService(&'static str);

impl Service for EchoService {
    fn into_router(self) -> Router {
        Router::new()
            .route(
                "/value",
                get(|State(value): State<&'static str>| async move { Json(value) }),
            )
            .with_state(self.0)
    }
}

#[tokio::test]
async fn builder_accepts_service_implementations() {
    let builder = Server::builder().with_service(EchoService("merged"));
    std::mem::drop(builder);
}

#[tokio::test]
#[cfg(not(tarpaulin_include))]
async fn serve_returns_bind_errors() {
    let err = Server::builder()
        .serve("256.256.256.256:1")
        .await
        .unwrap_err();
    assert_eq!(err.code, 500);
}
