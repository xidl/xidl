use crate::{gen::version_adapters::*, ledger::Ledger, projection::Hooks};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Method, Request, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) struct Chain;

#[derive(serde::Deserialize)]
struct ListQuery {
    limit: Option<i32>,
}

macro_rules! respond {
    ($result:expr, $outcome:ident) => {
        match $result {
            Ok(ProjectionOutcome::Projected($outcome::V1(Ok(value)))) => {
                Json(value).into_response()
            }
            Ok(ProjectionOutcome::Projected($outcome::V2(Ok(value)))) => {
                Json(value).into_response()
            }
            Ok(ProjectionOutcome::Projected($outcome::V1(Err(error)))) => error.into_response(),
            Ok(ProjectionOutcome::Projected($outcome::V2(Err(error)))) => error.into_response(),
            Ok(ProjectionOutcome::Unsupported { error, .. })
            | Err(AdaptationError::Unsupported(error)) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(Hooks::rejected(error)),
            )
                .into_response(),
            Err(AdaptationError::Validation(reason)) => {
                xidl_rust_axum::Error::new(400, reason).into_response()
            }
        }
    };
}

impl Chain {
    pub(crate) fn router(ledger: Arc<Ledger>) -> Router {
        Router::new().fallback(Self::handle).with_state(ledger)
    }

    async fn handle(
        State(ledger): State<Arc<Ledger>>,
        Extension(version): Extension<Version>,
        request: Request<Body>,
    ) -> Response {
        match Self::execute(ledger, version, request).await {
            Ok(response) => response,
            Err(error) => error.into_response(),
        }
    }

    async fn execute(
        ledger: Arc<Ledger>,
        version: Version,
        request: Request<Body>,
    ) -> xidl_rust_axum::Result<Response> {
        let mut wire = json!({});
        for value in request.headers().get_all("cookie") {
            let value = value
                .to_str()
                .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
            for cookie in xidl_rust_axum::cookie::Cookie::split_parse_encoded(value) {
                let cookie =
                    cookie.map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                if cookie.name() == "tenant" && wire.get("tenant").is_none() {
                    wire["tenant"] = json!(cookie.value());
                }
            }
        }
        let response = match (request.method(), request.uri().path()) {
            (&Method::POST, "/payments") => {
                if let Some(key) = request.headers().get("Idempotency-Key") {
                    wire["key"] = json!(key
                        .to_str()
                        .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?);
                }
                if !xidl_rust_axum::http::content_type_matches(
                    request.headers(),
                    "application/json",
                ) {
                    return Err(xidl_rust_axum::Error::unsupported_media_type());
                }
                let bytes = to_bytes(request.into_body(), 65536)
                    .await
                    .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                wire["capture"] = serde_json::from_slice::<Value>(&bytes)
                    .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                respond!(
                    dispatch_invoices_pay(
                        version,
                        wire,
                        |input, _| async move { ledger.pay(input) },
                        &Hooks
                    )
                    .await,
                    InvoicesPayOutcome
                )
            }
            (&Method::GET, "/invoices") => {
                let query = axum::extract::Query::<ListQuery>::try_from_uri(request.uri())
                    .map_err(|error| xidl_rust_axum::Error::new(400, error.to_string()))?;
                if let Some(limit) = query.limit {
                    wire["limit"] = json!(limit);
                }
                respond!(
                    dispatch_invoices_list(
                        version,
                        wire,
                        |input, _| async move { ledger.list(input) },
                        &Hooks
                    )
                    .await,
                    InvoicesListOutcome
                )
            }
            (&Method::GET, "/audit") => {
                respond!(
                    dispatch_invoices_audit(
                        version,
                        wire,
                        |input, _| async move { ledger.audit(input) },
                        &Hooks
                    )
                    .await,
                    InvoicesAuditOutcome
                )
            }
            (&Method::GET, path) if path.starts_with("/invoices/") && !path[10..].is_empty() => {
                wire["id"] = json!(&path[10..]);
                respond!(
                    dispatch_invoices_get(
                        version,
                        wire,
                        |input, _| async move { ledger.get(input) },
                        &Hooks
                    )
                    .await,
                    InvoicesGetOutcome
                )
            }
            _ => StatusCode::NOT_FOUND.into_response(),
        };
        Ok(response)
    }
}
